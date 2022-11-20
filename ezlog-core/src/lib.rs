#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![doc = include_str!("../README.md")]

mod appender;
mod compress;
mod config;
mod crypto;
mod errors;
mod events;
mod thread_name;

#[cfg(target_os = "android")]
#[allow(non_snake_case)]
mod android;
#[cfg(any(target_os = "macos", target_os = "ios"))]
#[allow(non_snake_case)]
mod ios;

pub use self::config::EZLogConfig;
pub use self::config::EZLogConfigBuilder;
pub use self::events::Event;
pub use self::events::EventPrinter;

use appender::EZAppender;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use compress::ZlibCodec;
use crossbeam_channel::{Sender, TrySendError};
use crypto::{Aes128Gcm, Aes256Gcm};
use errors::LogError;
use memmap2::MmapMut;
use once_cell::sync::OnceCell;

use std::error::Error;
use std::path::PathBuf;
use std::{
    cmp,
    collections::{hash_map::DefaultHasher, HashMap},
    fmt, fs,
    hash::{Hash, Hasher},
    io::{self, Cursor, Read, Write},
    mem::MaybeUninit,
    ptr,
    rc::Rc,
    sync::Once,
    thread,
};
use time::format_description::well_known::Rfc3339;
use time::Date;
use time::{Duration, OffsetDateTime};

#[cfg(feature = "backtrace")]
use backtrace::Backtrace;
#[cfg(feature = "decode")]
use io::BufRead;
#[cfg(feature = "log")]
use log::Record;

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

// maybe set as threadlocal variable
static mut LOG_MAP: MaybeUninit<HashMap<String, EZLogger>> = MaybeUninit::uninit();
static LOG_MAP_INIT: Once = Once::new();

static mut GLOBAL_CALLBACK: &dyn EZLogCallback = &NopCallback;
static CALLBACK_INIT: Once = Once::new();

type Result<T> = std::result::Result<T, LogError>;

#[inline]
fn get_map() -> &'static mut HashMap<String, EZLogger> {
    LOG_MAP_INIT.call_once(|| unsafe {
        ptr::write(LOG_MAP.as_mut_ptr(), HashMap::new());
    });
    unsafe { &mut (*LOG_MAP.as_mut_ptr()) }
}

#[inline]
fn get_sender() -> &'static Sender<EZMsg> {
    static SENDER: OnceCell<Sender<EZMsg>> = OnceCell::new();
    SENDER.get_or_init(init_log_channel)
}

#[inline]
fn get_fetch_sender() -> &'static Sender<FetchResult> {
    static FETCH_SENDER: OnceCell<Sender<FetchResult>> = OnceCell::new();
    FETCH_SENDER.get_or_init(init_callback_channel)
}

/// Init ezlog
///
/// init ezlog, setup panic hook, trigger event when panic.
///
/// # Examples
/// ```
/// use ezlog::init;
/// init();
/// ```
pub fn init() {
    hook_panic();
}

pub fn init_with_event(event: &'static dyn Event) {
    set_event_listener(event);
    init();
}

/// Trim all [EZLogger]s outdated files
///
/// manual trim the log files in disk. delete logs which are out of date.
pub fn trim() {
    post_msg(EZMsg::Trim());
}

