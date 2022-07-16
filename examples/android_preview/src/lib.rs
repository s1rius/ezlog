use std::{thread, time::Duration};

use ezlog::{
    CipherKind, CompressKind, EZLogCallback, EZLogConfig, EZLogConfigBuilder, EZRecordBuilder,
    EventPrinter, DEFAULT_LOG_NAME,
};

static EVENT_LISTENER: EventPrinter = EventPrinter;

/// Quick run example on android device
/// you can run this example, and see the logcat without open AndroidStudio/IDEA
///
/// # Example
///
/// ```shell
/// cargo install cargo-apk
/// cargo apk run -p ezlog_android_preview -Zbuild-std
/// adb logcat --pid=$(adb shell pidof -s rust.ezlog_android_preview)
/// ```
#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
#[allow(dead_code)]
fn main() {
    ezlog::init_with_event(&EVENT_LISTENER);
    ezlog::set_boxed_callback(Box::new(SimpleCallback {}));
    let log_config = get_config();
    ezlog::create_log(log_config);
    let record = EZRecordBuilder::new().content("12345".to_string()).build();
    ezlog::log(record);
    ezlog::flush(ezlog::DEFAULT_LOG_NAME);
    ezlog::request_log_files_for_date(DEFAULT_LOG_NAME, "2022_07_11");

    thread::sleep(Duration::from_secs(3));
    ezlog::log(EZRecordBuilder::new().content("5678".to_string()).build());
    println!("\n end");
}

fn get_config() -> EZLogConfig {
    let key = b"an example very very secret key.";
    let nonce = b"unique nonce";
    EZLogConfigBuilder::new()
        .level(ezlog::Level::Trace)
        .dir_path("data/data/rust.ezlog_android_preview/files/ezlog".to_string())
        .name(DEFAULT_LOG_NAME.to_string())
        .file_suffix(String::from("mmap"))
        .compress(CompressKind::ZLIB)
        .cipher(CipherKind::AES256GCM)
        .cipher_key(key.to_vec())
        .cipher_nonce(nonce.to_vec())
        .build()
}

struct SimpleCallback;

impl EZLogCallback for SimpleCallback {
    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]) {
        print!("{} {} {}", name, date, logs.join(" "));
    }
    fn on_fetch_fail(&self, name: &str, date: &str, err: &str) {
        print!("{} {} {}", name, date, err);
    }
}
