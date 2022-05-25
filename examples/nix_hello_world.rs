use std::thread;
use std::time::Duration;

use ezlog::{CipherKind, CompressKind, EZLogConfigBuilder, EZRecord};
use log::{LevelFilter, info, trace, warn, error, debug};
use log::{Level, Metadata, Record};

static LOGGER: SimpleLogger = SimpleLogger;

pub fn main() {
    println!("start");
    ezlog::init();
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .expect("log set error");

    let key = b"an example very very secret key.";
    let nonce = b"unique nonce";
    let log_config = EZLogConfigBuilder::new()
        .dir_path(
            dirs::desktop_dir()
                .unwrap()
                .into_os_string()
                .into_string()
                .expect("dir path error"),
        )
        .name(ezlog::DEFAULT_LOG_NAME.to_string())
        .file_suffix(String::from("mmap"))
        .max_size(1024)
        .compress(CompressKind::ZLIB)
        .cipher(CipherKind::AES256GCM)
        .cipher_key(key.to_vec())
        .cipher_nonce(nonce.to_vec())
        .build();

    ezlog::create_log(log_config);
    trace!("create default log");
    debug!("debug ez log");
    info!("now have a log");
    warn!("test log to file");
    error!("log complete");
    ezlog::flush(ezlog::DEFAULT_LOG_NAME);
    println!("end");

    thread::sleep(Duration::from_secs(3));
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            ezlog::log(EZRecord::from(record))
        }
    }

    fn flush(&self) {}
}
