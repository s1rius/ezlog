/// # EZLog Event Listener
///
/// [eventlisteners are good](https://publicobject.com/2022/05/01/eventlisteners-are-good/)
///
/// Jesse said
/// > defining an event listener to make your systems observable. It’s a lot of power in a simple pattern.
///
///
#[allow(unused_variables)]
pub trait Event {
    fn init(&self, info: &str) {}
    fn init_err(&self, err: &str) {}
    fn create_logger(&self, id: &str) {}
    fn create_logger_end(&self, id: &str) {}
    fn create_logger_err(&self, id: &str, err: &str) {}
    fn record(&self, id: &str) {}
    fn record_end(&self, id: &str) {}
    fn compress(&self, id: &str) {}
    fn compress_end(&self, id: &str) {}
    fn compress_err(&self, id: &str, err: &str) {}
    fn encrypt(&self, id: &str) {}
    fn encrypt_end(&self, id: &str) {}
    fn encrypt_err(&self, id: &str, err: &str) {}
    fn unknown_err(&self, id: &str, err: &str) {}
    fn flush(&self, id: &str) {}
    fn flush_end(&self, id: &str) {}
    fn flush_all(&self) {}
    fn flush_all_end(&self) {}
    fn internal_err(&self, err: &str) {}
    fn record_filter_out(&self, id: &str, info: &str) {}
}

use std::sync::Once;

pub static mut EVENT_LISTENER: &dyn Event = &NopEvent;
static EVENT_INIT: Once = Once::new();

pub fn listener() -> &'static dyn Event {
    if EVENT_INIT.is_completed() {
        unsafe { EVENT_LISTENER }
    } else {
        static NOP: NopEvent = NopEvent;
        &NOP
    }
}

pub fn set_event_listener(event: &'static dyn Event) {
    EVENT_INIT.call_once(|| unsafe {
        EVENT_LISTENER = event;
    })
}

