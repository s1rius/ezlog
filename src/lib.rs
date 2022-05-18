mod appender;
mod errors;

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use errors::{LogError, ParseError};
use log::{info, Level};
use memmap2::{MmapMut, MmapOptions};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Cursor, Read, Write},
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        RwLock,
    },
};
use time::{format_description, Duration, OffsetDateTime, Time};

pub const MAX_LOG_SIZE: u64 = 150 * 1024;
pub const V1_LOG_HEADER_SIZE: usize = 8;

pub struct EZLogger {}

pub trait EZLog {
    fn enable(&self, level: Level) -> bool;

    fn log(&self, record: &EZRecord);

    fn flush(&self);
}

#[derive(Debug, Clone)]
pub struct EZLogConfig {
    version: Version,
    // 文件夹目录
    dir_path: String,
    // 文件的前缀名
    name: String,
    // 文件的后缀名
    file_suffix: String,
    // 文件缓存的时间
    duration_ts: usize,
    // 日志文件的最大大小
    max_size: usize,
    // 单条日志的分隔符
    seperate: char,
    // 压缩方式
    compress: CompressKind,
    // 加密方式
    cipher: CipherKind,
}

impl EZLogConfig {
    pub fn new(
        version: Version,
        dir_path: String,
        name: String,
        file_suffix: String,
        duration_ts: usize,
        max_size: usize,
        seperate: char,
        compress: CompressKind,
        cipher: CipherKind,
    ) -> Self {
        EZLogConfig {
            version,
            dir_path,
            name,
            file_suffix,
            duration_ts,
            max_size,
            seperate,
            compress,
            cipher,
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
                version: Version::V1,
                dir_path: "".to_string(),
                name: "".to_string(),
                file_suffix: "".to_string(),
                duration_ts: 0,
                max_size: 0,
                seperate: '\n',
                compress: CompressKind::NONE,
                cipher: CipherKind::NONE,
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

    pub fn duration_ts(mut self, duration_ts: usize) -> Self {
        self.config.duration_ts = duration_ts;
        self
    }

    pub fn max_size(mut self, max_size: usize) -> Self {
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
pub trait Compress {
    fn compress(&self, data: &[u8]) -> Vec<u8>;
}

/// 加密
pub trait Encrypt {
    fn encrypt(&self, data: &[u8]) -> Vec<u8>;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Version {
    V1,
    UNKNOWN
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
        writer.write_u8(self.version.into())?;
        writer.write_u8(self.flag)?;
        writer.write_u32::<BigEndian>(self.recorder_size)?;
        writer.write_u8(self.compress.into())?;
        writer.write_u8(self.cipher.into())?;
        Ok(())
    }

    pub fn decode(reader: &mut dyn Read) -> Result<Self, errors::LogError> {
        // let mut reader = Cursor::new(data);
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

    pub fn is_valid(&self) -> bool {
        self.version != Version::UNKNOWN
    }
}

/// 单条的日志记录
#[derive(Debug, Clone)]
pub struct EZRecord<'a> {
    log_id: u64,
    level: Level,
    target: &'a str,
    timestamp: u128,
    thread_id: u64,
    thread_name: &'a str,
    content: &'a str,
}

impl<'a> EZRecord<'a> {
    #[inline]
    pub fn builder() -> EZRecordBuilder<'a> {
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
    pub fn target(&self) -> &'a str {
        self.target
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
    pub fn thread_name(&self) -> &'a str {
        self.thread_name
    }

    #[inline]
    pub fn content(&self) -> &'a str {
        self.content
    }

    #[inline]
    pub fn to_builder(&self) -> EZRecordBuilder<'a> {
        EZRecordBuilder {
            record: EZRecord {
                log_id: self.log_id,
                level: self.level,
                target: self.target,
                timestamp: self.timestamp,
                thread_id: self.thread_id,
                thread_name: self.thread_name,
                content: self.content,
            },
        }
    }
}

#[derive(Debug)]
pub struct EZRecordBuilder<'a> {
    record: EZRecord<'a>,
}

impl<'a> EZRecordBuilder<'a> {
    pub fn new() -> EZRecordBuilder<'a> {
        EZRecordBuilder {
            record: EZRecord {
                log_id: 0,
                level: Level::Info,
                target: "",
                timestamp: 0,
                thread_id: 0,
                thread_name: "",
                content: "",
            },
        }
    }

    pub fn level(&mut self, level: Level) -> &mut Self {
        self.record.level = level;
        self
    }

    pub fn target(&mut self, target: &'a str) -> &mut Self {
        self.record.target = target;
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
        self.record.thread_name = thread_name;
        self
    }

    pub fn content(&mut self, content: &'a str) -> &mut Self {
        self.record.content = content;
        self
    }

    pub fn build(&mut self) -> EZRecord {
        self.record.clone()
    }
}

impl<'a> Default for EZRecordBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

/// 内存缓存实现的[EZLog]
pub struct EZLogMemoryImpl {}

pub fn next_date(time: OffsetDateTime) -> OffsetDateTime {
    time.date().midnight().assume_utc() + Duration::days(1)
}

pub fn create_mmap_file(directory: &str, filename: &str) -> io::Result<File> {
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
    let len = file.metadata()?.len();
    if len == 0 {
        info!("set file len");
        file.set_len(MAX_LOG_SIZE)?;
    }
    Ok(file)
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
        file.set_len(MAX_LOG_SIZE)?;
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        time::SystemTime, mem,
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
