mod errors;

use errors::{LogError, ParseError};
use log::{info, Level};
use memmap2::MmapOptions;
use std::{
    fs::{self, File, OpenOptions},
    io,
    path::Path,
};

pub struct EZLog {}

#[derive(Debug, Clone)]
pub struct EZLogConfig {
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
}

struct EZLogConfigBuilder {
    config: EZLogConfig,
}

impl EZLogConfigBuilder {
    pub fn new() -> Self {
        EZLogConfigBuilder {
            config: EZLogConfig {
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
        self.config.clone()
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

#[derive(Debug, Clone)]
pub enum CipherKind {
    AES_128_GCM,
    AES_256_GCM,
    NONE,
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

#[derive(Debug, Clone)]
pub enum CompressKind {
    ZLIB,
    NONE,
}

/// 日志头
/// 日志的版本，写入大小等
pub struct Header {
    // 版本号，方便之后的升级
    version: u8,
    // 当前写入的下标
    cursor: u32,
    // 压缩方式
    compress: CompressKind,
    // 加密方式
    cipher: CipherKind,
}

impl Header {}

/// 单条的日志记录
#[derive(Debug, Clone)]
pub struct Recorder<'a> {
    level: Level,
    target: &'a str,
    timestamp: u128,
    thread_id: u64,
    thread_name: &'a str,
    content: &'a str,
}

impl<'a> Recorder<'a> {
    #[inline]
    pub fn builder() -> RecordBuilder<'a> {
        RecordBuilder::new()
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
    pub fn to_builder(&self) -> RecordBuilder<'a> {
        RecordBuilder {
            record: Recorder {
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
pub struct RecordBuilder<'a> {
    record: Recorder<'a>,
}

impl<'a> RecordBuilder<'a> {
    pub fn new() -> RecordBuilder<'a> {
        RecordBuilder {
            record: Recorder {
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

    pub fn build(&mut self) -> Recorder {
        self.record.clone()
    }
}

/// mmap 实现的[EZLog]
pub struct EZLogMmapImpl {}

/// 内存缓存实现的[EZLog]
pub struct EZLogMemoryImpl {}

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
        file.set_len(150 * 1024)?;
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
