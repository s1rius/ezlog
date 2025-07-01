/// # Event
///
///[eventlisteners are good](https://publicobject.com/2022/05/01/eventlisteners-are-good/)
///
/// Jesse said
///
/// defining an event listener to make your systems observable.
/// It’s a lot of power in a simple pattern.
use std::sync::OnceLock;

use crate::errors::LogError;

/// Event types that can occur during logging operations
#[derive(Debug, Clone, Copy)]
pub enum Event {
    // Initialization events
    Init,
    InitError,

    // Logger lifecycle events
    CreateLogger,
    CreateLoggerError,
    CreateLoggerEnd,

    // Record processing events
    Record,
    RecordError,
    RecordEnd,
    RecordFilterOut,

    // Compression events
    Compress,
    CompressError,
    CompressEnd,

    // Encryption events
    Encrypt,
    EncryptError,
    EncryptEnd,

    // I/O events
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

    // Other Error events
    FFIError,
    ChannelError,
}

/// Trait for listening to events with proper thread safety
///
/// EventListener must be Send + Sync to be used across threads safely.
/// It receives format_args! directly for zero-cost formatting when events are enabled.
pub trait EventListener: Send + Sync {
    /// Called for regular events
    fn on_event(&self, event: Event, args: std::fmt::Arguments<'_>);

    /// Called for error events with additional error information
    fn on_error_event(&self, event: Event, args: std::fmt::Arguments<'_>, err: Option<&LogError>);
}

// =============================================================================
// Feature-gated macros
// =============================================================================

/// Event macro with full formatting support when feature "event" is enabled
///
/// Usage:
/// - `event!(Event::Init)` - Simple event without description
/// - `event!(Event::CreateLogger, "logger name: {}", name)` - Formatted description
/// - `event!(Event::CreateLogger; description)` - Simple description (shorthand)
/// - `event!(Event::CreateLoggerError; err)` - Error event without description
/// - `event!(Event::CreateLoggerError, "failed to create {}", name; err)` - Error event with formatted description
#[cfg(feature = "event")]
macro_rules! event {
    // event!(Event::Init)
    ($event:expr) => {
        $crate::events::call_event($event, format_args!(""))
    };
    // event!(Event::CreateLogger, "logger name: {}", name)
    ($event:expr, $fmt:literal $(, $args:expr)*) => {
        $crate::events::call_event($event, format_args!($fmt, $($args),*))
    };
    // event!(Event::CreateLogger, desc) - shorthand for descriptions
    ($event:expr, $desc:expr) => {
        $crate::events::call_event($event, format_args!("{}", $desc))
    };
    // event!(Event::CreateLoggerError; "logger name: {}", name)
    (!$event:expr; $fmt:literal $(, $args:expr)*) => {
        $crate::events::call_event_error($event, format_args!($fmt, $($args),*), None)
    };
    // event!(Event::CreateLoggerError; err) - error event without description
    (!$event:expr; $err:expr) => {
        $crate::events::call_event_error($event, format_args!("{}", $err), None)
    };
    // event!(Event::CreateLoggerError, "failed to create {}", name; err) - error event with formatted description
    (!$event:expr, $fmt:literal $(, $args:expr)*; $err:expr) => {
        $crate::events::call_event_error($event, format_args!($fmt, $($args),*), Some($err))
    };
}

/// No-op event macro when feature "event" is disabled
///
/// These macros consume their arguments to prevent "unused variable" warnings
/// while generating zero runtime overhead.
#[cfg(not(feature = "event"))]
macro_rules! event {
    ($event:expr) => {
        {
            let _ = $event;
        }
    };
    ($event:expr, $fmt:literal $(, $args:expr)*) => {
        {
            let _ = $event;
            let _ = ($fmt, $($args),*);
        }
    };
    ($event:expr, $desc:expr) => {
        {
            let _ = $event;
            let _ = $desc;
        }
    };
    (!$event:expr; $fmt:literal $(, $args:expr)*) => {
        {
            let _ = $event;
            let _ = ($fmt, $($args),*);
        }
    };
    (!$event:expr; $err:expr) => {
        {
            let _ = $event;
            let _ = $err;
        }
    };
    (!$event:expr, $fmt:literal $(, $args:expr)*; $err:expr) => {
        {
            let _ = $event;
            let _ = ($fmt, $($args),*);
            let _err: &LogError = $err;
        }
    };
}

pub(crate) use event;

// =============================================================================
// Global event listener management
// =============================================================================

/// Thread-safe global event listener storage using OnceLock
///
/// This is safe and doesn't require unsafe code unlike the original implementation.
/// OnceLock ensures the listener is set only once and provides thread-safe access.
static EVENT_LISTENER: OnceLock<&'static dyn EventListener> = OnceLock::new();

/// Static no-op event listener instance
#[cfg(feature = "event")]
static NOP_EVENT: NopEvent = NopEvent;

/// Get the current event listener
///
/// Returns the registered listener or a no-op listener if none is set.
/// When events are disabled, always returns the no-op listener for dead code elimination.
#[cfg(feature = "event")]
pub fn listener() -> &'static dyn EventListener {
    EVENT_LISTENER.get().copied().unwrap_or(&NOP_EVENT)
}

/// Set the global event listener
///
/// This can only be called once. Subsequent calls are ignored.
/// Thread-safe and doesn't require unsafe code.
pub fn set_event_listener(event: &'static dyn EventListener) {
    let _ = EVENT_LISTENER.set(event);
}

// =============================================================================
// Event dispatch functions
// =============================================================================

/// Dispatch a regular event (feature-gated)
#[cfg(feature = "event")]
#[inline]
pub(crate) fn call_event(event: Event, args: std::fmt::Arguments<'_>) {
    listener().on_event(event, args);
}

/// Dispatch an error event (feature-gated)
#[cfg(feature = "event")]
#[inline]
pub(crate) fn call_event_error(
    event: Event,
    args: std::fmt::Arguments<'_>,
    err: Option<&LogError>,
) {
    listener().on_error_event(event, args, err)
}

// =============================================================================
// Platform-specific printing utilities
// =============================================================================

/// Print with timestamp on desktop platforms
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

/// Print via Android logger on Android (logcat already has timestamps)
#[cfg(target_os = "android")]
macro_rules! println_with_time {
    ($($arg:tt)*) => {{
        #[cfg(all(target_os = "android", feature = "android_logger"))] {
            crate::events::android_print(format_args!($($arg)*));
        }
    }};
}

