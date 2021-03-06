use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom};

use std::thread;
use std::time::Duration;

use ezlog::{
    create_log, CipherKind, CompressKind, EZLogCallback, EZLogConfig, EZLogConfigBuilder, EZLogger,
    EZRecord, EventPrinter, Header,
};
use log::{debug, error, info, trace, warn, LevelFilter};
use log::{Metadata, Record};
use time::OffsetDateTime;

static LOGGER: SimpleLogger = SimpleLogger;
static EVENT_LISTENER: EventPrinter = EventPrinter;

pub fn main() {
    println!("start");
    ezlog::init_with_event(&EVENT_LISTENER);
    ezlog::set_boxed_callback(Box::new(SimpleCallback));
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .expect("log set error");

    let log_config = get_config();

    create_log(log_config);

    trace!("1. create default log");
    debug!("2. debug ez log");
    info!("3. now have a log");
    warn!("4. test log to file");
    error!("5. log complete");

    ezlog::flush(ezlog::DEFAULT_LOG_NAME);
    ezlog::request_log_files_for_date(ezlog::DEFAULT_LOG_NAME, "2022_06_19");
    println!("end");

    thread::sleep(Duration::from_secs(1));
    read_log_file_rewrite();
}

fn get_config() -> EZLogConfig {
    let key = b"an example very very secret key.";
    let nonce = b"unique nonce";
    EZLogConfigBuilder::new()
        .level(ezlog::Level::Trace)
        .dir_path(
            dirs::cache_dir()
                .unwrap()
                .into_os_string()
                .into_string()
                .expect("dir path error"),
        )
        .name(ezlog::DEFAULT_LOG_NAME.to_string())
        .file_suffix(String::from("mmap"))
        .compress(CompressKind::ZLIB)
        .cipher(CipherKind::AES256GCM)
        .cipher_key(key.to_vec())
        .cipher_nonce(nonce.to_vec())
        .build()
}

#[allow(dead_code)]
fn read_log_file_rewrite() {
    let log_config = get_config();
    let (path, _mmap) = log_config
        .create_mmap_file(OffsetDateTime::now_utc())
        .unwrap();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let mut br = BufReader::new(&file);

    let mut buffer = Vec::new();
    br.read_to_end(&mut buffer).unwrap();

    let mut cursor = Cursor::new(buffer);
    cursor
        .seek(SeekFrom::Start(Header::fixed_size() as u64))
        .unwrap();

    let plaintext_log_path = path.with_extension("ez.log");
    let plaintext_log = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(plaintext_log_path)
        .unwrap();

    let mut writer = BufWriter::new(plaintext_log);

    let mut compression = EZLogger::create_compress(&log_config);
    let mut cryptor = EZLogger::create_cryptor(&log_config).unwrap();

    EZLogger::decode_body_and_write(&mut cursor, &mut writer, &mut compression, &mut cryptor)
        .unwrap();
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            ezlog::log(EZRecord::from(record))
        }
    }

    fn flush(&self) {}
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
