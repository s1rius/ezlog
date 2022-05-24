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
use log::{info, Level};
use memmap2::{MmapMut, MmapOptions};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{self, Cursor, Read, Write},
    mem::MaybeUninit,
    path::Path,
    ptr,
    rc::Rc,
    sync::Once,
    thread,
};
use time::{format_description, Duration, OffsetDateTime};

pub const FILE_SIGNATURE: &'static [u8; 2] = b"ez";

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
    });

    unsafe { &*CHANNEL.as_ptr() }
}

#[inline]
fn get_map() -> &'static mut HashMap<u64, EZLogger> {
    LOG_MAP_INIT.call_once(|| unsafe {
        ptr::write(LOG_MAP.as_mut_ptr(), HashMap::new());
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
                match msg {
                    EZMsg::CreateMsg(config) => {
                        let log = match EZLogger::new(config) {
                            Ok(log) => log,
                            Err(e) => {
                                info!("create logger error: {:?}", e);
                                continue;
                            }
                        };
                        let map = get_map();
                        map.insert(log.id(), log);
                    }
                    EZMsg::RecordMsg(record) => {
                        let log = match get_map().get_mut(&record.log_id) {
                            Some(l) => l,
                            None => {
                                info!("log lost : {:?}", record);
                                continue;
                            }
                        };
                        if log.config.level > record.level {
                            info!("log level ");
                            continue;
                        }
                        match log.append(&record) {
                            Ok(_) => (),
                            Err(_) => {
                                info!("append record error: {:?}", record);
                                continue;
                            }
                        }
                    }
                }
            }
        });
    });
}

pub fn create_log(config: EZLogConfig) {
    let msg = EZMsg::CreateMsg(config);
    match get_sender().try_send(msg) {
        Ok(_) => {}
        Err(_) => {}
    }
}

pub fn log(record: EZRecord) {
    let msg = EZMsg::RecordMsg(record);
    match get_sender().try_send(msg) {
        Ok(_) => {}
        Err(_) => {}
    }
}

enum EZMsg {
    CreateMsg(EZLogConfig),
    RecordMsg(EZRecord),
}

pub struct EZLogger {
    config: Rc<EZLogConfig>,
    appender: EZMmapAppender,
    compression: Option<Box<dyn Compression>>,
    cryptor: Option<Box<dyn Encryptor>>,
}

impl EZLogger {
    pub fn new(config: EZLogConfig) -> Result<Self, LogError> {
        let rc_conf = Rc::new(config);
        let appender = EZMmapAppender::new(Rc::clone(&rc_conf))?;
        let compression = EZLogger::create_compression(&rc_conf)?;
        let cryptor = EZLogger::create_cryptor(&rc_conf)?;
        Ok(Self {
            config: Rc::clone(&rc_conf),
            appender,
            compression,
            cryptor,
        })
    }

