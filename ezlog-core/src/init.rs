use std::time::Duration;

use crate::{
    event,
    EZLogCallback,
    EZMsg,
    EZRecord,
    EventListener,
    EventPrinter,
    Formatter,
    LogService,
};

/// InitBuilder is used to init ezlog
pub struct InitBuilder {
    listener: Option<&'static dyn EventListener>,
    debug: bool,
    layers: Vec<Box<dyn MsgHandler + Send + Sync>>,
    callback: Option<Box<dyn EZLogCallback>>,
    formatter: Option<Box<dyn Formatter>>,
}

impl InitBuilder {
    pub fn new() -> Self {
        Self {
            listener: None,
            debug: false,
            layers: vec![],
            callback: None,
            formatter: None,
        }
    }

    /// set debug mode
    ///
    /// # Example
    /// ```
    /// ezlog::InitBuilder::new().debug(true).init();
    /// ```
    pub fn debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// add a listener to handle all events
    ///
    /// # Example
    /// ```
    /// use std::fmt::Arguments;
    ///
    /// use ezlog::Event;
    /// use ezlog::LogError;
    ///
    /// struct MyEventListener;
    ///
    /// impl ezlog::EventListener for MyEventListener {
    ///     fn on_event(&self, event: Event, args: Arguments) {
    ///         println!("event: {:?}, desc: {}", event, args);
    ///     }
    ///
    ///     fn on_error_event(&self, event: Event, args: Arguments, err: Option<&LogError>) {
    ///         match err {
    ///             Some(error) => println!("event: {:?} {}, err: {:#?}", event, args, error),
    ///             None => println!("event: {:?} {}", event, args),
    ///         }
    ///     }
    /// }
    /// static LISTENER: MyEventListener = MyEventListener {};
    /// ezlog::InitBuilder::new()
    ///     .with_event_listener(&LISTENER)
    ///     .init();
    /// ```
    pub fn with_event_listener(mut self, listener: &'static dyn EventListener) -> Self {
        self.listener = Some(listener);
        self
    }

    /// add a layer to handle all operations
    ///
    /// # Example
    /// ```
    /// use ezlog::MsgHandler;
    /// struct MyLayer;
    ///
    /// impl MyLayer {
    ///     pub fn new() -> Self {
    ///         Self {}
    ///     }
    /// }
    ///
    /// impl ezlog::MsgHandler for MyLayer {
    ///     fn handle(&self, msg: &ezlog::EZMsg) {
    ///         println!("{:?}", msg);
    ///     }
    /// }
    ///
    /// ezlog::InitBuilder::new()
    ///     .with_layer(Box::new(MyLayer::new()))
    ///     .init();
    /// ```
    pub fn with_layer(mut self, layer: Box<dyn MsgHandler + Send + Sync>) -> Self {
        self.layers.push(layer);
        self
    }

    /// add a callback to receive log file path request result
    ///
    /// # Example
    /// ```
    /// let on_success: fn(&str, &str, &[&str]) = |name, date, logs| {
    ///     println!(
    ///         "on_success: name: {}, desc: {}, tags: {:?}",
    ///         name, date, logs
    ///     );
    /// };
    ///
    /// let on_fail: fn(&str, &str, &str) = |name, date, err| {
    ///     println!("on_fail: name: {}, desc: {}, err: {}", name, date, err);
    /// };
    /// ezlog::InitBuilder::new()
    ///     .with_request_callback_fn(on_success, on_fail)
    ///     .init();
    /// ```
    pub fn with_request_callback_fn(
        mut self,
        on_success: fn(&str, &str, &[&str]),
        on_fail: fn(&str, &str, &str),
    ) -> Self {
        let callback = EZLogCallbackProxy::new(on_success, on_fail);
        self.callback = Some(Box::new(callback));
        self
    }