/// Set global [Event] listener
pub fn set_event_listener(event: &'static dyn Event) {
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
                                let map = get_map();
                                map.insert(log.config.name.clone(), log);
                                event!(create_logger_end & name);
                            }
                            Err(e) => {
                                event!(create_logger_fail & name, &e.to_string());
                            }
                        };
                    }
                    EZMsg::Record(record) => {
                        let log = match get_map().get_mut(&record.log_name) {
                            Some(l) => l,
                            None => {
                                event!(unknown_err & record.t_id(), "logger not found");
                                continue;
                            }
                        };
                        if log.config.level < record.level {
                            event!(
                                record_filter_out & record.t_id(),
                                &format!(
                                    "current level{}, max level{}",
                                    &record.level, &log.config.level
                                )
                            );
                            continue;
                        }
                        match log.append(&record) {
                            Ok(_) => {
                                event!(record_end & record.t_id());
                            }
                            Err(err) => match err {
                                LogError::Compress(err) => {
                                    event!(compress_fail & record.t_id(), &err.to_string());
                                }
                                LogError::Crypto(err) => {
                                    event!(encrypt_fail & record.t_id(), &err.to_string())
                                }
                                _ => {
                                    event!(unknown_err & record.t_id(), &err.to_string())
                                }
                            },
                        }
                    }
                    EZMsg::ForceFlush(name) => {
                        let log = match get_map().get_mut(&name) {
                            Some(l) => l,
                            None => {
                                event!(internal_err & name);
                                continue;
                            }
                        };
                        log.appender.flush().ok();
                        event!(flush_end & name);
                    }
                    EZMsg::FlushAll() => {
                        get_map().values_mut().for_each(|item| {
                            item.flush().ok();
                        });
                        event!(flush_all_end);
                    }
                    EZMsg::Trim() => {
                        get_map().values().for_each(|logger| logger.trim());
                    }
                    EZMsg::FetchLog(task) => {
                        let logger = match get_map().get_mut(&task.name) {
                            Some(l) => l,
                            None => {
                                event!(
                                    ffi_call_err
                                        & format!("logger not found on fetch logs {}", task.name)
                                );
                                continue;
                            }
                        };
                        match config::parse_date_from_str(
                            &task.date,
                            "date format error in get_log_files_for_date",
                        ) {
                            Ok(date) => {
                                let logs = logger.query_log_files_for_date(date);
                                task.task_sender
                                    .try_send(FetchResult {
                                        name: task.name,
                                        date: task.date,
                                        logs: Some(logs),
                                        error: None,
                                    })
                                    .unwrap_or_else(ffi_err_handle);
                            }
                            Err(e) => {
                                task.task_sender
                                    .try_send(FetchResult {
                                        name: task.name,
                                        date: task.date,
                                        logs: None,
                                        error: Some(e.to_string()),
                                    })
                                    .unwrap_or_else(ffi_err_handle);
                            }
                        }
                    }
                },
                Err(err) => {
                    event!(internal_err & err.to_string());
                }
            }
        }) {
        Ok(_) => {
            event!(init "init ezlog success");
        }
        Err(e) => {
            event!(init & format!("init ezlog error {}", e));
        }
    }
    sender
}

fn init_callback_channel() -> Sender<FetchResult> {
    let (fetch_sender, fetch_receiver) = crossbeam_channel::unbounded::<FetchResult>();
    match thread::Builder::new()
        .name("ezlog_callback".to_string())
        .spawn(move || match fetch_receiver.recv() {
            Ok(result) => {
                invoke_fetch_callback(result);
            }
            Err(e) => event!(ffi_call_err & e.to_string()),
        }) {
        Ok(_) => {
            event!(init "init callback success");
        }
        Err(e) => {
            event!(init & format!("init callback err {}", e));
        }
    }
    fetch_sender
}

/// Create a new [EZLogger] from an [EZLogConfig]
pub fn create_log(config: EZLogConfig) {
    let name = config.name.clone();
    let msg = EZMsg::CreateLogger(config);
    if post_msg(msg) {
        event!(create_logger & name);
    }
}

/// Write a [EZRecord] to the log file
pub fn log(record: EZRecord) {
    let tid = record.t_id();
    let msg = EZMsg::Record(record);
    if post_msg(msg) {
        event!(record & tid);
    }
}

/// Force flush the log file
pub fn flush(log_name: &str) {
    let msg = EZMsg::ForceFlush(log_name.to_string());
    if post_msg(msg) {
        event!(flush log_name)
    }
}

/// Flush all log files
pub fn flush_all() {
    let msg = EZMsg::FlushAll();
    if post_msg(msg) {
        event!(flush_all)
    }
}

/// Request logs file path array at the date which [EZLogger]'s name is define in the parameter
pub fn request_log_files_for_date(log_name: &str, date_str: &str) {
    let msg = FetchReq {
        name: log_name.to_string(),
        date: date_str.to_string(),
        task_sender: get_fetch_sender().clone(),
    };

    get_sender()
        .try_send(EZMsg::FetchLog(msg))
        .unwrap_or_else(report_channel_send_err);
}

#[inline]
fn post_msg(msg: EZMsg) -> bool {
    get_sender()
        .try_send(msg)
        .map_err(report_channel_send_err)
        .is_ok()
}

#[inline]
fn report_channel_send_err<T>(err: TrySendError<T>) {
    event!(internal_err & err.to_string());
}

