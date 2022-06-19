use std::{
    env,
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use memmap2::{MmapMut, MmapOptions};
use time::{format_description, Date, Duration, OffsetDateTime};

use crate::{
    errors::{IllegalArgumentError, LogError, ParseError},
    CipherKind, CompressKind, CompressLevel, Header, Level, Version, DEFAULT_LOG_FILE_SUFFIX,
    DEFAULT_LOG_NAME, DEFAULT_MAX_LOG_SIZE,
};

pub const DATE_FORMAT: &str = "[year]_[month]_[day]";

#[derive(Debug, Clone)]
pub struct EZLogConfig {
    /// log等级
    pub level: Level,
    /// 版本号
    pub version: Version,
    /// 文件夹目录
    pub dir_path: String,
    /// 文件的前缀名
    pub name: String,
    /// 文件的后缀名
    pub file_suffix: String,
    /// 文件缓存的时间
    pub duration: Duration,
    /// 日志文件的最大大小
    pub max_size: u64,
    // 压缩方式
    pub compress: CompressKind,
    /// 压缩等级
    pub compress_level: CompressLevel,
    /// 加密方式
    pub cipher: CipherKind,
    /// 加密的密钥
    pub cipher_key: Option<Vec<u8>>,
    /// 加密的nonce
    pub cipher_nonce: Option<Vec<u8>>,
}

impl EZLogConfig {
    pub(crate) fn now_file_name(&self, now: OffsetDateTime) -> crate::Result<String> {
        let format = format_description::parse(DATE_FORMAT).map_err(|_e| {
            crate::errors::LogError::Parse(ParseError::new(format!(
                "Unable to create a formatter; this is a bug in EZLogConfig#now_file_name: {}",
                _e
            )))
        })?;
        let date = now.format(&format).map_err(|_| {
            crate::errors::LogError::Parse(ParseError::new(
                "Unable to format date; this is a bug in EZLogConfig#now_file_name".to_string(),
            ))
        })?;
        let str = format!("{}_{}.{}", self.name, date, self.file_suffix);
        Ok(str)
    }

    pub fn create_mmap_file(&self, time: OffsetDateTime) -> crate::Result<(PathBuf, MmapMut)> {
        let file_name = self.now_file_name(time)?;
        let max_size = self.max_size;
        let path = Path::new(&self.dir_path).join(file_name);

        let file = internal_create_log_file(&path, max_size)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok((path, mmap))
    }

    pub(crate) fn is_file_out_of_date(&self, file_name: &str) -> crate::Result<bool> {
        let log_date = self.read_file_name_as_date(file_name)?;
        let now = OffsetDateTime::now_utc();
        Ok(self.is_out_of_date(log_date, now))
    }

    pub(crate) fn read_file_name_as_date(&self, file_name: &str) -> crate::Result<OffsetDateTime> {
        const SAMPLE: &str = "2022_02_22";
        if !file_name.starts_with(format!("{}_", &self.name).as_str()) {
            return Err(LogError::IllegalArgument(IllegalArgumentError::new(
                format!("file name is not start with name {}", file_name),
            )));
        }
        if !file_name.len() < self.name.len() + SAMPLE.len() + 1 {
            return Err(LogError::IllegalArgument(IllegalArgumentError::new(
                format!("file name length is not right {}", file_name),
            )));
        }
        let date_str = &file_name[self.name.len() + 1..self.name.len() + 1 + SAMPLE.len()];
        let log_date = parse_date_from_str(
            date_str,
            "this is a bug in EZLogConfig#read_file_name_as_date:",
        )?;
        Ok(log_date.midnight().assume_utc())
    }

    fn is_out_of_date(&self, target: OffsetDateTime, now: OffsetDateTime) -> bool {
        if target.year() < now.year() {
            return true;
        }

        if target.year() == now.year() && target + self.duration < now {
            return true;
        }

        false
    }

    pub(crate) fn is_file_same_date(&self, file_name: &str, date: Date) -> bool {
        if let Ok(log_date) = self.read_file_name_as_date(file_name) {
            if log_date.date() == date {
                return true;
            }
        }
        false
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

    pub fn compress_level(mut self, compress_level: CompressLevel) -> Self {
        self.config.compress_level = compress_level;
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

    pub fn from_header(mut self, header: &Header) -> Self {
        self.config.version = header.version;
        self.config.compress = header.compress;
        self.config.cipher = header.cipher;
        self
    }

    pub fn build(self) -> EZLogConfig {
        self.config
    }
}

impl Default for EZLogConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn internal_create_log_file(path: &PathBuf, max_size: u64) -> crate::Result<File> {
    if let Some(p) = path.parent() {
        if !p.exists() {
            fs::create_dir_all(p)?;
        }
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;
    let mut len = file.metadata()?.len();
    if len == 0 {
        len = max_size;
        if len == 0 {
            len = DEFAULT_MAX_LOG_SIZE;
        }
        file.set_len(len)?;
    }
    Ok(file)
}

pub(crate) fn parse_date_from_str(date_str: &str, case: &str) -> crate::Result<Date> {
    let format = format_description::parse(DATE_FORMAT).map_err(|_e| {
        crate::errors::LogError::Parse(ParseError::new(format!("{} {} {}", case, date_str, _e)))
    })?;
    let date = Date::parse(date_str, &format).map_err(|_e| {
        crate::errors::LogError::Parse(ParseError::new(format!("{} {} {}", case, date_str, _e)))
    })?;
    Ok(date)
}

#[cfg(test)]
mod tests {

    use crate::EZLogConfigBuilder;
    use time::{macros::datetime, Duration, OffsetDateTime};

    #[test]
    fn test_is_out_of_date() {
        let config = EZLogConfigBuilder::default()
            .duration(Duration::days(1))
            .build();

        assert!(!config.is_out_of_date(OffsetDateTime::now_utc(), OffsetDateTime::now_utc()));
        assert!(config.is_out_of_date(
            datetime!(2022-06-13 0:00 UTC),
            datetime!(2022-06-14 0:01 UTC)
        ));
        assert!(!config.is_out_of_date(
            datetime!(2022-06-13 0:00 UTC),
            datetime!(2022-06-14 0:00 UTC)
        ))
    }

    #[test]
    fn test_read_file_name_as_date() {
        let config = EZLogConfigBuilder::default()
            .name("test".to_string())
            .build();

        assert!(config.read_file_name_as_date("test2019_06_13.log").is_err());
        assert!(config.read_file_name_as_date("test_201_06_13.log").is_err());
        assert!(config
            .read_file_name_as_date("test_2019_06_1X.log")
            .is_err());
        assert!(config.read_file_name_as_date("test_2019_06_13.log").is_ok());
        assert!(config
            .read_file_name_as_date("test_2019_06_13.1.log")
            .is_ok());
        assert!(config
            .read_file_name_as_date("test_2019_06_13.123.mmap")
            .is_ok());
    }
}
