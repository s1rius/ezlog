use ezlog::{
    CipherKind,
    CompressKind,
    EZLogCallback,
    EZLogConfig,
    EZLogConfigBuilder,
    EZRecordBuilder,
    DEFAULT_LOG_NAME,
};
use rand::Rng;
use time::OffsetDateTime;

/// Quick run example on android device
/// you can run this example, and see the logcat without open AndroidStudio/IDEA
///
/// # Example
///
/// ```shell
/// cargo install cargo-apk
/// cargo apk run -p ezlog_android_preview
/// adb logcat --pid=$(adb shell pidof -s rust.ezlog_android_preview)
/// ```
#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
#[allow(dead_code)]
fn main() {
    ezlog::InitBuilder::new().debug(true).init();
    ezlog::set_boxed_callback(Box::new(SimpleCallback {}));
    let log_config = get_config();
    ezlog::create_log(log_config);
    let record = EZRecordBuilder::new().content("12345").build();
    ezlog::log(record);
    ezlog::flush(ezlog::DEFAULT_LOG_NAME);
    ezlog::request_log_files_for_date(
        DEFAULT_LOG_NAME,
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc(),
    );

    for i in 0..10 {
        ezlog::log(
            EZRecordBuilder::new()
                .content(format!("{}{}", i, random_string(100)))
                .build(),
        );
    }
    println!("\n end");
}

fn get_config() -> EZLogConfig {
    let key = b"an example very very secret key.";
    let nonce = b"unique nonce";
    EZLogConfigBuilder::new()
        .level(ezlog::Level::Trace)
        .dir_path("data/data/rust.ezlog_android_preview/files/ezlog")
        .name(DEFAULT_LOG_NAME)
        .file_suffix(String::from("mmap"))
        .compress(CompressKind::ZLIB)
        .cipher(CipherKind::AES256GCMSIV)
        .cipher_key(key.to_vec())
        .cipher_nonce(nonce.to_vec())
        .build()
}

struct SimpleCallback;

impl EZLogCallback for SimpleCallback {
    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]) {
        println!("fetch sucess {} {} {}", name, date, logs.join(" "));
    }
    fn on_fetch_fail(&self, name: &str, date: &str, err: &str) {
        println!("fetch fail {} {} {}", name, date, err);
    }
}

const S: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789,.:;!@#$%^&*()_+-";

fn random_string(length: u32) -> String {
    let mut owned_string: String = "".to_owned();
    for _ in 0..length {
        let mut chars = S.chars();
        let index = rand::thread_rng().gen_range(0..S.len());
        let c = chars.nth(index).unwrap();
        owned_string.push(c);
    }
    owned_string
}