#[inline]
fn ffi_err_handle<T>(err: T)
where
    T: Error,
{
    event!(ffi_call_err & err.to_string());
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
                callback().on_fetch_fail(&result.name, &result.date, &err);
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

#[derive(Debug, Clone)]
pub(crate) enum EZMsg {
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
    date: String,
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
pub struct EZLogger {
    config: Rc<EZLogConfig>,
    appender: Box<dyn Write>,
    compression: Option<Box<dyn Compress>>,
    cryptor: Option<Box<dyn Cryptor>>,
}

impl EZLogger {
    pub fn new(config: EZLogConfig) -> Result<Self> {
        let rc_conf = Rc::new(config);
        let appender = Box::new(EZAppender::new(Rc::clone(&rc_conf))?);
        let compression = EZLogger::create_compress(&rc_conf);
        let cryptor = EZLogger::create_cryptor(&rc_conf)?;

        Ok(Self {
            config: Rc::clone(&rc_conf),
            appender,
            compression,
            cryptor,
        })
    }

    pub fn create_cryptor(config: &EZLogConfig) -> Result<Option<Box<dyn Cryptor>>> {
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

    pub fn create_compress(config: &EZLogConfig) -> Option<Box<dyn Compress>> {
        match config.compress {
            CompressKind::ZLIB => Some(Box::new(ZlibCodec::new(&config.compress_level))),
            CompressKind::NONE => None,
            CompressKind::UNKNOWN => None,
        }
    }

    fn append(&mut self, record: &EZRecord) -> Result<()> {
        let mut e: Option<LogError> = None;
        if record.content.len() > self.config.max_size as usize / 2 {
            record.trunks(&self.config).iter().for_each(|record| {
                match self.encode_as_block(record) {
                    Ok(buf) => match self.appender.write_all(&buf) {
                        Ok(_) => {}
                        Err(err) => e = Some(LogError::IoError(err)),
                    },
                    Err(err) => {
                        e = Some(err);
                    }
                }
            })
        } else {
            let buf = self.encode_as_block(record)?;
            self.appender.write_all(&buf)?;
        }
        if let Some(err) = e {
            Err(err)
        } else {
            Ok(())
        }
    }

    fn encode(&mut self, record: &EZRecord) -> Result<Vec<u8>> {
        let mut buf = self.format(record);
        if let Some(encryptor) = &self.cryptor {
            event!(encrypt_start & record.t_id());
            buf = encryptor.encrypt(&buf)?;
            event!(encrypt_end & record.t_id());
        }
        if let Some(compression) = &self.compression {
            event!(compress_start & record.t_id());
            buf = compression.compress(&buf).map_err(LogError::Compress)?;
            event!(compress_end & record.t_id());
        }
        Ok(buf)
    }

    ///
    pub fn encode_as_block(&mut self, record: &EZRecord) -> Result<Vec<u8>> {
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

    fn create_size_chunk(size: usize) -> Result<Vec<u8>> {
        let mut chunk: Vec<u8> = Vec::new();
        match size {
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

    #[cfg(feature = "decode")]
    pub fn decode_record(&mut self, reader: &mut dyn BufRead) -> Result<Vec<u8>> {
        EZLogger::decode_record_from_read(reader, &self.compression, &self.cryptor)
    }

    #[cfg(feature = "decode")]
    pub fn decode_body_and_write(
        reader: &mut dyn BufRead,
        writer: &mut dyn Write,
        compression: &Option<Box<dyn Compress>>,
        cryptor: &Option<Box<dyn Cryptor>>,
    ) -> io::Result<()> {
        loop {
            match EZLogger::decode_record_from_read(reader, compression, cryptor) {
                Ok(buf) => {
                    if buf.is_empty() {
                        break;
                    }
                    writer.write_all(&buf)?;
                }
                Err(e) => {
                    if let LogError::IoError(err) = e {
                        if err.kind() == io::ErrorKind::UnexpectedEof {
                            break;
                        }
                        println!("decode log error {err:?}");
                    } else {
                        println!("decode log error {e:?}");
                    }
                    continue;
                }
            }
        }
        writer.flush()
    }

    #[cfg(feature = "decode")]
    pub fn decode_record_from_read(
        reader: &mut dyn BufRead,
        compression: &Option<Box<dyn Compress>>,
        cryptor: &Option<Box<dyn Cryptor>>,
    ) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        let nums = reader.read_until(RECORD_SIGNATURE_START, &mut buf)?;
        if nums == 0 {
            return Ok(buf);
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
            return Err(LogError::Parse("record end sign error".to_string()));
        }
        EZLogger::decode_record_content(&chunk, compression, cryptor)
    }

    #[cfg(feature = "decode")]
    pub fn decode_record_content(
        chunk: &[u8],
        compression: &Option<Box<dyn Compress>>,
        cryptor: &Option<Box<dyn Cryptor>>,
    ) -> Result<Vec<u8>> {
        let mut buf = chunk.to_vec();

        if let Some(decompression) = compression {
            buf = decompression.decompress(&buf)?;
        }

        if let Some(decryptor) = cryptor {
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

    fn flush(&mut self) -> std::result::Result<(), io::Error> {
        self.appender.flush()
    }

    fn trim(&self) {
        match fs::read_dir(&self.config.dir_path) {
            Ok(dir) => {
                for file in dir {
                    match file {
                        Ok(file) => {
                            if let Some(name) = file.file_name().to_str() {
                                match self.config.is_file_out_of_date(name) {
                                    Ok(out_of_date) => {
                                        if out_of_date {
                                            fs::remove_file(file.path()).unwrap_or_else(|e| {
                                                event!(
                                                    trim_logger_err
                                                        & format!("trim: remove file err: {}", e)
                                                )
                                            });
                                        }
                                    }
                                    Err(e) => {
                                        event!(
                                            trim_logger_err
                                                & format!(
                                                    "trim: judge file out of date error: {}",
                                                    e
                                                )
                                        )
                                    }
                                }
                            };
                        }
                        Err(e) => {
                            event!(trim_logger_err & format!("trim: traversal file error: {}", e))
                        }
                    }
                }
            }
            Err(e) => event!(trim_logger_err & format!("trim: read dir error: {}", e)),
        }
    }

    pub fn query_log_files_for_date(&self, date: Date) -> Vec<PathBuf> {
        self.config.query_log_files_for_date(date)
    }
}

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

/// Encrypt function abstract
pub trait Encryptor {
    fn encrypt(&self, data: &[u8]) -> std::result::Result<Vec<u8>, LogError>;
}

/// decrypt function abstract
pub trait Decryptor {
    fn decrypt(&self, data: &[u8]) -> std::result::Result<Vec<u8>, LogError>;
}

impl<T: Encryptor + Decryptor> Cryptor for T {}

/// The Encryptor trait + Decryptor trait
pub trait Cryptor: Encryptor + Decryptor {}

/// Log version enum
///
/// current version: v1
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

/// Cipher kind current support
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

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "AES_128_GCM" => Ok(CipherKind::AES128GCM),
            "AES_256_GCM" => Ok(CipherKind::AES256GCM),
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

/// EZLog file Header
///
/// every log file starts with a header,
/// which is used to describe the version, log length, compress type, cipher kind and so on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Header {
    /// version code
    version: Version,
    /// unused flag
    flag: u8,
    /// current log file write position
    recorder_position: u32,
    /// compress type
    compress: CompressKind,
    /// cipher kind
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
            recorder_position: Header::fixed_size() as u32,
            compress: CompressKind::ZLIB,
            cipher: CipherKind::AES128GCM,
        }
    }

    pub fn create(config: &EZLogConfig) -> Self {
        Header {
            version: config.version,
            flag: 0,
            recorder_position: Header::fixed_size() as u32,
            compress: config.compress,
            cipher: config.cipher,
        }
    }

    pub fn fixed_size() -> usize {
        V1_LOG_HEADER_SIZE
    }

    pub fn encode(&self, writer: &mut dyn Write) -> std::result::Result<(), io::Error> {
        writer.write_all(crate::FILE_SIGNATURE)?;
        writer.write_u8(self.version.into())?;
        writer.write_u8(self.flag)?;
        writer.write_u32::<BigEndian>(self.recorder_position)?;
        writer.write_u8(self.compress.into())?;
        writer.write_u8(self.cipher.into())
    }

    pub fn decode(reader: &mut dyn Read) -> std::result::Result<Self, errors::LogError> {
        let mut signature = [0u8; 2];
        reader.read_exact(&mut signature)?;
        let version = reader.read_u8()?;
        let flag = reader.read_u8()?;
        let mut recorder_size = reader.read_u32::<BigEndian>()?;
        if recorder_size < Header::fixed_size() as u32 {
            recorder_size = Header::fixed_size() as u32;
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
        self.version == Version::UNKNOWN && self.recorder_position <= Header::fixed_size() as u32
    }
}

/// Single Log record
#[derive(Debug, Clone)]
pub struct EZRecord {
    id: u64,
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
                id: self.id,
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

    #[inline]
    pub fn to_trunk_builder(&self) -> EZRecordBuilder {
        EZRecordBuilder {
            record: EZRecord {
                id: 0,
                log_name: self.log_name.clone(),
                level: self.level,
                target: self.target.clone(),
                time: self.time,
                thread_id: self.thread_id,
                thread_name: self.thread_name.clone(),
                content: "".into(),
            },
        }
    }

    #[cfg(feature = "log")]
    pub fn from(r: &Record) -> Self {
        let t = thread::current();
        let t_id = thread_id::get();
        let t_name = t.name().unwrap_or_default();
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

    pub fn t_id(&self) -> String {
        format!("{}_{}", self.log_name, &self.id)
    }

    /// get EZRecord unique id
    pub fn id(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.content.hash(&mut hasher);
        self.time.hash(&mut hasher);
        hasher.finish()
    }

    pub fn trunks(&self, config: &EZLogConfig) -> Vec<EZRecord> {
        let mut trunks: Vec<EZRecord> = Vec::new();
        let mut split_content: Vec<char> = Vec::new();
        let mut size = 0;
        let chars = self.content.chars();
        chars.for_each(|c| {
            size += c.len_utf8();
            if size > config.max_size as usize / 2 {
                let ez = self
                    .to_trunk_builder()
                    .content(split_content.iter().collect::<String>())
                    .build();
                trunks.push(ez);
                split_content.clear();
                size = c.len_utf8();
                split_content.push(c)
            } else {
                split_content.push(c);
            }
        });
        if !split_content.is_empty() {
            let ez = self
                .to_trunk_builder()
                .content(String::from_iter(&split_content))
                .build();
            trunks.push(ez);
        }
        trunks
    }
}

/// [EZRecord]'s builder
#[derive(Debug)]
pub struct EZRecordBuilder {
    record: EZRecord,
}

impl EZRecordBuilder {
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
        self.record.id = self.record.id();
        self.record.clone()
    }
}

impl Default for EZRecordBuilder {
    fn default() -> Self {
        EZRecordBuilder {
            record: EZRecord {
                id: 0,
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

pub(crate) fn next_date(time: OffsetDateTime) -> OffsetDateTime {
    time.date().midnight().assume_utc() + Duration::days(1)
}

#[cfg(feature = "backtrace")]
fn hook_panic() {
    std::panic::set_hook(Box::new(|p| {
        let bt = Backtrace::new();
        event!(panic & format!("ezlog: \n {p:?} \n{bt:?} \n"));
    }));
}

#[cfg(not(feature = "backtrace"))]
fn hook_panic() {
    std::panic::set_hook(Box::new(|p| {
        event!(panic & format!("ezlog: \n {p:?}"));
    }));
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use aead::KeyInit;
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, Nonce}; // Or `Aes128Gcm`
    use flate2::{bufread::ZlibDecoder, write::ZlibEncoder, Compression};

    use crate::{
        config::EZLogConfigBuilder, EZLogConfig, EZRecordBuilder, Header, RECORD_SIGNATURE_END,
        RECORD_SIGNATURE_START,
    };

    fn create_config() -> EZLogConfig {
        EZLogConfig::default()
    }

    #[cfg(feature = "decode")]
    fn create_all_feature_config() -> EZLogConfig {
        use crate::CipherKind;
        use crate::CompressKind;

        let key = b"an example very very secret key.";
        let nonce = b"unique nonce";
        EZLogConfigBuilder::new()
            .dir_path(
                dirs::cache_dir()
                    .unwrap()
                    .join("ezlog_test")
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
        assert_eq!(v.len(), Header::fixed_size());
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
    fn test_cipher() {
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
        assert_eq!(trunks[0].content, "深");
        assert_eq!(trunks[1].content, "圳");
    }

    #[cfg(feature = "decode")]
    #[test]
    fn teset_encode_decode() {
        use crate::EZLogger;
        use std::fs::{self, OpenOptions};
        use std::io::{BufReader, Seek, SeekFrom};
        use time::OffsetDateTime;

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
            .seek(SeekFrom::Start(Header::fixed_size() as u64))
            .unwrap();

        let decode = logger.decode_record(&mut reader).unwrap();
        println!("{}", String::from_utf8(decode).unwrap());

        fs::remove_dir_all(&config.dir_path).unwrap();
    }
}
