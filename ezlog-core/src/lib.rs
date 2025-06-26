#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

//! ezlog is a high efficiency cross-platform logging library.
//!
//! It is inspired by [Xlog](https://github.com/Tencent/mars) and [Loagan](https://github.com/Meituan-Dianping/Logan), rewrite in Rust.
//!
//! Guide level documentation is found on the [website](https://s1rius.github.io/ezlog).
//!
//! ## Features
//! - multi platform: Flutter, Android, iOS, Windows, Linux, MacOS
//! - map file into memory by [mmap](https://man7.org/linux/man-pages/man2/mmap.2.html).
//! - compression support, eg: [zlib](https://en.wikipedia.org/wiki/Zlib).
//! - encryption support, eg: [AEAD encryption](https://en.wikipedia.org/wiki/Authenticated_encryption).
//! - fetch log by callback.
//! - trim out of date files.
//! - command line parser support.
//!
//! ## example
//!
//! ```
//! use ezlog::EZLogConfigBuilder;
//! use ezlog::Level;
//! use log::trace;
//!
//!
//! ezlog::InitBuilder::new().debug(true).init();
//!
//! let config: ezlog::EZLogConfig = EZLogConfigBuilder::new()
//!     .level(Level::Trace)
//!     .dir_path(
//!         dirs::cache_dir()
//!             .unwrap()
//!             .into_os_string()
//!             .into_string()
//!             .expect("dir path error"),
//!     )
//!     .build();
//! ezlog::create_log(config);
//!
//! trace!("hello ezlog");
//! ```
//!

mod appender;
mod compress;
mod config;
#[allow(deprecated)]
mod crypto;
mod errors;
mod events;
mod init;
mod logger;
mod recorder;
mod thread_name;

#[cfg(feature = "decode")]
pub mod decode;

#[cfg(any(target_os = "macos", target_os = "ios"))]
#[allow(non_snake_case)]
mod ffi_c;
#[cfg(target_os = "android")]
#[allow(non_snake_case)]
mod ffi_java;

use core::fmt;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{
    Mutex,
    OnceLock,
    RwLock,
    RwLockReadGuard,
};
use std::{
    collections::HashMap,
    hash::Hash,
    io::{
        self,
        Cursor,
        Read,
        Write,
    },
    sync::Once,
    thread,
};

use crossbeam_channel::{
    Sender,
    TrySendError,
};
use memmap2::MmapMut;
use time::Duration;
use time::OffsetDateTime;

pub use self::compress::CompressKind;
pub use self::compress::CompressLevel;
pub use self::config::EZLogConfig;
pub use self::config::EZLogConfigBuilder;
pub use self::config::Level;
pub use self::crypto::CipherKind;
pub use self::errors::LogError;
pub(crate) use self::events::event;
pub use self::events::Event;
pub use self::events::EventListener;
pub use self::events::EventPrinter;
pub use self::init::InitBuilder;
pub use self::init::MsgHandler;
pub use self::logger::create_compress;
pub use self::logger::create_cryptor;
pub use self::logger::EZLogger;
pub use self::logger::Header;
pub use self::recorder::EZRecord;
pub use self::recorder::EZRecordBuilder;

/// A [EZLogger] default name. current is "default".
pub const DEFAULT_LOG_NAME: &str = "default";
pub(crate) const FILE_SIGNATURE: &[u8; 2] = b"ez";

