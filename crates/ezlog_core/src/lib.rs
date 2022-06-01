#![feature(core_ffi_c)]

mod appender;
mod compress;
mod config;
mod crypto;
mod errors;
mod events;

#[cfg(target_os="android")]
#[allow(non_snake_case)]
mod android;

pub use self::config::EZLogConfig;
pub use self::config::EZLogConfigBuilder;

use appender::EZMmapAppender;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use compress::ZlibCodec;
use crossbeam_channel::{Receiver, Sender, TrySendError};
use crypto::{Aes128Gcm, Aes256Gcm};
use errors::{CryptoError, LogError, ParseError};
use log::Record;
use memmap2::MmapMut;
use std::ffi::c_char;
use std::ffi::c_uchar;
use std::ffi::c_uint;
use std::ffi::CStr;
use std::slice;
use std::{
    cmp,
    collections::{hash_map::DefaultHasher, HashMap},
    fmt,
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    io::{self, Cursor, Read, Write},
    mem::MaybeUninit,
    path::Path,
    ptr,
    rc::Rc,
    sync::Once,
    thread,
};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

pub const DEFAULT_LOG_NAME: &str = "default";
pub(crate) const FILE_SIGNATURE: &[u8; 2] = b"ez";

pub(crate) const DEFAULT_LOG_FILE_SUFFIX: &str = "mmap";
static LOG_LEVEL_NAMES: [&str; 6] = ["OFF", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];
pub(crate) const UNKNOWN: &str = "UNKNOWN";

pub(crate) const RECORD_SIGNATURE_START: u8 = 0x3b;
pub(crate) const RECORD_SIGNATURE_END: u8 = 0x21;

pub(crate) const DEFAULT_MAX_LOG_SIZE: u64 = 150 * 1024;
pub const V1_LOG_HEADER_SIZE: usize = 10;

static mut CHANNEL: MaybeUninit<(Sender<EZMsg>, Receiver<EZMsg>)> = MaybeUninit::uninit();
static CHANNEL_INIT: Once = Once::new();

static mut LOG_MAP: MaybeUninit<HashMap<u64, EZLogger>> = MaybeUninit::uninit();
static LOG_MAP_INIT: Once = Once::new();

static ONE_RECEIVER: Once = Once::new();

#[no_mangle]
pub extern "C" fn c_create_log(
    c_log_name: *const c_char,
    c_level: c_uchar,
    c_dir_path: *const c_char,
    c_keep_days: c_uint,
    c_compress: c_uchar,
    c_compress_level: c_uchar,
    c_cipher: c_uchar,
    c_cipher_key: *const c_uchar,
    c_key_len: usize,
    c_cipher_nonce: *const c_uchar,
    c_nonce_len: usize,
) {
    let log_name = unsafe { CStr::from_ptr(c_log_name).to_string_lossy().into_owned() };
    let level = Level::from_usize(c_level as usize).unwrap_or_else(|| Level::Trace);
    let dir_path = unsafe { CStr::from_ptr(c_dir_path).to_string_lossy().into_owned() };
    let duration = Duration::days(c_keep_days as i64);
    let compress = CompressKind::from(c_compress);
    let compress_level = CompressLevel::from(c_compress_level);
    let cipher = CipherKind::from(c_cipher);
    let key_bytes = unsafe { slice::from_raw_parts(c_cipher_key, c_key_len) };
    let cipher_key: Vec<u8> = Vec::from(key_bytes);
    let nonce_bytes = unsafe { slice::from_raw_parts(c_cipher_nonce, c_nonce_len) };
    let cipher_nonce: Vec<u8> = Vec::from(nonce_bytes);

    let config = EZLogConfigBuilder::new()
        .name(log_name)
        .dir_path(dir_path)
        .level(level)
        .duration(duration)
        .compress(compress)
        .compress_level(compress_level)
        .cipher(cipher)
        .cipher_key(cipher_key)
        .cipher_nonce(cipher_nonce)
        .build();

    create_log(config);
}

