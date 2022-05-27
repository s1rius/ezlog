use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Duration;
use std::{io, thread};

use ezlog::{
    CipherKind, CompressKind, EZLogConfig, EZLogConfigBuilder, EZLogger, EZRecord,
    V1_LOG_HEADER_SIZE,
};
use log::{debug, error, info, trace, warn, LevelFilter};
use log::{Level, Metadata, Record};
use time::OffsetDateTime;

static LOGGER: SimpleLogger = SimpleLogger;

pub fn main() {
    println!("start");
    ezlog::init();
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .expect("log set error");

    let log_config = get_config();

    ezlog::create_log(log_config);

    // thread::sleep(Duration::from_secs(1));

    trace!("1. create default log");
    debug!("2. debug ez log");
    info!("3. now have a log");
    warn!("4. test log to file");
    error!("5. log complete");
    ezlog::flush(ezlog::DEFAULT_LOG_NAME);
    println!("end");

    thread::sleep(Duration::from_secs(3));

    read_log_file_rewrite();
}

fn get_config() -> EZLogConfig {
    let key = b"an example very very secret key.";
    let nonce = b"unique nonce";
    EZLogConfigBuilder::new()
        .level(Level::Trace)
        .dir_path(
            dirs::desktop_dir()
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

fn read_log_file_rewrite() {
    let log_config = get_config();
    let (file, path) = log_config
        .create_mmap_file(OffsetDateTime::now_utc())
        .unwrap();
    let mut logger = EZLogger::new(log_config).unwrap();

    let mut br = BufReader::new(&file);

    let mut buffer = Vec::new();
    br.read_to_end(&mut buffer).unwrap();

    let mut cursor = Cursor::new(buffer);
    cursor
        .seek(SeekFrom::Start(V1_LOG_HEADER_SIZE as u64))
        .unwrap();

    let plaintext_log_path = path.with_extension(".log");
    let plaintext_log = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(plaintext_log_path)
        .unwrap();

    let mut w = BufWriter::new(plaintext_log);

    let mut end = false;

    loop {
        if end {
            break;
        }

        match logger.decode_from_read(&mut cursor) {
            Ok(buf) => {
                println!("{:?}", &buf);
                w.write(&buf).unwrap();
                w.write(b"\n").unwrap();
            }
            Err(_) => {
                end = true;
            }
        }
    }
    w.flush().unwrap();
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            ezlog::log(EZRecord::from(record))
        }
    }

    fn flush(&self) {}
}