pub(crate) const DEFAULT_LOG_FILE_SUFFIX: &str = "mmap";
static LOG_LEVEL_NAMES: [&str; 6] = ["OFF", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

pub(crate) const RECORD_SIGNATURE_START: u8 = 0x3b;
pub(crate) const RECORD_SIGNATURE_END: u8 = 0x21;

pub(crate) const DEFAULT_MAX_LOG_SIZE: u64 = 150 * 1000;

/// Minimum log file size may greater than 4kb.
/// https://stackoverflow.com/a/26002578/2782445
pub(crate) const MIN_LOG_SIZE: u64 = 100;

/// Log file fixed header length.
pub const V1_LOG_HEADER_SIZE: usize = 10;
pub const V2_LOG_HEADER_SIZE: usize = 22;

const MAX_PRE_INIT_QUEUE_SIZE: usize = 64;

static LOG_SERVICE: OnceLock<LogService> = OnceLock::new();
static PRE_INIT_QUEUE: OnceLock<Mutex<VecDeque<EZMsg>>> = OnceLock::new();

static mut GLOBAL_CALLBACK: &dyn EZLogCallback = &NopCallback;
static CALLBACK_INIT: Once = Once::new();

static GLOBAL_FORMATTER: OnceLock<Mutex<Option<&dyn Formatter>>> = OnceLock::new();

type Result<T> = std::result::Result<T, LogError>;

/// Init ezlog
///
/// init ezlog, setup panic hook, trigger event when panic.
///
/// # Examples
/// ```
/// ezlog::init();
/// ```
#[deprecated(
    since = "0.2.0",
    note = "please use `ezlog::InitBuilder::new().init()` instead"
)]
pub fn init() {
    InitBuilder::new().init();
}

#[deprecated(
    since = "0.2.0",
    note = "please use `ezlog::InitBuilder::new().with_listener(event).init()` instead"
)]
pub fn init_with_event(event: &'static dyn EventListener) {
    InitBuilder::new().with_event_listener(event).init();
}

struct LogService {
    layers: Vec<Box<dyn MsgHandler + Send + Sync>>,
    loggers: RwLock<HashMap<String, EZLogger>>,
    log_sender: Sender<EZMsg>,
    fetch_sender: Sender<FetchResult>,
}

impl LogService {
    fn new(layers: Vec<Box<dyn MsgHandler + Send + Sync>>) -> Self {
        LogService {
            layers,
            loggers: RwLock::new(HashMap::new()),
            log_sender: init_log_channel(),
            fetch_sender: init_callback_channel(),
        }
    }

    fn dispatch(&self, msg: EZMsg) {
        self.layers.iter().for_each(|layer| layer.handle(&msg));
        self.log_sender
            .try_send(msg)
            .unwrap_or_else(crate::report_channel_send_err);
    }

    fn fetch_logs(&self, task: FetchReq) -> crate::Result<()> {
        event!(Event::RequestLog, &format!("{task:?}"));
        let mut logs: Vec<PathBuf> = Vec::new();
        let mut error: Option<String> = None;
        self.loggers_read()
            .map(|map| {
                if let Some(logger) = map.get(&task.name) {
                    // Perform operations with the logger
                    let now = OffsetDateTime::now_utc();
                    if (now < task.end || now < task.start + Duration::days(1)) && now > task.start
                    {
                        logger.rotate_if_not_empty().unwrap_or_else(|e| {
                            error = Some(format!("rotate error: {}", e));
                        });
                    }

                    let days = (task.end - task.start).whole_days();
                    for day in 0..=days {
                        let mut query =
                            logger.query_log_files_for_date(task.start + Duration::days(day));
                        logs.append(&mut query);
                    }
                } else {
                    error = Some("Logger not found".into())
                }
            })
            .unwrap_or_else(|e| {
                error = Some(format!("Error reading loggers: {}", e));
            });

        self.on_fetch(FetchResult {
            name: task.name,
            date: task.start.date().to_string(),
            logs: Some(logs),
            error,
        })
    }

    fn on_fetch(&self, result: FetchResult) -> crate::Result<()> {
        self.fetch_sender
            .try_send(result)
            .map_err(|e| LogError::unknown(&format!("{:?}", e)))
    }

    fn insert_logger(&self, name: String, log: EZLogger) -> crate::Result<()> {
        self.loggers_write().map(|mut map| {
            if map.contains_key(&name) {
                event!(Event::CreateLoggerError, "logger already exists");
                return;
            } else {
                map.insert(name.clone(), log);
            }
            event!(Event::CreateLogger, &name);
        })
    }