#[no_mangle]
pub extern "C" fn c_log(
    c_log_name: *const c_char,
    c_level: c_uchar,
    c_target: *const c_char,
    c_content: *const c_char,
) {
    let log_name = unsafe { CStr::from_ptr(c_log_name).to_string_lossy().into_owned() };
    let level = Level::from_usize(c_level as usize).unwrap_or_else(|| Level::Trace);
    let target = unsafe { CStr::from_ptr(c_target).to_string_lossy().into_owned() };
    let content = unsafe { CStr::from_ptr(c_content).to_string_lossy().into_owned() };
    let record = EZRecordBuilder::new()
        .log_name(log_name)
        .level(level)
        .target(target)
        .content(content)
        .build();
    log(record)
}

#[inline]
fn get_channel() -> &'static (Sender<EZMsg>, Receiver<EZMsg>) {
    CHANNEL_INIT.call_once(|| unsafe {
        ptr::write(CHANNEL.as_mut_ptr(), crossbeam_channel::unbounded());
        event!(init);
    });

    unsafe { &*CHANNEL.as_ptr() }
}

#[inline]
fn get_map() -> &'static mut HashMap<u64, EZLogger> {
    LOG_MAP_INIT.call_once(|| unsafe {
        ptr::write(LOG_MAP.as_mut_ptr(), HashMap::new());
        event!(map_create);
    });
    unsafe { &mut (*LOG_MAP.as_mut_ptr()) }
}

#[inline]
fn get_sender() -> Sender<EZMsg> {
    get_channel().0.clone()
}

/// 初始化
pub fn init() {
    init_receiver();
}

pub(crate) fn init_receiver() {
    ONE_RECEIVER.call_once(|| {
        thread::spawn(|| loop {
            match get_channel().1.recv() {
                Ok(msg) => match msg {
                    EZMsg::CreateLogger(config) => {
                        let name = config.name.clone();
                        match EZLogger::new(config) {
                            Ok(log) => {
                                let log_id = log.config.log_id();
                                let map = get_map();
                                map.insert(log_id, log);
                                event!(log_create name);
                            }
                            Err(e) => {
                                event!(log_create_fail name, e);
                            }
                        };
                    }
                    EZMsg::Record(record) => {
                        let log = match get_map().get_mut(&record.log_id()) {
                            Some(l) => l,
                            None => {
                                event!(logger_not_match record.log_id());
                                continue;
                            }
                        };
                        if log.config.level < record.level {
                            event!(
                                record_filter_out & record.id(),
                                &record.level,
                                &log.config.level
                            );
                            continue;
                        }
                        match log.append(&record) {
                            Ok(_) => {
                                event!(record_complete record.id());
                            }
                            Err(err) => match err {
                                LogError::Encoding(_) => todo!(),
                                LogError::IoError(_) => todo!(),
                                LogError::Parse(_) => todo!(),
                                LogError::Crypto(c) => {
                                    event!(encrypt_fail record.id(), c)
                                }
                            },
                        }
                    }
                    EZMsg::ForceFlush(name) => {
                        let id = log_id(&name);
                        let log = match get_map().get_mut(&id) {
                            Some(l) => l,
                            None => {
                                event!(logger_not_match name);
                                continue;
                            }
                        };
                        log.appender.flush().ok();
                        event!(logger_force_flush name);
                    }
                    EZMsg::FlushAll() => get_map().values_mut().for_each(|item| {
                        item.flush().ok();
                    }),
                },
                Err(err) => {
                    event!(channel_recv_err err);
                }
            }
        });
    });
}

pub fn create_log(config: EZLogConfig) {
    let name = config.name.clone();
    let msg = EZMsg::CreateLogger(config);
    if post_msg(msg).is_some() {
        event!(channel_send_log_create name);
    }
}

pub fn log(record: EZRecord) {
    let id = &record.id();
    let msg = EZMsg::Record(record);
    if post_msg(msg).is_some() {
        event!(channel_send_record id)
    }
}

pub fn flush(log_name: &str) {
    let msg = EZMsg::ForceFlush(log_name.to_string());
    if post_msg(msg).is_some() {
        event!(channel_send_flush log_name)
    }
}

pub fn flush_all() {
    let msg = EZMsg::FlushAll();
    if post_msg(msg).is_some() {
        event!(channel_send_flush_all)
    }
}