/// Every important log case make an event. 
/// if you care about what's things going on, just register an event listener.
#[macro_export]
macro_rules! event {
    (init $e:expr) => {
        $crate::events::listener().init($e)
    };
    (init_err $e:expr) => {
        $crate::events::listener().init_err($e);
    };
    (create_logger $log_name:expr) => {
        $crate::events::listener().create_logger($log_name);
    };
    (create_logger_end $log_name:expr) => {
        $crate::events::listener().create_logger_end($log_name);
    };
    (create_logger_fail $log_name:expr, $err:expr) => {
        $crate::events::listener().create_logger_err($log_name, $err);
    };
    (record $record_id:expr) => {
        $crate::events::listener().record($record_id);
    };
    (compress_start $record_id:expr) => {
        $crate::events::listener().compress($record_id);
    };
    (compress_end $record_id:expr) => {
        $crate::events::listener().compress_end($record_id);
    };
    (compress_fail $record_id:expr, $e:expr) => {
        $crate::events::listener().compress_err($record_id, $e);
    };
    (encrypt_start $record_id:expr) => {
        $crate::events::listener().encrypt($record_id);
    };
    (encrypt_end $record_id:expr) => {
        $crate::events::listener().encrypt_end($record_id);
    };
    (encrypt_fail $record_id:expr, $e:expr) => {
        $crate::events::listener().encrypt_err($record_id, $e)
    };
    (record_end $record_id:expr) => {
        $crate::events::listener().record_end($record_id);
    };
    (record_filter_out $record_id:expr, $info:expr) => {
        $crate::events::listener().record_filter_out($record_id, $info);
    };
    (unknown_err $record_id:expr, $e:expr) => {
        $crate::events::listener().unknown_err($record_id, $e)
    };
    (internal_err $e:expr) => {
        $crate::events::listener().internal_err($e)
    };
    (flush $log_name:expr) => {
        $crate::events::listener().flush($log_name)
    };
    (flush_end $log_name:expr) => {
        $crate::events::listener().flush_end($log_name);
    };
    (flush_all) => {
        $crate::events::listener().flush_all()
    };
    (flush_all_end) => {
        $crate::events::listener().flush_all_end();
    };
    (trim_logger_err $e:expr) => {
        $crate::events::listener().internal_err($e)
    };
    (query_log_files_err $e:expr) => {
        $crate::events::listener().internal_err($e)
    };
    (ffi_call_err $e:expr) => {
        $crate::events::listener().internal_err($e)
    };
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
macro_rules! println_with_time {
    ($($arg:tt)*) => {{
        use time::{OffsetDateTime, format_description::well_known::Rfc3339};
        let now = OffsetDateTime::now_utc().format(&Rfc3339).unwrap_or("".to_string());
        println!("{} {}", now, format_args!($($arg)*));
    }};
}

#[cfg(target_os = "android")]
use android_logger::AndroidLogger;

#[cfg(target_os = "android")]
/// android logcat already has time prefix
/// just print current log
macro_rules! println_with_time {
    ($($arg:tt)*) => {{
        crate::events::android_print(format_args!($($arg)*));
    }};
}

#[cfg(target_os = "android")]
static ANDROID_LOGGER: once_cell::sync::OnceCell<AndroidLogger> = once_cell::sync::OnceCell::new();

#[cfg(target_os = "android")]
fn android_print(record: std::fmt::Arguments) {
    use android_logger::Config;
    use log::{Log, RecordBuilder};

    let log = ANDROID_LOGGER.get_or_init(|| {
        AndroidLogger::new(
            Config::default()
                .with_tag("ezlog")
                .with_min_level(log::Level::Trace),
        )
    });

    let r = RecordBuilder::new().args(record).build();

    log.log(&r);
}

struct NopEvent;
impl Event for NopEvent {}

/// Default [Event] implementation, print every event in console
pub struct EventPrinter;
impl EventPrinter {}

impl Event for EventPrinter {
    fn init(&self, info: &str) {
        println_with_time!("{}, {}", "init", info);
    }
    fn init_err(&self, err: &str) {
        println_with_time!("{}, {}", "init err", err);
    }
    fn create_logger(&self, id: &str) {
        println_with_time!("{}, {}", "create logger", id)
    }
    fn create_logger_end(&self, id: &str) {
        println_with_time!("{}, {}", "create logger end", id)
    }
    fn create_logger_err(&self, id: &str, err: &str) {
        println_with_time!("{}, {}, {}", "create logger err", id, err)
    }
    fn record(&self, id: &str) {
        println_with_time!("{}, {}", id, "record")
    }
    fn record_end(&self, id: &str) {
        println_with_time!("{}, {}", id, "record end")
    }
    fn compress(&self, id: &str) {
        println_with_time!("{}, {}", id, "compress")
    }
    fn compress_end(&self, id: &str) {
        println_with_time!("{}, {}", id, "compress end")
    }
    fn compress_err(&self, id: &str, err: &str) {
        println_with_time!("{}, {}, {}", id, "compress end", err)
    }
    fn encrypt(&self, id: &str) {
        println_with_time!("{} encrypt ", id)
    }
    fn encrypt_end(&self, id: &str) {
        println_with_time!("{} encrypt end ", id)
    }
    fn encrypt_err(&self, id: &str, err: &str) {
        println_with_time!("{} encrypt err {}", id, err)
    }
    fn unknown_err(&self, id: &str, err: &str) {
        println_with_time!("{} unknown err {}", id, err)
    }
    fn flush(&self, id: &str) {
        println_with_time!("{} flush", id)
    }
    fn flush_end(&self, id: &str) {
        println_with_time!("{} flush end", id)
    }
    fn flush_all(&self) {
        println_with_time!("flush all")
    }
    fn flush_all_end(&self) {
        println_with_time!("flush all end")
    }
    fn internal_err(&self, err: &str) {
        println_with_time!("interanl err {}", err)
    }
    fn record_filter_out(&self, id: &str, info: &str) {
        println_with_time!("{} log filter , {}", id, info)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_print_with_time() {
        println_with_time!("{}", "no");
    }
}