    fn log(&self, record: EZRecord) -> crate::Result<()> {
        self.loggers_read().map(|map| {
            if let Some(log) = map.get(&record.log_name().to_owned()) {
                // TODO: add fileter logic
                if log.config.level() < record.level() {
                    event!(
                        Event::RecordFilterOut,
                        &format!(
                            "current level {}, max level {}",
                            &record.level(),
                            &log.config.level()
                        )
                    );
                }
                log.append(record).map(|_| {})
            } else {
                Err(LogError::Illegal("no logger found".into()))
            }
        })?
    }

    fn flush(&self, name: String) -> crate::Result<()> {
        self.loggers_read().and_then(|map| {
            map.get(&name)
                .map(|v| v.flush())
                .unwrap_or_else(|| Err(LogError::Illegal("Logger not found".into())))
        })
    }

    fn flush_all(&self) -> crate::Result<()> {
        self.loggers_read()?
            .values()
            .map(|logger| logger.flush())
            .collect::<std::result::Result<Vec<()>, _>>()
            .map(|_| ())
    }

    fn trim(&self) -> crate::Result<()> {
        self.loggers_read()?
            .values()
            .for_each(|logger| logger.trim());
        Ok(())
    }

    pub(crate) fn loggers_read(
        &self,
    ) -> crate::Result<RwLockReadGuard<'_, HashMap<String, EZLogger>>> {
        self.loggers.read().map_err(errors::LogError::from)
    }

    pub(crate) fn loggers_write(
        &self,
    ) -> crate::Result<std::sync::RwLockWriteGuard<'_, HashMap<String, EZLogger>>> {
        self.loggers.write().map_err(errors::LogError::from)
    }
}

/// Trim all [EZLogger]s outdated files
///
/// manual trim the log files in disk. delete logs which are out of date.
pub fn trim() {
    event!(Event::Trim);
    post_msg(EZMsg::Trim());
}

/// Set global [EventListener]
pub fn set_event_listener(event: &'static dyn EventListener) {
    events::set_event_listener(event);
}

fn init_log_channel() -> Sender<EZMsg> {
    // TODO: check the channel full error
    let (sender, receiver) = crossbeam_channel::bounded::<EZMsg>(200);
    match thread::Builder::new()
        .name("ezlog_task".to_string())
        .spawn(move || loop {
            match receiver.recv() {
                Ok(msg) => match msg {
                    EZMsg::CreateLogger(config) => {
                        let name = config.name().to_string();
                        match EZLogger::new(config) {
                            Ok(log) => {
                                LOG_SERVICE.get().map_or_else(
                                    || {
                                        event!(Event::CreateLoggerError, "log service not init");
                                    },
                                    |service| {
                                        service
                                            .insert_logger(log.config.name().to_string(), log)
                                            .unwrap_or_else(|e| {
                                                event!(
                                                    Event::CreateLoggerError,
                                                    "create logger error",
                                                    &e
                                                );
                                            });
                                    },
                                );
                            }
                            Err(e) => {
                                event!(Event::CreateLoggerError, &name, &e);
                            }
                        };
                    }
                    EZMsg::Record(record) => {
                        LOG_SERVICE.wait().log(record).unwrap_or_else(|e| {
                            event!(Event::RecordError, &e.to_string());
                        });
                    }
                    EZMsg::ForceFlush(name) => {
                        LOG_SERVICE.wait().flush(name).unwrap_or_else(|e| {
                            event!(Event::FlushError, &e.to_string());
                        });
                    }
                    EZMsg::FlushAll() => {
                        LOG_SERVICE.wait().flush_all().unwrap_or_else(|e| {
                            event!(Event::FlushError, &e.to_string());
                        });
                    }
                    EZMsg::Trim() => {
                        LOG_SERVICE.wait().trim().unwrap_or_else(|e| {
                            event!(Event::TrimError, &e.to_string());
                        });
                    }
                    EZMsg::FetchLog(task) => {
                        LOG_SERVICE.wait().fetch_logs(task).unwrap_or_else(|e| {
                            event!(Event::RequestLogError, &e.to_string());
                        });
                    }
                    EZMsg::Action(call) => {
                        call();
                    }
                },
                Err(err) => {
                    event!(Event::ChannelError, "log channel rec", &err.into());
                }
            }
        }) {
        Ok(_) => {
            event!(Event::Init);
        }
        Err(e) => {
            event!(Event::InitError, &format!("init ezlog error {}", e));
        }
    }
    sender
}