    /// add a callback to receive log file path request result
    ///
    /// # Example
    /// ```
    /// use ezlog::EZLogCallback;
    ///
    /// struct MyCallback {}
    ///
    /// impl EZLogCallback for MyCallback {
    ///    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]) {
    ///        println!("on_success: name: {}, desc: {}, tags: {:?}", name, date, logs);
    ///    }
    ///
    ///    fn on_fetch_fail(&self, name: &str, date: &str, err: &str) {
    ///        println!("on_fail: name: {}, desc: {}, err: {}", name, date, err);
    ///    }
    /// }
    ///
    /// let callback: MyCallback = MyCallback {  };
    /// ezlog::InitBuilder::new()
    ///     .with_request_callback(callback)
    ///     .init();
    pub fn with_request_callback(mut self, callback: impl EZLogCallback + 'static) -> Self {
        let boxed_callback = Box::new(callback);
        self.callback = Some(boxed_callback);
        self
    }

    ///  add a layer to handle all operations
    ///
    /// # Example
    /// ```
    /// ezlog::InitBuilder::new()
    ///     .with_layer_fn(|msg| println!("{:?}", msg))
    ///     .init();
    /// ```
    pub fn with_layer_fn(mut self, layer: fn(&EZMsg)) -> Self {
        self.layers.push(Box::new(MsgHandlerFn::new(layer)));
        self
    }

    /// set a formatter to format log record
    ///
    /// # Example
    /// ```
    /// use ezlog::EZRecord;
    /// use ezlog::Formatter;
    /// struct MyFormatter;
    /// impl Formatter for MyFormatter {
    ///     fn format(&self, msg: &EZRecord) -> std::result::Result<Vec<u8>, ezlog::LogError> {
    ///         Ok(format!("{:?}", msg).into_bytes())
    ///     }
    /// }
    /// ezlog::InitBuilder::new()
    ///     .with_formatter(Box::new(MyFormatter))
    ///     .init();
    /// ```
    pub fn with_formatter(mut self, formatter: Box<dyn Formatter>) -> Self {
        self.formatter = Some(formatter);
        self
    }

    /// set a formatter to format log record
    ///
    /// # Example
    /// ```
    /// ezlog::InitBuilder::new()
    ///     .with_formatter_fn(|msg| format!("{:?}", msg).into_bytes())
    ///     .init();
    /// ```
    pub fn with_formatter_fn(mut self, op: fn(&EZRecord) -> Vec<u8>) -> Self {
        self.formatter = Some(Box::new(FormatterProxy::new(op)));
        self
    }

    /// real init ezlog
    pub fn init(self) -> EZLog {
        let log_service = crate::LOG_SERVICE.get_or_init(|| {
            #[cfg(all(target_os = "android", feature = "android_logger"))]
            if self.debug {
                android_logger::init_once(
                    android_logger::Config::default()
                        .with_max_level(log::LevelFilter::Trace)
                        .with_log_buffer(android_logger::LogId::Main),
                );
            }

            // if debug, set a default event listener
            if self.debug && self.listener.is_none() {
                static EVENT: EventPrinter = EventPrinter {};
                crate::set_event_listener(&EVENT);
            }

            if let Some(listener) = self.listener {
                crate::set_event_listener(listener);
            }

            if let Some(callback) = self.callback {
                crate::set_boxed_callback(callback);
            }

            if let Some(formatter) = self.formatter {
                crate::set_boxed_formatter(formatter);
            }
            LogService::new(self.layers)
        });

        // after init, drain messages in the queue, and dispatch them
        let create_logger_msgs: Vec<EZMsg> = {
            let mut create_logger_msgs = Vec::new();

            if let Some(mut deque) = crate::PRE_INIT_QUEUE.try_lock_for(Duration::from_millis(200))
            {
                while let Some(msg) = deque.front() {
                    // Collect all CreateLogger messages
                    if matches!(msg, EZMsg::CreateLogger { .. }) {
                        // Remove from front and collect
                        if let Some(msg) = deque.pop_front() {
                            create_logger_msgs.push(msg);
                        }
                    } else {
                        break;
                    }
                }
            } else {
                event!(
                    crate::Event::InitError,
                    "Failed to acquire lock on PRE_INIT_QUEUE, messages may be lost."
                );
            }
            create_logger_msgs
        };

        // Dispatch all CreateLogger messages
        for msg in create_logger_msgs {
            log_service.dispatch(msg);
        }
        EZLog {}
    }
}