/// Android-specific print function
#[cfg(all(target_os = "android", feature = "android_logger"))]
#[inline]
pub(crate) fn android_print(record: std::fmt::Arguments) {
    let s = log::RecordBuilder::new()
        .args(record)
        .level(log::Level::Trace)
        .module_path(Some("ezlog"))
        .build();
    android_logger::log(&s);
}

pub(crate) use println_with_time;

/// Default event listener that prints events to console
///
/// Useful for debugging and development.
pub struct EventPrinter;

impl EventListener for EventPrinter {
    fn on_event(&self, event: Event, args: std::fmt::Arguments<'_>) {
        println_with_time!("{:?}, {}", event, args);
    }

    fn on_error_event(&self, event: Event, args: std::fmt::Arguments<'_>, err: Option<&LogError>) {
        match err {
            Some(error) => println!("event: {:?} {}, err: {:#?}", event, args, error),
            None => println!("event: {:?} {}", event, args),
        }
    }
}

/// No-op event listener that does nothing
///
/// Used as default when no listener is set or when events are disabled.
#[cfg(feature = "event")]
struct NopEvent;

#[cfg(feature = "event")]
impl EventListener for NopEvent {
    fn on_event(&self, _event: Event, _args: std::fmt::Arguments<'_>) {}
    fn on_error_event(
        &self,
        _event: Event,
        _args: std::fmt::Arguments<'_>,
        _err: Option<&LogError>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_macros() {
        // Simple event
        event!(Event::Init);

        // Formatted event
        event!(Event::CreateLogger, "logger name: {}", "test");

        // Shorthand event
        let desc = "test description";
        event!(Event::CreateLogger, desc);

        // Error event without description
        let err = LogError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        event!(!Event::CreateLoggerError; &err);

        // Error event with formatted description
        event!(!Event::CreateLoggerError, "failed to create {}", "test"; &err);
    }

    #[test]
    #[cfg(feature = "event")]
    fn test_event_listener() {
        static PRINTER: EventPrinter = EventPrinter;
        set_event_listener(&PRINTER);

        // Test that listener is set
        let current = listener();
        // Note: We can't directly compare trait objects, but we can test behavior
        current.on_event(Event::Init, format_args!("test"));
    }
}
