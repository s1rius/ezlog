use std::{
    cmp,
    collections::hash_map::DefaultHasher,
    fmt,
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use memmap2::{MmapMut, MmapOptions};
use time::{format_description, Date, Duration, OffsetDateTime};

use crate::events::Event;
#[allow(unused_imports)]
use crate::EZLogger;
use crate::{
    errors::LogError, events::event, logger::Header, CipherKind, CompressKind, CompressLevel,
    Version, DEFAULT_LOG_FILE_SUFFIX, DEFAULT_LOG_NAME, DEFAULT_MAX_LOG_SIZE, LOG_LEVEL_NAMES,
    MIN_LOG_SIZE,
};

pub const DATE_FORMAT: &str = "[year]_[month]_[day]";

/// A config to set up [EZLogger]
#[derive(Debug, Clone)]
pub struct EZLogConfig {
    /// max log level
    ///
    /// if record level is greater than this, it will be ignored
    pub level: Level,
    /// EZLog version
    ///
    /// logger version, default is [Version::V2]
    pub version: Version,
    /// Log file dir path
    ///
    /// all log files will be saved in this dir
    pub dir_path: String,
    /// Log name to identify the [EZLogger]
    ///
    /// log file name will be `log_name` + `file_suffix`
    pub name: String,
    /// Log file suffix
    ///
    /// file suffix, default is [DEFAULT_LOG_FILE_SUFFIX]
    pub file_suffix: String,
    /// Log file expired after duration
    ///
    /// the duration after which the log file will be trimmed
    pub trim_duration: Duration,
    /// The maxium size of log file
    ///
    /// if log file size is greater than this, logger will rotate the log file
    pub max_size: u64,
    /// Log content compress kind.
    ///
    // compress kind, default is [CompressKind::NONE]
    pub compress: CompressKind,
    /// Log content compress level.
    ///
    /// compress level, default is [CompressLevel::Default]
    pub compress_level: CompressLevel,
    /// Log content cipher kind.
    ///
    /// cipher kind, default is [CipherKind::NONE]
    pub cipher: CipherKind,
    /// Log content cipher key.
    ///
    /// cipher key, default is `None`
    pub cipher_key: Option<Vec<u8>>,
    /// Log content cipher nonce.
    ///
    /// cipher nonce, default is `None`
    pub cipher_nonce: Option<Vec<u8>>,
    /// rotate duration
    ///
    /// the duration after which the log file will be rotated
    pub rotate_duration: Duration,

    /// Extra info to be added to log header
    ///
    /// Plaintext infomation write in log file header
    pub extra: Option<String>,
}

impl EZLogConfig {
    pub(crate) fn file_name(&self) -> crate::Result<String> {
        let str = format!("{}.{}", self.name, self.file_suffix);
        Ok(str)
    }

    pub(crate) fn file_name_with_date(
        &self,
        time: OffsetDateTime,
        count: i32,
    ) -> crate::Result<String> {
        let format = time::format_description::parse(DATE_FORMAT).map_err(|e| {
            crate::errors::LogError::Parse(format!(
                "Unable to create a formatter; this is a bug in EZLogConfig#file_name_with_date: {}",
                e
            ))
        })?;
        let date = time.format(&format).map_err(|_| {
            crate::errors::LogError::Parse(
                "Unable to format date; this is a bug in EZLogConfig#file_name_with_date"
                    .to_string(),
            )
        })?;
        let new_name = format!("{}_{}.{}.{}", self.name, date, count, self.file_suffix);
        Ok(new_name)
    }

    pub fn is_valid(&self) -> bool {
        !self.dir_path.is_empty() && !self.name.is_empty() && !self.file_suffix.is_empty()
    }

    pub fn create_mmap_file(&self) -> crate::Result<(PathBuf, MmapMut)> {
        let (file, path) = self.create_log_file()?;
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        Ok((path, mmap))
    }

    pub(crate) fn create_log_file(&self) -> crate::Result<(File, PathBuf)> {
        let file_name = self.file_name()?;
        let max_size = cmp::max(self.max_size, MIN_LOG_SIZE);
        let path = Path::new(&self.dir_path).join(file_name);

        if let Some(p) = &path.parent() {
            if !p.exists() {
                fs::create_dir_all(p)?;
            }
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;
        let mut len = file.metadata()?.len();
        len = if len != max_size && len != 0 {
            len
        } else {
            max_size
        };
        file.set_len(len)?;
        Ok((file, path))
    }

    pub(crate) fn is_file_out_of_date(&self, file_name: &str) -> crate::Result<bool> {
        if file_name == format!("{}.{}", &self.name, &self.file_suffix) {
            // ignore logging file
            return Ok(false);
        }
        let log_date = self.read_file_name_as_date(file_name)?;
        let now = OffsetDateTime::now_utc();
        Ok(self.is_out_of_date(log_date, now))
    }

    pub(crate) fn read_file_name_as_date(&self, file_name: &str) -> crate::Result<OffsetDateTime> {
        const SAMPLE: &str = "2022_02_22";
        if file_name == format!("{}.{}", &self.name, &self.file_suffix) {
            return Err(LogError::Illegal(
                "The file is logging file".to_string(),
            ));
        }
        if !file_name.starts_with(format!("{}_", &self.name).as_str()) {
            return Err(LogError::Illegal(format!(
                "file name is not start with name {}",
                file_name
            )));
        }
        if file_name.len() < self.name.len() + 1 + SAMPLE.len() {
            return Err(LogError::Illegal(format!(
                "file name length is not right {}",
                file_name
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
        target + self.trim_duration < now
    }

    pub(crate) fn is_file_same_date(&self, file_name: &str, date: Date) -> bool {
        if file_name == format!("{}.{}", &self.name, &self.file_suffix) {
            // ignore logging file
            return false;
        }

        self.read_file_name_as_date(file_name)
            .map(|log_date| log_date.date() == date)
            .unwrap_or(false)
    }

    pub(crate) fn writable_size(&self) -> u64 {
        self.max_size - Header::length_compat(&self.version) as u64
    }

    pub fn query_log_files_for_date(&self, date: Date) -> Vec<PathBuf> {
        let mut logs = Vec::new();
        match fs::read_dir(&self.dir_path) {
            Ok(dir) => {
                for file in dir {
                    match file {
                        Ok(file) => {
                            if let Some(name) = file.file_name().to_str() {
                                if self.is_file_same_date(name, date) {
                                    logs.push(file.path());
                                }
                            };
                        }
                        Err(e) => {
                            event!(Event::RequestLogError, "get dir entry in dir", &e.into());
                        }
                    }
                }
            }
            Err(e) => event!(Event::RequestLogError, "read dir", &e.into()),
        }
        logs
    }

    pub(crate) fn rotate_time(&self, time: OffsetDateTime) -> OffsetDateTime {
        time + self.rotate_duration
    }

    pub(crate) fn cipher_hash(&self) -> u32 {
        let mut hasher = DefaultHasher::new();
        self.cipher.hash(&mut hasher);
        self.cipher_key.hash(&mut hasher);
        hasher.finish() as u32
    }

    pub fn check_valid(&self) -> crate::Result<()> {
        if self.dir_path.is_empty() {
            return Err(LogError::Illegal("dir_path is empty".to_string()));
        }
        if self.name.is_empty() {
            return Err(LogError::Illegal("name is empty".to_string()));
        }
        Ok(())
    }
}

impl Default for EZLogConfig {
    fn default() -> Self {
        EZLogConfigBuilder::new().build()
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
        self.extra.hash(state)
    }
}

/// The builder of [EZLogConfig]
#[derive(Debug, Clone)]
pub struct EZLogConfigBuilder {
    config: EZLogConfig,
}

impl EZLogConfigBuilder {
    pub fn new() -> Self {
        EZLogConfigBuilder {
            config: EZLogConfig {
                level: Level::Trace,
                version: Version::V2,
                dir_path: "".to_string(),
                name: DEFAULT_LOG_NAME.to_string(),
                file_suffix: DEFAULT_LOG_FILE_SUFFIX.to_string(),
                trim_duration: Duration::days(7),
                max_size: DEFAULT_MAX_LOG_SIZE,
                compress: CompressKind::NONE,
                compress_level: CompressLevel::Default,
                cipher: CipherKind::NONE,
                cipher_key: None,
                cipher_nonce: None,
                rotate_duration: Duration::days(1),
                extra: None,
            },
        }
    }

    #[inline]
    pub fn version(mut self, version: Version) -> Self {
        self.config.version = version;
        self
    }

    #[inline]
    pub fn level(mut self, level: Level) -> Self {
        self.config.level = level;
        self
    }

    #[inline]
    pub fn dir_path(mut self, dir_path: String) -> Self {
        self.config.dir_path = dir_path;
        self
    }

    #[inline]
    pub fn name(mut self, name: String) -> Self {
        self.config.name = name;
        self
    }

    #[inline]
    pub fn file_suffix(mut self, file_suffix: String) -> Self {
        self.config.file_suffix = file_suffix;
        self
    }

    #[inline]
    pub fn trim_duration(mut self, duration: Duration) -> Self {
        self.config.trim_duration = duration;
        self
    }

    #[inline]
    pub fn max_size(mut self, max_size: u64) -> Self {
        self.config.max_size = max_size;
        self
    }

    #[inline]
    pub fn compress(mut self, compress: CompressKind) -> Self {
        self.config.compress = compress;
        self
    }

    #[inline]
    pub fn compress_level(mut self, compress_level: CompressLevel) -> Self {
        self.config.compress_level = compress_level;
        self
    }

    #[inline]
    pub fn cipher(mut self, cipher: CipherKind) -> Self {
        self.config.cipher = cipher;
        self
    }

    #[inline]
    pub fn cipher_key(mut self, cipher_key: Vec<u8>) -> Self {
        self.config.cipher_key = Some(cipher_key);
        self
    }

    #[inline]
    pub fn cipher_nonce(mut self, cipher_nonce: Vec<u8>) -> Self {
        self.config.cipher_nonce = Some(cipher_nonce);
        self
    }

    #[inline]
    pub fn from_header(mut self, header: &Header) -> Self {
        self.config.version = header.version;
        self.config.compress = header.compress;
        self.config.cipher = header.cipher;
        self
    }

    #[inline]
    pub fn rotate_duration(mut self, duration: Duration) -> Self {
        self.config.rotate_duration = duration;
        self
    }

    #[inline]
    pub fn extra(mut self, extra: String) -> Self {
        self.config.extra = Some(extra);
        self
    }

    #[inline]
    pub fn build(self) -> EZLogConfig {
        self.config
    }
}

impl Default for EZLogConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn parse_date_from_str(date_str: &str, case: &str) -> crate::Result<Date> {
    let format = format_description::parse(DATE_FORMAT)
        .map_err(|_e| crate::errors::LogError::Parse(format!("{} {} {}", case, date_str, _e)))?;
    let date = Date::parse(date_str, &format)
        .map_err(|_e| crate::errors::LogError::Parse(format!("{} {} {}", case, date_str, _e)))?;
    Ok(date)
}

/// Log level, used to filter log records
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
        (1..6).map(|i| Self::from_usize(i).unwrap_or(Level::Error))
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

#[cfg(feature = "log")]
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

#[cfg(test)]
mod tests {

    use std::fs::{self, OpenOptions};

    use crate::{appender::EZAppender, CipherKind, CompressKind, EZLogConfigBuilder};
    use time::{macros::datetime, Duration, OffsetDateTime};

    #[test]
    fn test_config_cipher_hash() {
        let config_builder = EZLogConfigBuilder::default();

        let default1 = config_builder.clone().build();
        let default2 = config_builder.clone().build();
        assert_eq!(default1.cipher_hash(), default2.cipher_hash());

        let cipher1 = config_builder
            .clone()
            .cipher(CipherKind::AES128GCMSIV)
            .cipher_key(vec![])
            .build();
        let cipher2 = config_builder
            .clone()
            .cipher(CipherKind::AES128GCMSIV)
            .cipher_key(vec![])
            .build();
        assert_eq!(cipher1.cipher_hash(), cipher2.cipher_hash());

        let cipher3 = config_builder
            .clone()
            .cipher(CipherKind::AES256GCMSIV)
            .cipher_key(vec![])
            .build();
        assert_ne!(cipher1.cipher_hash(), cipher3.cipher_hash());

        let cipher4 = config_builder
            .clone()
            .cipher(CipherKind::AES128GCMSIV)
            .cipher_key(vec![1, 2, 3])
            .build();
        assert_ne!(cipher1.cipher_hash(), cipher4.cipher_hash());
    }

    #[test]
    fn test_is_out_of_date() {
        let config = EZLogConfigBuilder::default()
            .trim_duration(Duration::days(1))
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

    #[test]
    fn test_query_log_files() {
        let temp = dirs::cache_dir().unwrap().join("ezlog_test_config");
        if temp.exists() {
            fs::remove_dir_all(&temp).unwrap();
        }

        let key = b"an example very very secret key.";
        let nonce = b"unique nonce";
        let config = EZLogConfigBuilder::new()
            .dir_path(temp.clone().into_os_string().into_string().unwrap())
            .name(String::from("all_feature"))
            .file_suffix(String::from("mmap"))
            .compress(CompressKind::ZLIB)
            .cipher(CipherKind::AES128GCMSIV)
            .cipher_key(key.to_vec())
            .cipher_nonce(nonce.to_vec())
            .max_size(1024)
            .build();

        let mut appender = EZAppender::create_inner(&config).unwrap();
        let f = OpenOptions::new()
            .write(true)
            .create(true)
            .open(appender.file_path())
            .unwrap();
        appender.write(&[0u8; 512]).unwrap();
        drop(appender);

        f.set_len((crate::Header::max_length() + 1) as u64).unwrap();

        let mut appender = EZAppender::new(std::rc::Rc::new(config.clone())).unwrap();
        appender.check_config_rolling(&config).unwrap();
        drop(appender);

        let files = config.query_log_files_for_date(OffsetDateTime::now_utc().date());

        assert_eq!(files.len(), 1);
        if temp.exists() {
            fs::remove_dir_all(&temp).unwrap();
        }
    }
}
