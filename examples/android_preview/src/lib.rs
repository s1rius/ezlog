use ezlog::{
    CipherKind, CompressKind, EZLogConfig, EZLogConfigBuilder, EZRecordBuilder, EventPrinter,
    DEFAULT_LOG_NAME,
};

static EVENT_LISTENER: EventPrinter = EventPrinter;

/// Quick run example on android device
/// you can run this example, and see the logcat without open AndroidStudio/IDEA
///
/// # Example
///
/// ```shell
/// cargo install cargo-apk
/// cargo apk run -p ezlog-android-preview
/// adb logcat --pid=$(adb shell pidof -s rust.ezlog_android_preview)
/// ```
#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
#[allow(dead_code)]
fn main() {
    ezlog::init_with_event(&EVENT_LISTENER);
    let log_config = get_config();
    ezlog::create_log(log_config);
    let record = EZRecordBuilder::new().content("12345".to_string()).build();
    ezlog::log(record);
    ezlog::flush(ezlog::DEFAULT_LOG_NAME);
    println!("end");
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