pub fn post_msg(msg: EZMsg) -> Option<()> {
    match get_sender().try_send(msg).map_err(on_channel_send_err) {
        Ok(_) => Some(()),
        Err(_) => None,
    }
}

fn on_channel_send_err<T>(err: TrySendError<T>) {
    event!(channel_send_err err);
}

pub fn log_id(name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone)]
pub enum EZMsg {
    CreateLogger(EZLogConfig),
    Record(EZRecord),
    ForceFlush(String),
    FlushAll(),
}

pub struct EZLogger {
    config: Rc<EZLogConfig>,
    appender: EZMmapAppender,
    compression: Option<Box<dyn Compress>>,
    cryptor: Option<Box<dyn Cryptor>>,
}

impl EZLogger {
    pub fn new(config: EZLogConfig) -> Result<Self, LogError> {
        let rc_conf = Rc::new(config);
        let appender = EZMmapAppender::new(Rc::clone(&rc_conf))?;
        let compression = EZLogger::create_compress(&rc_conf)?;
        let cryptor = EZLogger::create_cryptor(&rc_conf)?;

        Ok(Self {
            config: Rc::clone(&rc_conf),
            appender,
            compression,
            cryptor,
        })
    }

    pub fn create_cryptor(config: &EZLogConfig) -> Result<Option<Box<dyn Cryptor>>, CryptoError> {
        if let Some(key) = &config.cipher_key {
            if let Some(nonce) = &config.cipher_nonce {
                match config.cipher {
                    CipherKind::AES128GCM => {
                        let encryptor = Aes128Gcm::new(key, nonce)?;
                        Ok(Some(Box::new(encryptor)))
                    }
                    CipherKind::AES256GCM => {
                        let encryptor = Aes256Gcm::new(key, nonce)?;
                        Ok(Some(Box::new(encryptor)))
                    }
                    CipherKind::NONE => Ok(None),
                    CipherKind::UNKNOWN => Ok(None),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn create_decryptor(
        config: &EZLogConfig,
    ) -> Result<Option<Box<dyn Decryptor>>, CryptoError> {
        if let Some(key) = &config.cipher_key {
            if let Some(nonce) = &config.cipher_nonce {
                match config.cipher {
                    CipherKind::AES128GCM => {
                        let encryptor = Aes128Gcm::new(key, nonce)?;
                        Ok(Some(Box::new(encryptor)))
                    }
                    CipherKind::AES256GCM => {
                        let encryptor = Aes256Gcm::new(key, nonce)?;
                        Ok(Some(Box::new(encryptor)))
                    }
                    CipherKind::NONE => Ok(None),
                    CipherKind::UNKNOWN => Ok(None),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn create_compress(config: &EZLogConfig) -> Result<Option<Box<dyn Compress>>, LogError> {
        match config.compress {
            CompressKind::ZLIB => Ok(Some(Box::new(ZlibCodec::new(&config.compress_level)))),
            CompressKind::NONE => Ok(None),
            CompressKind::UNKNOWN => Ok(None),
        }
    }

    pub fn create_decompression(
        config: &EZLogConfig,
    ) -> Result<Option<Box<dyn Decompression>>, LogError> {
        match config.compress {
            CompressKind::ZLIB => Ok(Some(Box::new(ZlibCodec::new(&config.compress_level)))),
            CompressKind::NONE => Ok(None),
            CompressKind::UNKNOWN => Ok(None),
        }
    }

    fn append(&mut self, record: &EZRecord) -> Result<(), LogError> {
        let buf = self.encode_as_block(record)?;
        self.appender.write_all(&buf)?;
        Ok(())
    }

    fn encode(&mut self, record: &EZRecord) -> Result<Vec<u8>, LogError> {
        let mut buf = self.format(record);
        if let Some(encryptor) = &self.cryptor {
            buf = encryptor.encrypt(&buf)?;
        }
        if let Some(compression) = &self.compression {
            buf = compression.compress(&buf)?;
        }
        Ok(buf)
    }

    ///
    pub fn encode_as_block(&mut self, record: &EZRecord) -> Result<Vec<u8>, LogError> {
        let mut chunk: Vec<u8> = Vec::new();
        chunk.append(&mut vec![RECORD_SIGNATURE_START]);
        let mut buf = self.encode(record)?;
        let size = buf.len();
        let mut size_chunk = EZLogger::create_size_chunk(size)?;
        chunk.append(&mut size_chunk);
        chunk.append(&mut buf);
        chunk.append(&mut vec![RECORD_SIGNATURE_END]);
        Ok(chunk)
    }

    fn create_size_chunk(size: usize) -> Result<Vec<u8>, LogError> {
        let mut chunk: Vec<u8> = Vec::new();
        match size {
            // u8::MAX
            size if size < (u8::MAX as usize) => {
                chunk.write_u8(1)?;
                chunk.write_u8(size as u8)?;
            }
            size if size >= (u8::MAX as usize) && size < (u32::MAX as usize) => {
                chunk.write_u8(2)?;
                chunk.write_u16::<BigEndian>(size as u16)?;
            }
            size if size >= (u32::MAX as usize) => {
                chunk.write_u8(4)?;
                chunk.write_u32::<BigEndian>(size as u32)?;
            }
            _ => {}
        };
        Ok(chunk)
    }

    pub fn decode_from_read(&mut self, reader: &mut dyn Read) -> Result<Vec<u8>, LogError> {
        let start_sign = reader.read_u8()?;
        if RECORD_SIGNATURE_START != start_sign {
            return Err(LogError::Parse(ParseError::new(
                "record start sign error".to_string(),
            )));
        }
        let size_of_size = reader.read_u8()?;
        let content_size: usize = match size_of_size {
            1 => reader.read_u8()? as usize,
            2 => reader.read_u16::<BigEndian>()? as usize,
            _ => reader.read_u32::<BigEndian>()? as usize,
        };
        let mut chunk = vec![0u8; content_size];
        reader.read_exact(&mut chunk)?;
        let end_sign = reader.read_u8()?;
        if RECORD_SIGNATURE_END != end_sign {
            return Err(LogError::Parse(ParseError::new(
                "record end sign error".to_string(),
            )));
        }
        self.decode(&chunk)
    }

    pub fn decode(&mut self, chunk: &[u8]) -> Result<Vec<u8>, LogError> {
        let mut buf = chunk.to_vec();

        if let Some(decompression) = &self.compression {
            buf = decompression.decompress(&buf)?;
        }

        if let Some(decryptor) = &self.cryptor {
            buf = decryptor.decrypt(&buf)?;
        }
        Ok(buf)
    }

    fn format(&self, record: &EZRecord) -> Vec<u8> {
        let time = record
            .time
            .format(&Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());
        format!(
            "\n{} {} {} [{}:{}] {}",
            time,
            record.level(),
            record.target(),
            record.thread_name(),
            record.thread_id(),
            record.content()
        )
        .into_bytes()
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        self.appender.flush()
    }
}

/// 日志文件编码器
///
/// ## 根据不同的安全要求实现
/// - 明文实现
/// - 对称加密实现
///
pub struct Encoder {}

/// 压缩
pub trait Compression {
    fn compress(&self, data: &[u8]) -> std::io::Result<Vec<u8>>;
}

pub trait Decompression {
    fn decompress(&self, data: &[u8]) -> std::io::Result<Vec<u8>>;
}

pub trait Compress: Compression + Decompression {}

impl<T: Compression + Decompression> Compress for T {}

/// 加密
pub trait Encryptor {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError>;
}

pub trait Decryptor {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError>;
}

impl<T: Encryptor + Decryptor> Cryptor for T {}

pub trait Cryptor: Encryptor + Decryptor {}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum Version {
    V1,
    UNKNOWN,
}

impl From<u8> for Version {
    fn from(v: u8) -> Self {
        match v {
            1 => Version::V1,
            _ => Version::UNKNOWN,
        }
    }
}

impl From<Version> for u8 {
    fn from(v: Version) -> Self {
        match v {
            Version::V1 => 1,
            Version::UNKNOWN => 0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum CipherKind {
    AES128GCM,
    AES256GCM,
    NONE,
    UNKNOWN,
}

impl From<u8> for CipherKind {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => CipherKind::NONE,
            0x01 => CipherKind::AES128GCM,
            0x02 => CipherKind::AES256GCM,
            _ => CipherKind::UNKNOWN,
        }
    }
}

impl From<CipherKind> for u8 {
    fn from(orig: CipherKind) -> Self {
        match orig {
            CipherKind::NONE => 0x00,
            CipherKind::AES128GCM => 0x01,
            CipherKind::AES256GCM => 0x02,
            CipherKind::UNKNOWN => 0xff,
        }
    }
}

impl core::fmt::Display for CipherKind {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            CipherKind::AES128GCM => write!(f, "AES_128_GCM"),
            CipherKind::AES256GCM => write!(f, "AES_256_GCM"),
            CipherKind::NONE => write!(f, "NONE"),
            _ => write!(f, "UNKNOWN"),
        }
    }
}

impl std::str::FromStr for CipherKind {
    type Err = LogError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AES_128_GCM" => Ok(CipherKind::AES128GCM),
            "AES_256_GCM" => Ok(CipherKind::AES256GCM),
            "NONE" => Ok(CipherKind::NONE),
            _ => Err(errors::LogError::Parse(ParseError::new(String::from(
                "unknown cipher kind",
            )))),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum CompressKind {
    ZLIB,
    NONE,
    UNKNOWN,
}

impl From<u8> for CompressKind {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => CompressKind::NONE,
            0x01 => CompressKind::ZLIB,
            _ => CompressKind::UNKNOWN,
        }
    }
}

impl From<CompressKind> for u8 {
    fn from(orig: CompressKind) -> Self {
        match orig {
            CompressKind::NONE => 0x00,
            CompressKind::ZLIB => 0x01,
            CompressKind::UNKNOWN => 0xff,
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum CompressLevel {
    Fast,
    Default,
    Best,
}

impl From<u8> for CompressLevel {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => CompressLevel::Default,
            0x01 => CompressLevel::Fast,
            0x02 => CompressLevel::Best,
            _ => CompressLevel::Default,
        }
    }
}

impl From<CompressLevel> for u8 {
    fn from(orig: CompressLevel) -> Self {
        match orig {
            CompressLevel::Default => 0x00,
            CompressLevel::Fast => 0x01,
            CompressLevel::Best => 0x02,
        }
    }
}

/// 日志头
/// 日志的版本，写入大小等
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    // 版本号，方便之后的升级
    version: Version,
    // 标记文件是否可用
    flag: u8,
    // 当前写入的下标
    recorder_position: u32,
    // 压缩方式
    compress: CompressKind,
    // 加密方式
    cipher: CipherKind,
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

impl Header {
    pub fn new() -> Self {
        Header {
            version: Version::V1,
            flag: 0,
            recorder_position: V1_LOG_HEADER_SIZE as u32,
            compress: CompressKind::ZLIB,
            cipher: CipherKind::AES128GCM,
        }
    }

    pub fn create(config: &EZLogConfig) -> Self {
        Header {
            version: config.version,
            flag: 0,
            recorder_position: V1_LOG_HEADER_SIZE as u32,
            compress: config.compress,
            cipher: config.cipher,
        }
    }

    pub fn encode(&self, writer: &mut dyn Write) -> Result<(), io::Error> {
        writer.write_all(crate::FILE_SIGNATURE)?;
        writer.write_u8(self.version.into())?;
        writer.write_u8(self.flag)?;
        writer.write_u32::<BigEndian>(self.recorder_position)?;
        writer.write_u8(self.compress.into())?;
        writer.write_u8(self.cipher.into())?;
        Ok(())
    }

    pub fn decode(reader: &mut dyn Read) -> Result<Self, errors::LogError> {
        let mut signature = [0u8; 2];
        reader.read_exact(&mut signature)?;
        let version = reader.read_u8()?;
        let flag = reader.read_u8()?;
        let mut recorder_size = reader.read_u32::<BigEndian>()?;
        if recorder_size < V1_LOG_HEADER_SIZE as u32 {
            recorder_size = V1_LOG_HEADER_SIZE as u32;
        }

        let compress = reader.read_u8()?;
        let cipher = reader.read_u8()?;
        Ok(Header {
            version: Version::from(version),
            flag,
            recorder_position: recorder_size,
            compress: CompressKind::from(compress),
            cipher: CipherKind::from(cipher),
        })
    }

    pub fn is_valid(&self, config: &EZLogConfig) -> bool {
        self.version == config.version
            && self.compress == config.compress
            && self.cipher == config.cipher
    }

    pub fn is_empty(&self) -> bool {
        self.version == Version::UNKNOWN && self.recorder_position <= V1_LOG_HEADER_SIZE as u32
    }
}

/// 单条的日志记录
#[derive(Debug, Clone)]
pub struct EZRecord {
    log_name: String,
    level: Level,
    target: String,
    time: OffsetDateTime,
    thread_id: usize,
    thread_name: String,
    content: String,
}

impl EZRecord {
    #[inline]
    pub fn builder() -> EZRecordBuilder {
        EZRecordBuilder::new()
    }

    #[inline]
    pub fn log_id(&self) -> u64 {
        crate::log_id(&self.log_name)
    }

    #[inline]
    pub fn level(&self) -> Level {
        self.level
    }

    #[inline]
    pub fn target(&self) -> &str {
        self.target.as_str()
    }

    #[inline]
    pub fn timestamp(&self) -> i64 {
        self.time.unix_timestamp()
    }

    #[inline]
    pub fn thread_id(&self) -> usize {
        self.thread_id
    }

    #[inline]
    pub fn thread_name(&self) -> &str {
        self.thread_name.as_str()
    }

    #[inline]
    pub fn content(&self) -> &str {
        self.content.as_str()
    }

    #[inline]
    pub fn to_builder(&self) -> EZRecordBuilder {
        EZRecordBuilder {
            record: EZRecord {
                log_name: self.log_name.clone(),
                level: self.level,
                target: self.target.clone(),
                time: self.time,
                thread_id: self.thread_id,
                thread_name: self.thread_name.clone(),
                content: self.content.clone(),
            },
        }
    }

    pub fn from(r: &Record) -> Self {
        let t = thread::current();
        let t_id = thread_id::get();
        let t_name = t.name().unwrap_or(UNKNOWN);
        EZRecordBuilder::new()
            .log_name(DEFAULT_LOG_NAME.to_string())
            .level(r.metadata().level().into())
            .target(r.target().to_string())
            .time(OffsetDateTime::now_utc())
            .thread_id(t_id)
            .thread_name(t_name.to_string())
            .content(format!("{}", r.args()))
            .build()
    }

    /// get EZRecord unique id
    pub fn id(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.content.hash(&mut hasher);
        self.time.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug)]
pub struct EZRecordBuilder {
    record: EZRecord,
}

impl<'a> EZRecordBuilder {
    pub fn new() -> EZRecordBuilder {
        EZRecordBuilder::default()
    }

    pub fn level(&mut self, level: Level) -> &mut Self {
        self.record.level = level;
        self
    }

    pub fn target(&mut self, target: String) -> &mut Self {
        self.record.target = target;
        self
    }

    pub fn timestamp(&mut self, timestamp: i64) -> &mut Self {
        let time = OffsetDateTime::from_unix_timestamp(timestamp)
            .unwrap_or_else(|_| OffsetDateTime::now_utc());
        self.record.time = time;
        self
    }

    pub fn time(&mut self, time: OffsetDateTime) -> &mut Self {
        self.record.time = time;
        self
    }

    pub fn thread_id(&mut self, thread_id: usize) -> &mut Self {
        self.record.thread_id = thread_id;
        self
    }

    pub fn thread_name(&mut self, thread_name: String) -> &mut Self {
        self.record.thread_name = thread_name;
        self
    }

    pub fn content(&mut self, content: String) -> &mut Self {
        self.record.content = content;
        self
    }

    pub fn log_name(&mut self, name: String) -> &mut Self {
        self.record.log_name = name;
        self
    }

    pub fn build(&mut self) -> EZRecord {
        self.record.clone()
    }
}

impl<'a> Default for EZRecordBuilder {
    fn default() -> Self {
        EZRecordBuilder {
            record: EZRecord {
                log_name: DEFAULT_LOG_NAME.to_string(),
                level: Level::Info,
                target: "".to_string(),
                time: OffsetDateTime::now_utc(),
                thread_id: thread_id::get(),
                thread_name: thread::current().name().unwrap_or("unknown").to_string(),
                content: "".to_string(),
            },
        }
    }
}

#[repr(usize)]
#[derive(Copy, Eq, Debug)]
pub enum Level {
    /// The "error" level.
    ///
    /// Designates very serious errors.
    // This way these line up with the discriminants for LevelFilter below
    // This works because Rust treats field-less enums the same way as C does:
    // https://doc.rust-lang.org/reference/items/enumerations.html#custom-discriminant-values-for-field-less-enumerations
    Error = 1,
    /// The "warn" level.
    ///
    /// Designates hazardous situations.
    Warn,
    /// The "info" level.
    ///
    /// Designates useful information.
    Info,
    /// The "debug" level.
    ///
    /// Designates lower priority information.
    Debug,
    /// The "trace" level.
    ///
    /// Designates very low priority, often extremely verbose, information.
    Trace,
}

impl Level {
    pub fn from_usize(u: usize) -> Option<Level> {
        match u {
            1 => Some(Level::Error),
            2 => Some(Level::Warn),
            3 => Some(Level::Info),
            4 => Some(Level::Debug),
            5 => Some(Level::Trace),
            _ => None,
        }
    }

    /// Returns the most verbose logging level.
    #[inline]
    pub fn max() -> Level {
        Level::Trace
    }

    /// Returns the string representation of the `Level`.
    ///
    /// This returns the same string as the `fmt::Display` implementation.
    pub fn as_str(&self) -> &'static str {
        LOG_LEVEL_NAMES[*self as usize]
    }

    /// Iterate through all supported logging levels.
    ///
    /// The order of iteration is from more severe to less severe log messages.
    ///
    /// # Examples
    ///
    /// ```
    /// use log::Level;
    ///
    /// let mut levels = Level::iter();
    ///
    /// assert_eq!(Some(Level::Error), levels.next());
    /// assert_eq!(Some(Level::Trace), levels.last());
    /// ```
    pub fn iter() -> impl Iterator<Item = Self> {
        (1..6).map(|i| Self::from_usize(i).unwrap())
    }
}

impl Clone for Level {
    #[inline]
    fn clone(&self) -> Level {
        *self
    }
}

impl PartialEq for Level {
    #[inline]
    fn eq(&self, other: &Level) -> bool {
        *self as usize == *other as usize
    }
}

impl PartialOrd for Level {
    #[inline]
    fn partial_cmp(&self, other: &Level) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }

    #[inline]
    fn lt(&self, other: &Level) -> bool {
        (*self as usize) < *other as usize
    }

    #[inline]
    fn le(&self, other: &Level) -> bool {
        *self as usize <= *other as usize
    }

    #[inline]
    fn gt(&self, other: &Level) -> bool {
        *self as usize > *other as usize
    }

    #[inline]
    fn ge(&self, other: &Level) -> bool {
        *self as usize >= *other as usize
    }
}

impl Ord for Level {
    #[inline]
    fn cmp(&self, other: &Level) -> cmp::Ordering {
        (*self as usize).cmp(&(*other as usize))
    }
}

impl From<log::Level> for Level {
    fn from(log_level: log::Level) -> Self {
        match log_level {
            log::Level::Error => Level::Error,
            log::Level::Warn => Level::Warn,
            log::Level::Info => Level::Info,
            log::Level::Debug => Level::Debug,
            log::Level::Trace => Level::Trace,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.pad(self.as_str())
    }
}

/// 内存缓存实现的[EZLog]
pub struct EZLogMemoryImpl {}

pub fn next_date(time: OffsetDateTime) -> OffsetDateTime {
    time.date().midnight().assume_utc() + Duration::days(1)
}

pub fn init_mmap_temp_file(path: &Path) -> io::Result<File> {
    // check dir exists, else create
    if let Some(p) = path.parent() {
        if !p.exists() {
            fs::create_dir_all(p)?;
        }
    }

    // create file
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;

    // check file lenth ok or set len
    let len = file.metadata()?.len();
    if len == 0 {
        file.set_len(DEFAULT_MAX_LOG_SIZE)?;
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::{BufReader, Read, Seek, SeekFrom, Write};

    use aes_gcm::aead::{Aead, NewAead};
    use aes_gcm::{Aes256Gcm, Key, Nonce}; // Or `Aes128Gcm`
    use flate2::{bufread::ZlibDecoder, write::ZlibEncoder, Compression};
    use time::OffsetDateTime;

    use crate::{
        config::EZLogConfigBuilder, event, CipherKind, CompressKind, EZLogConfig, EZLogger,
        EZRecordBuilder, Header, RECORD_SIGNATURE_END, RECORD_SIGNATURE_START, V1_LOG_HEADER_SIZE,
    };

    fn create_config() -> EZLogConfig {
        EZLogConfig::default()
    }

    fn create_all_feature_config() -> EZLogConfig {
        let key = b"an example very very secret key.";
        let nonce = b"unique nonce";
        EZLogConfigBuilder::new()
            .dir_path(
                dirs::desktop_dir()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(String::from("all_feature"))
            .file_suffix(String::from("mmap"))
            .max_size(150 * 1024)
            .compress(CompressKind::ZLIB)
            .cipher(CipherKind::AES256GCM)
            .cipher_key(key.to_vec())
            .cipher_nonce(nonce.to_vec())
            .build()
    }

    #[test]
    fn test_const() {
        assert_eq!(RECORD_SIGNATURE_START, b';');
        assert_eq!(RECORD_SIGNATURE_END, b'!');
    }

    #[test]
    fn teset_level() {
        assert!(crate::Level::Debug < crate::Level::Trace);
    }

    #[test]
    fn test_header_size() {
        let header = Header::new();
        let mut v = Vec::new();
        header.encode(&mut v).unwrap();
        assert_eq!(v.len(), V1_LOG_HEADER_SIZE);
    }

    #[test]
    fn test_compress() {
        let plaint_text = b"dsafafafaasdlfkaldfjiiwoeuriowiiwueroiwur\n";
        println!("{:?}", b"\n");
        println!("{:?}", plaint_text);

        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(plaint_text).unwrap();
        let compressed = e.finish().unwrap();

        println!("{:?}", compressed);

        let mut d = ZlibDecoder::new(compressed.as_slice());

        let mut s = Vec::new();
        d.read_to_end(&mut s).unwrap();
        assert_eq!(s, plaint_text);
    }

    /// https://docs.rs/aes-gcm/latest/aes_gcm/
    #[test]
    fn test_cipher() {
        let key = Key::from_slice(b"an example very very secret key.");
        let cipher = Aes256Gcm::new(key);

        let nonce = Nonce::from_slice(b"unique nonce"); // 96-bits; unique per message

        let ciphertext = cipher
            .encrypt(nonce, b"plaintext message".as_ref())
            .expect("encryption failure!"); // NOTE: handle this error to avoid panics!

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .expect("decryption failure!"); // NOTE: handle this error to avoid panics!

        assert_eq!(&plaintext, b"plaintext message");
    }

    #[test]
    fn test_create_log() {
        create_config();
    }

    #[test]
    fn teset_encode_decode() {
        let config = create_all_feature_config();
        let mut logger = EZLogger::new(config).unwrap();

        for i in 0..1000 {
            logger
                .append(
                    &EZRecordBuilder::default()
                        .content(format!("hello world {}", i))
                        .build(),
                )
                .unwrap();
        }

        logger.flush().unwrap();

        let config = create_all_feature_config();
        let (path, _mmap) = config.create_mmap_file(OffsetDateTime::now_utc()).unwrap();

        let origin_log = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();

        let mut reader = BufReader::new(origin_log);
        reader
            .seek(SeekFrom::Start(V1_LOG_HEADER_SIZE as u64))
            .unwrap();

        let decode = logger.decode_from_read(&mut reader).unwrap();
        println!("{}", String::from_utf8(decode).unwrap())
    }

    #[test]
    fn macro_test() {
        event!(log_create "default");

        event!(log_create "logger fail");
    }
}