fn init_callback_channel() -> Sender<FetchResult> {
    let (fetch_sender, fetch_receiver) = crossbeam_channel::unbounded::<FetchResult>();
    match thread::Builder::new()
        .name("ezlog_callback".to_string())
        .spawn(move || loop {
            match fetch_receiver.recv() {
                Ok(result) => {
                    invoke_fetch_callback(result);
                }
                Err(e) => event!(Event::FFiError, "init callback channel", &e.into()),
            }
        }) {
        Ok(_) => {
            event!(Event::Init, "init callback channel success");
        }
        Err(e) => {
            event!(Event::InitError, "init callback channel err", &e.into());
        }
    }
    fetch_sender
}

/// Create a new [EZLogger] from an [EZLogConfig]
pub fn create_log(config: EZLogConfig) {
    if let Err(log_error) = &config.check_valid() {
        event!(Event::CreateLoggerError, "config is not valid", log_error);
        return;
    }
    let config_desc = format!("{:?}", config);
    let msg = EZMsg::CreateLogger(config);

    event!(Event::CreateLogger, &config_desc);
    post_msg(msg);
}

/// Write a [EZRecord] to the log file
pub fn log(record: EZRecord) {
    let tid = record.t_id();
    let msg = EZMsg::Record(record);
    event!(Event::Record, &tid);
    post_msg(msg);
}

/// Force flush the log file
pub fn flush(log_name: &str) {
    let msg = EZMsg::ForceFlush(log_name.to_string());
    event!(Event::Flush, log_name);
    post_msg(msg);
}

/// Flush all log files
pub fn flush_all() {
    event!(Event::Flush);
    let msg = EZMsg::FlushAll();
    post_msg(msg);
}

/// Request logs file path array at the date which [EZLogger]'s name is define in the parameter
pub fn request_log_files_for_date(log_name: &str, start: OffsetDateTime, end: OffsetDateTime) {
    let req = FetchReq {
        name: log_name.to_string(),
        start,
        end,
    };
    post_msg(EZMsg::FetchLog(req));
}

#[inline]
fn post_msg(msg: EZMsg) {
    if let Some(service) = LOG_SERVICE.get() {
        service.dispatch(msg);
    } else {
        event!(Event::InitError, "post msg when log service not init");
        // if not init, push to pre-init queue
        let q = PRE_INIT_QUEUE.get_or_init(|| Mutex::new(VecDeque::<EZMsg>::new()));
        let mut guard = q.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(msg, EZMsg::CreateLogger(_)) {
            guard.push_front(msg);
        } else if guard.len() < MAX_PRE_INIT_QUEUE_SIZE {
            guard.push_back(msg);
        } else {
            event!(Event::InitError, "pre-init queue full, cant queue message");
        }
    }
}

#[inline]
pub(crate) fn report_channel_send_err<T>(err: TrySendError<T>) {
    event!(Event::ChannelError, "channel send err", &err.into());
}

fn invoke_fetch_callback(result: FetchResult) {
    match result.logs {
        Some(logs) => {
            event!(
                Event::RequestLogEnd,
                &format!("{} {} {}", &result.name, &result.date, &logs.len())
            );
            callback().on_fetch_success(
                &result.name,
                &result.date,
                &logs
                    .iter()
                    .map(|l| l.to_str().unwrap_or(""))
                    .collect::<Vec<&str>>(),
            );
        }
        None => {
            if let Some(err) = result.error {
                event!(Event::RequestLogError, &err);
                callback().on_fetch_fail(&result.name, &result.date, &err)
            }
        }
    }
}

