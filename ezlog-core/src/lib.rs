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

pub use self::config::EZLogConfig;
pub use self::config::EZLogConfigBuilder;
pub use self::config::Level;
pub use self::errors::LogError;
pub use self::events::Event;
pub use self::events::EventListener;
pub use self::events::EventPrinter;
pub use self::init::InitBuilder;
pub use self::init::MsgHandler;
pub use self::logger::EZLogger;
pub use self::logger::Header;
pub use self::recorder::EZRecord;
pub use self::recorder::EZRecordBuilder;

pub(crate) use self::events::event;

use crossbeam_channel::{Sender, TrySendError};
use memmap2::MmapMut;
use time::Duration;
use time::OffsetDateTime;

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::{
    collections::HashMap,
    hash::Hash,
    io::{self, Cursor, Read, Write},
    mem::MaybeUninit,
    sync::Once,
    thread,
};

#[cfg(feature = "backtrace")]
use backtrace::Backtrace;

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

static mut LOG_SERVICE: MaybeUninit<LogService> = MaybeUninit::uninit();
static LOG_SERVICE_INIT: Once = Once::new();

static mut GLOBAL_CALLBACK: &dyn EZLogCallback = &NopCallback;
static CALLBACK_INIT: Once = Once::new();

static mut FORMATTER: &dyn Formatter = &DefaultFormatter;
static mut FORMATTER_INIT: Once = Once::new();

type Result<T> = std::result::Result<T, LogError>;

#[inline]
fn get_map() -> Result<&'static mut HashMap<String, EZLogger>> {
    if !LOG_SERVICE_INIT.is_completed() {
        return Err(LogError::NotInit);
    }
    unsafe { Ok(&mut LOG_SERVICE.assume_init_mut().loggers) }
}

#[inline]
fn get_sender() -> Result<&'static Sender<EZMsg>> {
    if !LOG_SERVICE_INIT.is_completed() {
        return Err(LogError::NotInit);
    }
    unsafe { Ok(&LOG_SERVICE.assume_init_mut().log_sender) }
}

#[inline]
fn get_fetch_sender() -> Result<&'static Sender<FetchResult>> {
    if !LOG_SERVICE_INIT.is_completed() {
        return Err(LogError::NotInit);
    }
    unsafe { Ok(&LOG_SERVICE.assume_init_mut().fetch_sender) }
}

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
    layers: Arc<Vec<Box<dyn MsgHandler>>>,
    loggers: HashMap<String, EZLogger>,
    log_sender: Sender<EZMsg>,
    fetch_sender: Sender<FetchResult>,
}

