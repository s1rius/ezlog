use ezlog::Event;
use ezlog::LogError;
use ezlog::{EZLogConfigBuilder, EZRecord};
use time::OffsetDateTime;

#[test]
fn test_ezlog_init() {
    struct MyLayer;

    impl MyLayer {
        pub fn new() -> Self {
            Self {}
        }
    }

    impl ezlog::MsgHandler for MyLayer {
        fn handle(&self, msg: &ezlog::EZMsg) {
            println!("{:?}", msg);
        }
    }

    struct MyEventListener;

    impl ezlog::EventListener for MyEventListener {
        fn on_event(&self, event: Event, desc: &str) {
            println!("event: {:?} {}", event, desc);
        }

        fn on_error_event(&self, event: Event, desc: &str, err: &LogError) {
            println!("event: {:?} {}, err: {}", event, desc, err);
        }
    }
    static LISTENER: MyEventListener = MyEventListener {};

    let on_success: fn(&str, &str, &[&str]) = |name, date, logs| {
        println!(
            "on_success: name: {}, desc: {}, tags: {:?}",
            name, date, logs
        );
    };

    let on_fail: fn(&str, &str, &str) = |name, date, err| {
        println!("on_fail: name: {}, desc: {}, err: {}", name, date, err);
    };

    ezlog::InitBuilder::new()
        .with_layer(Box::new(MyLayer::new()))
        .with_event_listener(&LISTENER)
        .with_request_callback_fn(on_success, on_fail)
        .with_formatter_fn(|msg| format!("{:?}", msg).into_bytes())
        .init();
}

#[test]
fn test_logger_create() {
    ezlog::InitBuilder::new().debug(true).init();
    let config = EZLogConfigBuilder::new()
        .dir_path(
            dirs::cache_dir()
                .unwrap()
                .into_os_string()
                .into_string()
                .expect("dir path error"),
        )
        .name("test".to_string())
        .build();
    ezlog::create_log(config);
}

#[test]
fn test_ezlog_log() {
    test_logger_create();
    let record = EZRecord::builder().log_name("test".to_string()).build();
    ezlog::log(record);
}

#[test]
fn test_ezlog_trim() {
    test_logger_create();
    ezlog::trim();
}

#[test]
fn test_ezlog_flush() {
    test_logger_create();
    ezlog::flush_all();
}

#[test]
fn test_ezlog_request() {
    test_logger_create();
    ezlog::request_log_files_for_date("test", OffsetDateTime::now_utc(), OffsetDateTime::now_utc());
}