pub(crate) fn callback() -> &'static dyn EZLogCallback {
    if CALLBACK_INIT.is_completed() {
        unsafe { GLOBAL_CALLBACK }
    } else {
        static NOP: NopCallback = NopCallback;
        &NOP
    }
}

/// Async callback for fetch log files
///
/// [`set_boxed_callback`] sets the boxed callback.
///
/// # Examples
/// ```no_run
/// # use ezlog::{EZLogCallback};
///
/// struct SimpleCallback;
///
/// impl EZLogCallback for SimpleCallback {
///    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]) {
///        print!("{} {} {}", name, date, logs.join(" "));
///    }
///    fn on_fetch_fail(&self, name: &str, date: &str, err: &str) {
///        print!("{} {} {}", name, date, err);
///    }
///}
/// fn main() {
///     ezlog::set_boxed_callback(Box::new(SimpleCallback));
/// }
/// ```
pub trait EZLogCallback {
    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]);
    fn on_fetch_fail(&self, name: &str, date: &str, err: &str);
}

/// Set the boxed [EZLogCallback]
pub fn set_boxed_callback(callback: Box<dyn EZLogCallback>) {
    set_callback_inner(|| Box::leak(callback))
}

fn set_callback_inner<F>(make_callback: F)
where
    F: FnOnce() -> &'static dyn EZLogCallback,
{
    CALLBACK_INIT.call_once(|| unsafe {
        GLOBAL_CALLBACK = make_callback();
    });
}

struct NopCallback;
impl EZLogCallback for NopCallback {
    fn on_fetch_success(&self, _name: &str, _date: &str, _logs: &[&str]) {}
    fn on_fetch_fail(&self, _name: &str, _date: &str, _err: &str) {}
}

type SuccessCallback = Box<dyn Fn(&str, &str, &[&str])>;
type FailCallback = Box<dyn Fn(&str, &str, &str)>;

#[allow(dead_code)]
fn set_callback_fn(success: SuccessCallback, fail: FailCallback) {
    set_boxed_callback(Box::new(EZLogCallbackFn { success, fail }))
}

struct EZLogCallbackFn {
    success: SuccessCallback,
    fail: FailCallback,
}

impl EZLogCallback for EZLogCallbackFn {
    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]) {
        (self.success)(name, date, logs)
    }

    fn on_fetch_fail(&self, name: &str, date: &str, err: &str) {
        (self.fail)(name, date, err)
    }
}

pub enum EZMsg {
    CreateLogger(EZLogConfig),
    Record(EZRecord),
    ForceFlush(String),
    FlushAll(),
    Trim(),
    FetchLog(FetchReq),
    Action(Box<dyn Fn() + Send>),
}

impl fmt::Debug for EZMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EZMsg::CreateLogger(cfg) => f.debug_tuple("CreateLogger").field(cfg).finish(),
            EZMsg::Record(rec) => f.debug_tuple("Record").field(rec).finish(),
            EZMsg::ForceFlush(name) => f.debug_tuple("ForceFlush").field(name).finish(),
            EZMsg::FlushAll() => f.write_str("FlushAll"),
            EZMsg::Trim() => f.write_str("Trim"),
            EZMsg::FetchLog(req) => f.debug_tuple("FetchLog").field(req).finish(),
            EZMsg::Action(_) => f.write_str("Callback(<dyn Fn>)"),
        }
    }
}

/// Fetch Logs file‘s path reqeust
///
/// name: log name
/// date: log date
/// task_sender: channel sender for fetch result
#[derive(Debug, Clone)]
pub struct FetchReq {
    name: String,
    start: OffsetDateTime,
    end: OffsetDateTime,
}