impl LogService {
    fn new() -> Self {
        LogService {
            layers: Arc::new(Vec::new()),
            loggers: HashMap::new(),
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
    let (sender, receiver) = crossbeam_channel::unbounded::<EZMsg>();
    match thread::Builder::new()
        .name("ezlog_task".to_string())
        .spawn(move || loop {
            match receiver.recv() {
                Ok(msg) => match msg {
                    EZMsg::CreateLogger(config) => {
                        let name = config.name.clone();
                        match EZLogger::new(config) {
                            Ok(log) => {
                                if let Ok(map) = get_map() {
                                    map.insert(log.config.name.clone(), log);
                                    event!(Event::CreateLoggerEnd, &name);
                                }
                            }
                            Err(e) => {
                                event!(Event::CreateLoggerError, &name, &e);
                            }
                        };
                    }
                    EZMsg::Record(record) => {
                        let map = match get_map() {
                            Ok(map) => map,
                            Err(e) => {
                                service_not_init_op(e);
                                continue;
                            }
                        };

                        let log = match map.get_mut(&record.log_name().to_owned()) {
                            Some(l) => l,
                            None => {
                                event!(
                                    Event::RecordError,
                                    &record.t_id(),
                                    &LogError::Illegal("no logger found".into())
                                );
                                continue;
                            }
                        };
                        if log.config.level < record.level() {
                            event!(
                                Event::RecordFilterOut,
                                &format!(
                                    "current level {}, max level {}",
                                    &record.level(),
                                    &log.config.level
                                )
                            );
                            continue;
                        }
                        match log.append(&record) {
                            Ok(_) => {
                                event!(Event::RecordEnd, &record.t_id());
                            }
                            Err(err) => match err {
                                LogError::Compress(err) => {
                                    event!(Event::CompressError, &record.t_id(), &err.into());
                                }
                                LogError::Crypto(err) => {
                                    event!(
                                        Event::EncryptError,
                                        &record.t_id(),
                                        &LogError::Crypto(err)
                                    )
                                }
                                _ => {
                                    event!(Event::RecordError, &record.t_id(), &err)
                                }
                            },
                        }
                    }
                    EZMsg::ForceFlush(name) => {
                        get_map().map_or_else(service_not_init_op, |map| {
                            if let Some(log) = map.get_mut(&name) {
                                log.appender.flush().ok();
                                event!(Event::FlushEnd, &name);
                            } else {
                                event!(
                                    Event::FlushError,
                                    &name,
                                    &LogError::Illegal("no logger found".into())
                                );
                            }
                        });
                    }
                    EZMsg::FlushAll() => {
                        get_map().map_or_else(service_not_init_op, |map| {
                            map.values_mut().for_each(|item| {
                                item.flush().ok();
                            })
                        });
                        event!(Event::FlushEnd);
                    }
                    EZMsg::Trim() => {
                        get_map().map_or_else(service_not_init_op, |map| {
                            map.values().for_each(|logger| logger.trim())
                        });
                        event!(Event::TrimEnd)
                    }
                    EZMsg::FetchLog(task) => {
                        let map = match get_map() {
                            Ok(map) => map,
                            Err(e) => {
                                service_not_init_op(e);
                                continue;
                            }
                        };
                        let logger = match map.get_mut(&task.name) {
                            Some(l) => l,
                            None => {
                                event!(
                                    Event::RequestLog,
                                    "fetchLog",
                                    &LogError::Illegal(format!("no logger found {}", task.name))
                                );
                                continue;
                            }
                        };

                        let now = OffsetDateTime::now_utc();
                        if (now < task.end || now < task.start + Duration::days(1))
                            && now > task.start
                        {
                            logger
                                .rotate_if_not_empty()
                                .map_err(|e| {
                                    event!(Event::RotateFileError, "logger rorate error ", &e)
                                })
                                .ok();
                        }

                        let mut logs: Vec<PathBuf> = Vec::new();

                        let days = (task.end - task.start).whole_days();
                        for day in 0..=days {
                            let mut query =
                                logger.query_log_files_for_date(task.start + Duration::days(day));
                            logs.append(&mut query);
                        }

                        task.task_sender
                            .try_send(FetchResult {
                                name: task.name,
                                date: task.start.date().to_string(),
                                logs: Some(logs),
                                error: None,
                            })
                            .unwrap_or_else(ffi_err_handle);
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
                    event!(Event::RequestLogEnd)
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
    let sender = match get_fetch_sender() {
        Ok(sender) => sender,
        Err(e) => {
            service_not_init_op(e);
            return;
        }
    };
    let msg = FetchReq {
        name: log_name.to_string(),
        start,
        end,
        task_sender: sender.clone(),
    };

    get_sender().map_or_else(service_not_init_op, |sender| {
        sender
            .try_send(EZMsg::FetchLog(msg))
            .unwrap_or_else(report_channel_send_err);
    });
}

#[inline]
fn post_msg(msg: EZMsg) {
    if LOG_SERVICE_INIT.is_completed() {
        unsafe { LOG_SERVICE.assume_init_mut().dispatch(msg) }
    } else {
        service_not_init_op(LogError::NotInit)
    }
}

#[inline]
pub(crate) fn report_channel_send_err<T>(err: TrySendError<T>) {
    event!(Event::ChannelError, "channel send err", &err.into());
}

#[inline]
pub(crate) fn ffi_err_handle<T>(err: T)
where
    T: Error,
{
    let e = LogError::unknown(&format!("{:?}", err));
    event!(Event::FFiError, "ffi error handle", &e);
}

#[inline]
pub(crate) fn service_not_init_op(e: LogError) {
    event!(Event::InitError, "log service not init ", &e);
}

fn invoke_fetch_callback(result: FetchResult) {
    match result.logs {
        Some(logs) => {
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

#[derive(Debug, Clone)]
pub enum EZMsg {
    CreateLogger(EZLogConfig),
    Record(EZRecord),
    ForceFlush(String),
    FlushAll(),
    Trim(),
    FetchLog(FetchReq),
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
    task_sender: Sender<FetchResult>,
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

pub trait Formatter {
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
        vec.write_all(&[b'['])?;
        vec.write_all(time.as_bytes())?;
        vec.write_all(&[b' '])?;
        vec.write_all(record.level().as_str().as_bytes())?;
        vec.write_all(&[b' '])?;
        vec.write_all(record.target().as_bytes())?;
        vec.write_all(&[b' '])?;
        vec.write_all(record.thread_name().as_bytes())?;
        vec.write_all(&[b':'])?;
        vec.write_all(record.thread_id().to_string().as_bytes())?;
        if let Some(file) = record.file() {
            vec.write_all(&[b' '])?;
            vec.write_all(file.as_bytes())?;
            vec.write_all(&[b':'])?;
            vec.write_all(record.line().unwrap_or_default().to_string().as_bytes())?;
        }
        vec.write_all("] ".as_bytes())?;
        vec.write_all(record.content().as_bytes())?;
        Ok(vec)
    }
}

pub(crate) fn formatter() -> &'static dyn Formatter {
    unsafe {
        if FORMATTER_INIT.is_completed() {
            FORMATTER
        } else {
            static DEFAULT_FORMATTER: DefaultFormatter = DefaultFormatter;
            &DEFAULT_FORMATTER
        }
    }
}

pub fn format(record: &EZRecord) -> String {
    String::from_utf8_lossy(&formatter().format(record).unwrap_or_default()).to_string()
}

fn set_formatter<F>(make_formatter: F)
where
    F: FnOnce() -> &'static dyn Formatter,
{
    unsafe {
        FORMATTER_INIT.call_once(|| {
            FORMATTER = make_formatter();
        })
    };
}

pub fn set_boxed_formatter(formatter: Box<dyn Formatter>) {
    set_formatter(|| Box::leak(formatter))
}

/// Log version enum
///
/// current version: v1
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum Version {
    V1,
    V2,
    UNKNOWN,
}

impl From<u8> for Version {
    fn from(v: u8) -> Self {
        match v {
            1 => Version::V1,
            2 => Version::V2,
            _ => Version::UNKNOWN,
        }
    }
}

impl From<Version> for u8 {
    fn from(v: Version) -> Self {
        match v {
            Version::V1 => 1,
            Version::V2 => 2,
            Version::UNKNOWN => 0,
        }
    }
}

/// Cipher kind current support
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum CipherKind {
    #[deprecated(since = "0.2.0", note = "Use AES128GCMSIV instead")]
    AES128GCM,
    #[deprecated(since = "0.2.0", note = "Use AES256GCMSIV instead")]
    AES256GCM,
    AES128GCMSIV,
    AES256GCMSIV,
    NONE,
    UNKNOWN,
}

#[allow(deprecated)]
impl From<u8> for CipherKind {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => CipherKind::NONE,
            0x01 => CipherKind::AES128GCM,
            0x02 => CipherKind::AES256GCM,
            0x03 => CipherKind::AES128GCMSIV,
            0x04 => CipherKind::AES256GCMSIV,
            _ => CipherKind::UNKNOWN,
        }
    }
}

#[allow(deprecated)]
impl From<CipherKind> for u8 {
    fn from(orig: CipherKind) -> Self {
        match orig {
            CipherKind::NONE => 0x00,
            CipherKind::AES128GCM => 0x01,
            CipherKind::AES256GCM => 0x02,
            CipherKind::AES128GCMSIV => 0x03,
            CipherKind::AES256GCMSIV => 0x04,
            CipherKind::UNKNOWN => 0xff,
        }
    }
}

#[allow(deprecated)]
impl core::fmt::Display for CipherKind {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            CipherKind::AES128GCM => write!(f, "AEAD_AES_128_GCM"),
            CipherKind::AES256GCM => write!(f, "AEAD_AES_256_GCM"),
            CipherKind::AES128GCMSIV => write!(f, "AEAD_AES_128_GCM_SIV"),
            CipherKind::AES256GCMSIV => write!(f, "AEAD_AES_128_GCM_SIV"),
            CipherKind::NONE => write!(f, "NONE"),
            _ => write!(f, "UNKNOWN"),
        }
    }
}

#[allow(deprecated)]
impl std::str::FromStr for CipherKind {
    type Err = LogError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "AEAD_AES_128_GCM" => Ok(CipherKind::AES128GCM),
            "AEAD_AES_256_GCM" => Ok(CipherKind::AES256GCM),
            "AEAD_AES_128_GCM_SIV" => Ok(CipherKind::AES128GCMSIV),
            "AEAD_AES_256_GCM_SIV" => Ok(CipherKind::AES256GCMSIV),
            "NONE" => Ok(CipherKind::NONE),
            _ => Err(errors::LogError::Parse("unknown cipher kind".to_string())),
        }
    }
}

/// Compress type can be used to compress the log file.
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum CompressKind {
    /// ZLIB compression
    /// we use [flate2](https://github.com/rust-lang/flate2-rs) to implement this
    ZLIB,
    /// No compression
    NONE,
    /// Unknown compression
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

/// Compress level
///
/// can be define as one of the following: FAST, DEFAULT, BEST
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

#[cfg(feature = "backtrace")]
fn hook_panic() {
    std::panic::set_hook(Box::new(|p| {
        let bt = Backtrace::new();
        event!(Event::Panic, &format!("ezlog: \n {p:?} \n{bt:?} \n"));
    }));
}

#[cfg(not(feature = "backtrace"))]
fn hook_panic() {
    std::panic::set_hook(Box::new(|p| {
        event!(Event::Panic, &format!("ezlog: \n {p:?}"));
    }));
}

#[cfg(test)]
mod tests {
    use crate::recorder::EZRecordBuilder;
    use flate2::{bufread::ZlibDecoder, write::ZlibEncoder, Compression};
    use std::io::{Read, Write};
    use time::OffsetDateTime;

    use crate::Header;
    use crate::{
        config::EZLogConfigBuilder, EZLogConfig, RECORD_SIGNATURE_END, RECORD_SIGNATURE_START,
    };

    #[cfg(feature = "decode")]
    use aead::{Aead, KeyInit};

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
        let mut cache_dir = dirs::cache_dir().unwrap();
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
