mod appender;
mod compress;
mod crypto;
mod errors;

use appender::EZMmapAppender;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use compress::ZlibCodec;
use crossbeam_channel::{Receiver, Sender};
use crypto::{Aes128Gcm, Aes256Gcm};
use errors::{CryptoError, LogError, ParseError};
use log::{Level, Record};
use memmap2::{MmapMut, MmapOptions};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    env,
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    io::{self, Cursor, Read, Write},
    mem::MaybeUninit,
    path::{Path, PathBuf},
    ptr,
    rc::Rc,
    sync::Once,
    thread,
};
use time::{format_description, Duration, OffsetDateTime};

pub const FILE_SIGNATURE: &'static [u8; 2] = b"ez";
pub const DEFAULT_LOG_NAME: &'static str = "default";
pub const DEFAULT_LOG_FILE_SUFFIX: &'static str = "mmap";
/// ";"
pub const RECORD_SIGNATURE_START: u8 = 0x3b;
pub const RECORD_SIGNATURE_END: u8 = 0x21;
pub const UNKNOWN: &'static str = "UNKNOWN";

pub const DEFAULT_MAX_LOG_SIZE: u64 = 150 * 1024;
pub const V1_LOG_HEADER_SIZE: usize = 10;

static mut CHANNEL: MaybeUninit<(Sender<EZMsg>, Receiver<EZMsg>)> = MaybeUninit::uninit();
static CHANNEL_INIT: Once = Once::new();

static mut LOG_MAP: MaybeUninit<HashMap<u64, EZLogger>> = MaybeUninit::uninit();
static LOG_MAP_INIT: Once = Once::new();

static ONE_RECEIVER: Once = Once::new();

#[inline]
fn get_channel() -> &'static (Sender<EZMsg>, Receiver<EZMsg>) {
    CHANNEL_INIT.call_once(|| unsafe {
        ptr::write(CHANNEL.as_mut_ptr(), crossbeam_channel::unbounded());
        println!("channel create")
    });

    unsafe { &*CHANNEL.as_ptr() }
}

#[inline]
fn get_map() -> &'static mut HashMap<u64, EZLogger> {
    LOG_MAP_INIT.call_once(|| unsafe {
        ptr::write(LOG_MAP.as_mut_ptr(), HashMap::new());
        println!("map create");
    });
    unsafe { &mut (*LOG_MAP.as_mut_ptr()) }
}

#[inline]
fn get_sender() -> Sender<EZMsg> {
    get_channel().0.clone()
}

#[no_mangle]
pub extern "C" fn hello_from_rust() {
    println!("Hello from Rust!");
}

#[no_mangle]
pub extern "C" fn create_ezlog() {
    println!("Hello from Rust!");
}

/// 初始化
pub fn init() {
    init_receiver();
}

pub(crate) fn init_receiver() {
    ONE_RECEIVER.call_once(|| {
        thread::spawn(|| loop {
            if let Some(msg) = get_channel().1.recv().ok() {
                println!("message recv");
                match msg {
                    EZMsg::CreateLogger(config) => {
                        let log_id = config.log_id();
                        println!("create log id: {}", log_id);
                        match EZLogger::new(config) {
                            Ok(log) => {
                                let map = get_map();
                                map.insert(log_id, log);
                            }
                            Err(e) => {
                                println!("create logger error:{:?}", e);
                            }
                        };
                    }
                    EZMsg::RecordMsg(record) => {
                        let log = match get_map().get_mut(&record.log_id) {
                            Some(l) => l,
                            None => {
                                println!("log lost : {:?}", record);
                                continue;
                            }
                        };
                        if log.config.level < record.level {
                            println!("log leve filter : {:?}", &record.level);
                            continue;
                        }
                        match log.append(&record) {
                            Ok(_) => {
                                println!("append record ok: {:?}", record);
                            }
                            Err(_) => {
                                println!("append record error: {:?}", record);
                            }
                        }
                    }
                    EZMsg::ForceFlush(name) => {
                        let mut hasher = DefaultHasher::new();
                        name.hash(&mut hasher);
                        let id = hasher.finish();
                        let log = match get_map().get_mut(&id) {
                            Some(l) => l,
                            None => {
                                println!("log lost : {:?}", name);
                                continue;
                            }
                        };
                        log.appender.flush().ok();
                        println!("log flush : {:?}", name);
                    }
                }
            }
        });
    });
}