/// # Fetch Logs file‘s path result.
///
/// if error is None, mean fetch process is ok.
/// logs maybe None if no logs write at the date.
#[derive(Debug)]
pub struct FetchResult {
    /// logger's name
    name: String,
    /// request date in string, like "2020_01_01"
    date: String,
    /// logs file's path
    logs: Option<Vec<PathBuf>>,
    /// error message
    error: Option<String>,
}

/// The Logger struct to implement the Log encode.
/// Compress function abstract
pub trait Compression {
    fn compress(&self, data: &[u8]) -> std::io::Result<Vec<u8>>;
}

/// Decompress function abstract
pub trait Decompression {
    fn decompress(&self, data: &[u8]) -> std::io::Result<Vec<u8>>;
}

/// The Compression trait + Decompression trait
pub trait Compress: Compression + Decompression {}

impl<T: Compression + Decompression> Compress for T {}

type NonceGenFn = Box<dyn Fn(&[u8]) -> Vec<u8>>;

/// Encrypt function abstract
pub trait Encryptor {
    fn encrypt(&self, data: &[u8], op: NonceGenFn) -> std::result::Result<Vec<u8>, LogError>;
}

/// decrypt function abstract
pub trait Decryptor {
    fn decrypt(&self, data: &[u8], op: NonceGenFn) -> std::result::Result<Vec<u8>, LogError>;
}

impl<T: Encryptor + Decryptor> Cryptor for T {}

/// The Encryptor trait + Decryptor trait
pub trait Cryptor: Encryptor + Decryptor {}

pub trait Formatter: Sync + Send {
    fn format(&self, record: &EZRecord) -> Result<Vec<u8>>;
}

struct DefaultFormatter;

impl Formatter for DefaultFormatter {
    fn format(&self, record: &EZRecord) -> Result<Vec<u8>> {
        let time = record
            .time()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());

        let mut vec = Vec::<u8>::new();
        vec.write_all(b"[")?;
        vec.write_all(time.as_bytes())?;
        vec.write_all(b" ")?;
        vec.write_all(record.level().as_str().as_bytes())?;
        vec.write_all(b" ")?;
        vec.write_all(record.target().as_bytes())?;
        vec.write_all(b" ")?;
        vec.write_all(record.thread_name().as_bytes())?;
        vec.write_all(b":")?;
        vec.write_all(record.thread_id().to_string().as_bytes())?;
        if let Some(file) = record.file() {
            vec.write_all(b" ")?;
            vec.write_all(file.as_bytes())?;
            vec.write_all(b":")?;
            vec.write_all(record.line().unwrap_or_default().to_string().as_bytes())?;
        }
        vec.write_all("] ".as_bytes())?;
        vec.write_all(record.content().as_bytes())?;
        Ok(vec)
    }
}

pub(crate) fn formatter() -> &'static dyn Formatter {
    static DEFAULT_FORMATTER: DefaultFormatter = DefaultFormatter;

    if let Some(mutex) = GLOBAL_FORMATTER.get() {
        if let Ok(guard) = mutex.lock() {
            if let Some(formatter) = *guard {
                return formatter;
            }
        }
    }
    &DEFAULT_FORMATTER
}

fn set_formatter<F>(make_formatter: F)
where
    F: FnOnce() -> &'static dyn Formatter,
{
    GLOBAL_FORMATTER.get_or_init(|| Mutex::new(Some(make_formatter())));
}

pub fn set_boxed_formatter(formatter: Box<dyn Formatter>) {
    set_formatter(|| Box::leak(formatter))
}

/// Log version enum
///
/// current version: v1
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
pub enum Version {
    NONE,
    V1,
    V2,
    UNKNOWN,
}

impl From<u8> for Version {
    fn from(v: u8) -> Self {
        match v {
            1 => Version::V1,
            2 => Version::V2,
            0 => Version::NONE,
            _ => Version::UNKNOWN,
        }
    }
}