pub(crate) fn dispatch_cache_records(log_name: impl AsRef<str>) {
    if let Some(log_service) = crate::LOG_SERVICE.get() {
        if let Some(mut deque) = crate::PRE_INIT_QUEUE.try_lock_for(Duration::from_millis(50)) {
            //iterator the deque, remove the records that match the target
            let mut i = 0;
            while i < deque.len() {
                if let Some(EZMsg::Record(record)) = deque.get(i) {
                    if record.log_name() == log_name.as_ref() {
                        // Remove and dispatch
                        if let Some(msg) = deque.remove(i) {
                            log_service.dispatch(msg);
                            continue;
                        } else {
                            // If we can't remove the message, just break
                            break;
                        }
                    }
                }
                i += 1;
            }
        } else {
            event!(
                crate::Event::InitError,
                "Failed to acquire lock on PRE_INIT_QUEUE, records may be lost."
            );
        }
    }
}

impl Default for InitBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub trait MsgHandler {
    fn handle(&self, msg: &EZMsg);
}

pub(crate) struct MsgHandlerFn {
    handler: fn(&EZMsg),
}

impl MsgHandlerFn {
    pub fn new(handler: fn(&EZMsg)) -> Self {
        Self { handler }
    }
}

impl MsgHandler for MsgHandlerFn {
    fn handle(&self, msg: &EZMsg) {
        (self.handler)(msg);
    }
}

struct EZLogCallbackProxy {
    success_fn: fn(&str, &str, &[&str]),
    fail_fn: fn(&str, &str, &str),
}

impl EZLogCallbackProxy {
    pub fn new(success_fn: fn(&str, &str, &[&str]), fail_fn: fn(&str, &str, &str)) -> Self {
        Self {
            success_fn,
            fail_fn,
        }
    }
}

impl EZLogCallback for EZLogCallbackProxy {
    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]) {
        (self.success_fn)(name, date, logs);
    }

    fn on_fetch_fail(&self, name: &str, date: &str, err: &str) {
        (self.fail_fn)(name, date, err);
    }
}

struct FormatterProxy {
    op: fn(&EZRecord) -> Vec<u8>,
}
impl FormatterProxy {
    fn new(op: fn(&EZRecord) -> Vec<u8>) -> FormatterProxy {
        FormatterProxy { op }
    }
}

impl Formatter for FormatterProxy {
    fn format(&self, msg: &EZRecord) -> std::result::Result<Vec<u8>, crate::LogError> {
        Ok((self.op)(msg))
    }
}

#[derive(Clone, Copy)]
pub struct EZLog {}

#[cfg(feature = "log")]
use log::{
    Metadata,
    Record,
};

#[cfg(feature = "log")]
impl log::Log for EZLog {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        crate::log(record.into())
    }

    fn flush(&self) {}
}

#[cfg(test)]
mod tests {
    use std::{
        sync::mpsc::channel,
        time::Duration,
    };

    use crate::{
        EZLogConfigBuilder,
        EZRecord,
    };

    #[test]
    pub fn test_log_before_init() {
        let (tx, rx) = channel::<bool>();

        let record = EZRecord::builder()
            .log_name(crate::DEFAULT_LOG_NAME)
            .level(crate::Level::Debug)
            .target("test")
            .build();
        crate::log(record);

        assert!(crate::PRE_INIT_QUEUE.lock().len() == 1);

        let config = EZLogConfigBuilder::new()
            .level(crate::Level::Trace)
            .dir_path(
                test_compat::test_path()
                    .into_os_string()
                    .into_string()
                    .expect("dir path error"),
            )
            .name(crate::DEFAULT_LOG_NAME)
            .build();

        crate::create_log(config);

        assert!(crate::PRE_INIT_QUEUE.lock().len() == 2);

        crate::InitBuilder::new().debug(true).init();

        let tx_clone = tx.clone();
        crate::post_msg(crate::EZMsg::Action(Box::new(move || {
            let is_empty = crate::LOG_SERVICE.wait().loggers.read().unwrap().is_empty();
            let _ = tx_clone.send(is_empty);
        })));

        let is_empty = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(!is_empty)
    }
}