pub fn create_log(config: EZLogConfig) {
    let msg = EZMsg::CreateLogger(config);
    match get_sender().send(msg) {
        Ok(_) => {
            println!("send create log")
        }
        Err(_) => {
            println!("create log error: ");
        }
    }
}

pub fn log(record: EZRecord) {
    let msg = EZMsg::RecordMsg(record);
    match get_sender().try_send(msg) {
        Ok(_) => {
            println!("send record")
        }
        Err(_) => {
            println!("log error: ");
        }
    }
}

pub fn flush(log_name: &str) {
    let msg = EZMsg::ForceFlush(log_name.to_string());
    match get_sender().try_send(msg) {
        Ok(_) => {
            println!("flush log")
        }
        Err(_) => {
            println!("flush error: ");
        }
    }
}

pub fn post_msg(msg: EZMsg) -> Result<(), crossbeam_channel::TrySendError<EZMsg>> {
    get_sender().try_send(msg)
}

pub fn log_id(name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone)]
pub enum EZMsg {
    CreateLogger(EZLogConfig),
    RecordMsg(EZRecord),
    ForceFlush(String),
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
        self.appender.write(&buf)?;
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
        chunk.push(RECORD_SIGNATURE_START);
        let mut buf = self.encode(record)?;
        let size = buf.len();
        let mut size_chunk = EZLogger::create_size_chunk(size)?;
        chunk.append(&mut size_chunk);
        chunk.append(&mut buf);
        chunk.push(RECORD_SIGNATURE_END);
        Ok(chunk)
    }

    fn create_size_chunk(size: usize) -> Result<Vec<u8>, LogError> {
        let mut chunk: Vec<u8> = Vec::new();
        match size {
            // u8::MAX
            0usize..=255usize => {
                chunk.write_u8(1)?;
                chunk.write_u8(size as u8)?;
            }
            // u16::MAX
            256usize..=65535usize => {
                chunk.write_u8(2)?;
                chunk.write_u16::<BigEndian>(size as u16)?;
            }
            // u32::MAX
            65536usize..=4294967295usize => {
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
        self.decode(&mut chunk)
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
        return format!("{:?}", record).into_bytes();
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        self.appender.flush()
    }
}

#[derive(Debug, Clone)]
pub struct EZLogConfig {
    /// log等级
    level: Level,
    /// 版本号
    version: Version,
    /// 文件夹目录
    dir_path: String,
    /// 文件的前缀名
    name: String,
    /// 文件的后缀名
    file_suffix: String,
    /// 文件缓存的时间
    duration: Duration,
    /// 日志文件的最大大小
    max_size: u64,
    // 压缩方式
    compress: CompressKind,
    /// 压缩等级
    compress_level: CompressLevel,
    /// 加密方式
    cipher: CipherKind,
    /// 加密的密钥
    cipher_key: Option<Vec<u8>>,
    /// 加密的nonce
    cipher_nonce: Option<Vec<u8>>,
}

impl EZLogConfig {
    pub fn new(
        level: Level,
        version: Version,
        dir_path: String,
        name: String,
        file_suffix: String,
        duration: Duration,
        max_size: u64,
        compress: CompressKind,
        compress_level: CompressLevel,
        cipher: CipherKind,
        cipher_key: Option<Vec<u8>>,
        cipher_nonce: Option<Vec<u8>>,
    ) -> Self {
        EZLogConfig {
            level,
            version,
            dir_path,
            name,
            file_suffix,
            duration,
            max_size,
            compress,
            compress_level,
            cipher,
            cipher_key,
            cipher_nonce,
        }
    }

    pub fn now_file_name(&self, now: OffsetDateTime) -> String {
        let format = format_description::parse("[year]_[month]_[day]")
            .expect("Unable to create a formatter; this is a bug in tracing-appender");
        let date = now
            .format(&format)
            .expect("Unable to format OffsetDateTime; this is a bug in tracing-appender");
        let str = format!("{}_{}.{}", self.name, date, self.file_suffix);
        str
    }

    pub fn create_mmap_file(&self, time: OffsetDateTime) -> io::Result<(File, PathBuf)> {
        let file_name = self.now_file_name(time);
        let max_size = self.max_size;
        let path = Path::new(&self.dir_path).join(file_name);

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
            .open(&path)?;

        // check file lenth ok or set len
        let mut len = file.metadata()?.len();
        if len == 0 {
            println!("set file len");
            len = max_size;
            if len == 0 {
                len = DEFAULT_MAX_LOG_SIZE;
            }
            file.set_len(len)?;
        }

        Ok((file, path))
    }

    fn log_id(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for EZLogConfig {
    fn default() -> Self {
        EZLogConfigBuilder::new()
            .dir_path(
                env::current_dir()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(DEFAULT_LOG_NAME.to_string())
            .file_suffix(String::from("mmap"))
            .max_size(1024)
            .build()
    }
}

impl Hash for EZLogConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.version.hash(state);
        self.dir_path.hash(state);
        self.name.hash(state);
        self.compress.hash(state);
        self.cipher.hash(state);
        self.cipher_key.hash(state);
        self.cipher_nonce.hash(state);
    }
}

pub struct EZLogConfigBuilder {
    config: EZLogConfig,
}

impl EZLogConfigBuilder {
    pub fn new() -> Self {
        EZLogConfigBuilder {
            config: EZLogConfig {
                level: Level::Trace,
                version: Version::V1,
                dir_path: "".to_string(),
                name: DEFAULT_LOG_NAME.to_string(),
                file_suffix: DEFAULT_LOG_FILE_SUFFIX.to_string(),
                duration: Duration::days(7),
                max_size: DEFAULT_MAX_LOG_SIZE,
                compress: CompressKind::NONE,
                compress_level: CompressLevel::Default,
                cipher: CipherKind::NONE,
                cipher_key: None,
                cipher_nonce: None,
            },
        }
    }

    pub fn level(mut self, level: Level) -> Self {
        self.config.level = level;
        self
    }

    pub fn dir_path(mut self, dir_path: String) -> Self {
        self.config.dir_path = dir_path;
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.config.name = name;
        self
    }

    pub fn file_suffix(mut self, file_suffix: String) -> Self {
        self.config.file_suffix = file_suffix;
        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.config.duration = duration;
        self
    }

    pub fn max_size(mut self, max_size: u64) -> Self {
        self.config.max_size = max_size;
        self
    }

    pub fn compress(mut self, compress: CompressKind) -> Self {
        self.config.compress = compress;
        self
    }

    pub fn cipher(mut self, cipher: CipherKind) -> Self {
        self.config.cipher = cipher;
        self
    }

    pub fn cipher_key(mut self, cipher_key: Vec<u8>) -> Self {
        self.config.cipher_key = Some(cipher_key);
        self
    }

    pub fn cipher_nonce(mut self, cipher_nonce: Vec<u8>) -> Self {
        self.config.cipher_nonce = Some(cipher_nonce);
        self
    }

    pub fn build(self) -> EZLogConfig {
        self.config
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

    pub fn encode(&self, writer: &mut dyn Write) -> Result<(), LogError> {
        writer.write(crate::FILE_SIGNATURE)?;
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
}

/// 单条的日志记录
#[derive(Debug, Clone)]
pub struct EZRecord {
    log_id: u64,
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
        self.log_id
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
                log_id: self.log_id,
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
            .id(crate::log_id(DEFAULT_LOG_NAME))
            .level(r.metadata().level())
            .target(r.target().to_string())
            .time(OffsetDateTime::now_utc())
            .thread_id(t_id)
            .thread_name(t_name.to_string())
            .content(format!("{}", r.args()))
            .build()
    }
}

#[derive(Debug)]
pub struct EZRecordBuilder {
    record: EZRecord,
}

impl<'a> EZRecordBuilder {
    pub fn new() -> EZRecordBuilder {
        EZRecordBuilder {
            record: EZRecord {
                log_id: 0,
                level: Level::Info,
                target: "".to_string(),
                time: OffsetDateTime::now_utc(),
                thread_id: 0,
                thread_name: "".to_string(),
                content: "".to_string(),
            },
        }
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
        let time =
            OffsetDateTime::from_unix_timestamp(timestamp).unwrap_or(OffsetDateTime::now_utc());
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

    pub fn id(&mut self, id: u64) -> &mut Self {
        self.record.log_id = id;
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
                log_id: 0,
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
        println!("set file len");
        file.set_len(DEFAULT_MAX_LOG_SIZE)?;
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};

    use aes_gcm::aead::{Aead, NewAead};
    use aes_gcm::{Aes256Gcm, Key, Nonce}; // Or `Aes128Gcm`
    use flate2::{bufread::ZlibDecoder, write::ZlibEncoder, Compression};
    use time::OffsetDateTime;

    use crate::{
        CipherKind, CompressKind, EZLogConfig, EZLogConfigBuilder, EZLogger, EZRecord,
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
        let (mut _file, path) = config.create_mmap_file(OffsetDateTime::now_utc()).unwrap();

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
}