impl From<Version> for u8 {
    fn from(v: Version) -> Self {
        match v {
            Version::V1 => 1,
            Version::V2 => 2,
            Version::UNKNOWN => u8::MAX,
            Version::NONE => 0,
        }
    }
}

#[cfg(feature = "json")]
use serde::Deserializer;
#[cfg(feature = "json")]
use serde::Serializer;

#[cfg(feature = "json")]
pub fn serialize_time<S>(
    date: &OffsetDateTime,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use time::format_description::well_known::Rfc3339;
    serializer.serialize_str(&date.format(&Rfc3339).unwrap_or("".to_string()))
}

#[cfg(feature = "json")]
pub fn deserialize_time<'de, D>(deserializer: D) -> std::result::Result<OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::Deserialize;
    use time::format_description::well_known::Rfc3339;

    let s = String::deserialize(deserializer)?;
    OffsetDateTime::parse(&s, &Rfc3339).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use std::io::{
        Read,
        Write,
    };

    #[cfg(feature = "decode")]
    use aead::{
        Aead,
        KeyInit,
    };
    use flate2::{
        bufread::ZlibDecoder,
        write::ZlibEncoder,
        Compression,
    };
    use time::OffsetDateTime;

    use crate::recorder::EZRecordBuilder;
    use crate::Header;
    use crate::{
        config::EZLogConfigBuilder,
        EZLogConfig,
        RECORD_SIGNATURE_END,
        RECORD_SIGNATURE_START,
    };

    fn create_config() -> EZLogConfig {
        EZLogConfig::default()
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
        assert_eq!(v.len(), header.length());
    }

    #[test]
    fn test_compress() {
        let plaint_text = b"dsafafafaasdlfkaldfjiiwoeuriowiiwueroiwur\n";

        let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
        e.write_all(plaint_text).unwrap();
        let compressed = e.finish().unwrap();

        let mut d = ZlibDecoder::new(compressed.as_slice());

        let mut s = Vec::new();
        d.read_to_end(&mut s).unwrap();
        assert_eq!(s, plaint_text);
    }

    /// https://docs.rs/aes-gcm/latest/aes_gcm/
    #[test]
    #[cfg(feature = "decode")]
    fn test_cipher() {
        use aes_gcm::Aes256Gcm;
        use aes_gcm::Nonce;

        let cipher = Aes256Gcm::new_from_slice(b"an example very very secret key.").unwrap();

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
    fn test_record_truncks() {
        let config = EZLogConfigBuilder::new().max_size(6).build();
        let record = EZRecordBuilder::new().content("深圳".into()).build();
        let trunks = record.trunks(&config);
        assert_eq!(trunks.len(), 2);
        assert_eq!(trunks[0].content(), "深");
        assert_eq!(trunks[1].content(), "圳");
    }

    #[test]
    fn test_request_logs() {
        let mut cache_dir = test_compat::test_path();
        cache_dir.push("test");
        std::fs::create_dir_all(&cache_dir).unwrap();
        let dir_clone = cache_dir.clone();
        crate::InitBuilder::new().debug(true).init();
        let config = EZLogConfigBuilder::new()
            .dir_path(cache_dir.into_os_string().into_string().unwrap())
            .name("test".to_owned())
            .build();
        crate::create_log(config);

        crate::log(
            EZRecordBuilder::new()
                .log_name("test".to_owned())
                .content("test log".to_string())
                .build(),
        );
        let (tx, tv) = crossbeam_channel::bounded::<usize>(1);
        let success_call: Box<dyn Fn(&str, &str, &[&str])> =
            Box::new(move |_name, _datee, logs| {
                tx.send(logs.len()).unwrap();
            });
        crate::set_callback_fn(success_call, Box::new(|_name, _date, _err| {}));
        crate::request_log_files_for_date(
            "test",
            OffsetDateTime::now_utc(),
            OffsetDateTime::now_utc(),
        );
        let count = tv.recv().unwrap();
        std::fs::remove_dir_all(dir_clone).unwrap();
        assert!(count == 1)
    }
}