    pub fn create_cryptor(config: &EZLogConfig) -> Result<Option<Box<dyn Encryptor>>, CryptoError> {
        if let Some(key) = &config.cipher_key {
            if let Some(nonce) = &config.cipher_nonce {
                match config.cipher {
                    CipherKind::AES_128_GCM => {
                        let encryptor = Aes128Gcm::new(key, nonce)?;
                        Ok(Some(Box::new(encryptor)))
                    }
                    CipherKind::AES_256_GCM => {
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

    pub fn create_compression(
        config: &EZLogConfig,
    ) -> Result<Option<Box<dyn Compression>>, LogError> {
        match config.compress {
            CompressKind::ZLIB => Ok(Some(Box::new(ZlibCodec::new(&config.compress_level)))),
            CompressKind::NONE => Ok(None),
            CompressKind::UNKNOWN => Ok(None),
        }
    }

    fn append(&mut self, record: &EZRecord) -> Result<(), LogError> {
        if self.config.level > record.level {
            // todo
            return Ok(());
        }
        let mut buf = self.format(record);
        if let Some(encryptor) = &self.cryptor {
            buf = encryptor.encrypt(&buf)?;
        }
        if let Some(compression) = &self.compression {
            buf = compression.compress(&buf)?;
        }
        self.appender.write(&buf)?;
        Ok(())
    }

    fn format(&self, record: &EZRecord) -> Vec<u8> {
        todo!()
    }

    fn id(&self) -> u64 {
        todo!()
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
    /// 单条日志的分隔符
    seperate: char,
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
        seperate: char,
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
            seperate,
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

    pub fn create_mmap_file(directory: &str, filename: &str, max_size: u64) -> io::Result<File> {
        let path = Path::new(directory).join(filename);

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
        let mut len = file.metadata()?.len();
        if len == 0 {
            info!("set file len");
            len = max_size;
            if len == 0 {
                len = DEFAULT_MAX_LOG_SIZE;
            }
            file.set_len(len)?;
        }
        Ok(file)
    }
}

impl Default for EZLogConfig {
    fn default() -> Self {
        EZLogConfigBuilder::new().build()
    }
}

struct EZLogConfigBuilder {
    config: EZLogConfig,
}

impl EZLogConfigBuilder {
    pub fn new() -> Self {
        EZLogConfigBuilder {
            config: EZLogConfig {
                level: Level::Trace,
                version: Version::V1,
                dir_path: "".to_string(),
                name: "".to_string(),
                file_suffix: "".to_string(),
                duration: Duration::days(7),
                max_size: DEFAULT_MAX_LOG_SIZE,
                seperate: '\n',
                compress: CompressKind::NONE,
                compress_level: CompressLevel::Default,
                cipher: CipherKind::NONE,
                cipher_key: None,
                cipher_nonce: None,
            },
        }
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

    pub fn separate(mut self, separate: char) -> Self {
        self.config.seperate = separate;
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

/// 加密
pub trait Encryptor {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError>;
}

pub trait Decryptor {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError>;
}

#[derive(Debug, Copy, Clone, PartialEq)]
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CipherKind {
    AES_128_GCM,
    AES_256_GCM,
    NONE,
    UNKNOWN,
}

impl From<u8> for CipherKind {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => CipherKind::NONE,
            0x01 => CipherKind::AES_128_GCM,
            0x02 => CipherKind::AES_256_GCM,
            _ => CipherKind::UNKNOWN,
        }
    }
}

impl From<CipherKind> for u8 {
    fn from(orig: CipherKind) -> Self {
        match orig {
            CipherKind::NONE => 0x00,
            CipherKind::AES_128_GCM => 0x01,
            CipherKind::AES_256_GCM => 0x02,
            CipherKind::UNKNOWN => 0xff,
        }
    }
}

impl core::fmt::Display for CipherKind {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            CipherKind::AES_128_GCM => write!(f, "AES_128_GCM"),
            CipherKind::AES_256_GCM => write!(f, "AES_256_GCM"),
            CipherKind::NONE => write!(f, "NONE"),
            _ => write!(f, "UNKNOWN"),
        }
    }
}

impl std::str::FromStr for CipherKind {
    type Err = LogError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AES_128_GCM" => Ok(CipherKind::AES_128_GCM),
            "AES_256_GCM" => Ok(CipherKind::AES_256_GCM),
            "NONE" => Ok(CipherKind::NONE),
            _ => Err(errors::LogError::Parse(ParseError::new(String::from(
                "unknown cipher kind",
            )))),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
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

#[derive(Debug, Clone)]
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
    recorder_size: u32,
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
            recorder_size: 0,
            compress: CompressKind::ZLIB,
            cipher: CipherKind::AES_128_GCM,
        }
    }

    pub fn create(config: &EZLogConfig) -> Self {
        Header {
            version: config.version,
            flag: 0,
            recorder_size: 0,
            compress: config.compress,
            cipher: config.cipher,
        }
    }

    pub fn encode(&self, writer: &mut dyn Write) -> Result<(), LogError> {
        writer.write(crate::FILE_SIGNATURE)?;
        writer.write_u8(self.version.into())?;
        writer.write_u8(self.flag)?;
        writer.write_u32::<BigEndian>(self.recorder_size)?;
        writer.write_u8(self.compress.into())?;
        writer.write_u8(self.cipher.into())?;
        Ok(())
    }

    pub fn decode(reader: &mut dyn Read) -> Result<Self, errors::LogError> {
        let mut signature = [0u8; 2];
        reader.read_exact(&mut signature)?;
        let version = reader.read_u8()?;
        let flag = reader.read_u8()?;
        let recorder_size = reader.read_u32::<BigEndian>()?;
        let compress = reader.read_u8()?;
        let cipher = reader.read_u8()?;
        Ok(Header {
            version: Version::from(version),
            flag,
            recorder_size,
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
    timestamp: u128,
    thread_id: u64,
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
    pub fn timestamp(&self) -> u128 {
        self.timestamp
    }

    #[inline]
    pub fn thread_id(&self) -> u64 {
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
                timestamp: self.timestamp,
                thread_id: self.thread_id,
                thread_name: self.thread_name.clone(),
                content: self.content.clone(),
            },
        }
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
                timestamp: 0,
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

    pub fn target(&mut self, target: &'a str) -> &mut Self {
        self.record.target = target.to_string();
        self
    }

    pub fn timestamp(&mut self, timestamp: u128) -> &mut Self {
        self.record.timestamp = timestamp;
        self
    }

    pub fn thread_id(&mut self, thread_id: u64) -> &mut Self {
        self.record.thread_id = thread_id;
        self
    }

    pub fn thread_name(&mut self, thread_name: &'a str) -> &mut Self {
        self.record.thread_name = thread_name.to_string();
        self
    }

    pub fn content(&mut self, content: &'a str) -> &mut Self {
        self.record.content = content.to_string();
        self
    }

    pub fn build(&mut self) -> EZRecord {
        self.record.clone()
    }
}

impl<'a> Default for EZRecordBuilder {
    fn default() -> Self {
        Self::new()
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
        info!("set file len");
        file.set_len(DEFAULT_MAX_LOG_SIZE)?;
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        time::SystemTime,
    };

    use aes_gcm::aead::{Aead, NewAead};
    use aes_gcm::{Aes256Gcm, Key, Nonce}; // Or `Aes128Gcm`
    use flate2::{bufread::ZlibDecoder, write::ZlibEncoder, Compression};

    use crate::{Header, V1_LOG_HEADER_SIZE};

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);

        let unix_now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        println!("{}", unix_now);
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
}
