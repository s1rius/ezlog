/// # EZLog Event Listener
///[eventlisteners are good](https://publicobject.com/2022/05/01/eventlisteners-are-good/)
///
/// Jesse said
///
/// defining an event listener to make your systems observable.
/// It’s a lot of power in a simple pattern.
///
#[derive(Debug)]
pub enum Event {
    Init,
    InitError,
    CreateLogger,
    CreateLoggerError,
    CreateLoggerEnd,
    Record,
    RecordError,
    RecordEnd,
    RecordFilterOut,
    Compress,
    CompressError,
    CompressEnd,
    Encrypt,
    EncryptError,
    EncryptEnd,
    Flush,
    FlushError,
    FlushEnd,
    RequestLog,
    RequestLogError,
    RequestLogEnd,
    MapFile,
    MapFileError,
    MapFileEnd,
    RotateFile,
    RotateFileError,
    Trim,
    TrimError,
    TrimEnd,
    FFiError,
    ChannelError,
    Panic,
}

/// Every important log case make an event.
/// if you care about what's things going on, just register an event listener.
macro_rules! event {
    ($t:expr) => {
        crate::events::call_event($t, "ok")
    };
    ($t:expr, $info: expr) => {
        crate::events::call_event($t, $info)
    };
    ($t:expr, $info:expr, $err:expr) => {
        crate::events::call_event_error($t, $info, $err)
    };
}
pub(crate) use event;

pub trait EventListener {
    fn on_event(&self, event: Event, desc: &str);
    fn on_error_event(&self, event: Event, desc: &str, err: &LogError);
}

#[inline]
pub(crate) fn call_event(event: Event, desc: &str) {
    listener().on_event(event, desc);
}

#[inline]
pub(crate) fn call_event_error(event: Event, desc: &str, err: &LogError) {
    listener().on_error_event(event, desc, err)
}

use std::sync::Once;

pub static mut EVENT_LISTENER: &dyn EventListener = &NopEvent;
static EVENT_INIT: Once = Once::new();

pub fn listener() -> &'static dyn EventListener {
    if EVENT_INIT.is_completed() {
        unsafe { EVENT_LISTENER }
    } else {
        static NOP: NopEvent = NopEvent;
        &NOP
    }
}

pub fn set_event_listener(event: &'static dyn EventListener) {
    EVENT_INIT.call_once(|| unsafe {
        EVENT_LISTENER = event;
    })
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "linux",
    target_os = "windows"
))]
macro_rules! println_with_time {
    ($($arg:tt)*) => {{
        use time::{OffsetDateTime, format_description::well_known::Rfc3339};
        let now = OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or("".to_string());
        println!("{} {}", now, format_args!($($arg)*));
    }};
}

#[cfg(all(target_os = "android", feature = "log"))]
use android_logger::AndroidLogger;

/// android logcat already has time prefix
/// just print current log

#[cfg(target_os = "android")]
macro_rules! println_with_time {
    ($($arg:tt)*) => {{
        #[cfg(feature = "log")]
        crate::events::android_print(format_args!($($arg)*));
    }};
}
use crate::errors::LogError;
pub(crate) use println_with_time;

#[cfg(all(target_os = "android", feature = "log"))]
static ANDROID_LOGGER: once_cell::sync::OnceCell<AndroidLogger> = once_cell::sync::OnceCell::new();

#[cfg(all(target_os = "android", feature = "log"))]
pub(crate) fn android_print(record: std::fmt::Arguments) {
    use android_logger::Config;
    use log::{Log, RecordBuilder};

    let log = ANDROID_LOGGER.get_or_init(|| {
        AndroidLogger::new(
            Config::default()
                .with_tag("ezlog")
                .with_max_level(log::LevelFilter::Trace),
        )
    });

    let r = RecordBuilder::new().args(record).build();

    log.log(&r);
}

struct NopEvent;
impl EventListener for NopEvent {
    fn on_event(&self, _event: Event, _desc: &str) {}

    fn on_error_event(&self, _event: Event, _desc: &str, _err: &LogError) {}
}

/// Default [EventListener] implementation, print every event in console
pub struct EventPrinter;
impl EventPrinter {}

#[allow(unused_variables)]
impl EventListener for EventPrinter {
    fn on_event(&self, event: Event, desc: &str) {
        println_with_time!("{:?}, {}", event, desc);
    }
    fn on_error_event(&self, event: Event, desc: &str, err: &LogError) {
        println_with_time!("{:?}, {}, {:?}", event, desc, &err);
    }
}
