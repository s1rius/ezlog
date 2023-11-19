use std::fs::OpenOptions;
use std::io::{
    BufReader,
    BufWriter,
    Cursor,
    Read,
    Write,
};
use std::thread;
use std::time::Duration;

use ezlog::EZMsg;
use ezlog::Level;
use ezlog::{
    create_log,
    CipherKind,
    CompressKind,
    EZLogCallback,
    EZLogConfig,
    EZLogConfigBuilder,
    EventPrinter,
    Header,
};
use log::{
    debug,
    error,
    info,
    trace,
    warn,
    LevelFilter,
};
use rand::Rng;
use time::OffsetDateTime;

static EVENT_LISTENER: EventPrinter = EventPrinter;

pub fn main() {
    println!("start");
    let ezlog = ezlog::InitBuilder::new()
        .with_layer_fn(|msg| {
            if let EZMsg::Record(recode) = msg {
                println!("{}", ezlog::format(recode));
            }
        })
        .with_event_listener(&EVENT_LISTENER)
        .with_request_callback(SimpleCallback)
        .init();

    let log_config = get_config();

    create_log(log_config);

    log::set_boxed_logger(Box::new(ezlog))
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap();

    trace!("1. create default log");
    debug!("2. debug ez log");
    info!("3. now have a log");
    warn!("4. test log to file");
    error!("5. log complete");

    for i in 0..10 {
        trace!("{}{}", i, random_string(300));
    }

    ezlog::flush(ezlog::DEFAULT_LOG_NAME);

    println!("end");

    thread::sleep(Duration::from_secs(1));
    read_log_file_rewrite();

    ezlog::set_boxed_callback(Box::new(SimpleCallback));
    ezlog::request_log_files_for_date(
        ezlog::DEFAULT_LOG_NAME,
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc(),
    );
    thread::sleep(Duration::from_secs(1));
}

struct SimpleCallback;

impl EZLogCallback for SimpleCallback {
    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]) {
        println!("{} {} {}", name, date, logs.join(" "));
    }
    fn on_fetch_fail(&self, name: &str, date: &str, err: &str) {
        println!("{} {} {}", name, date, err);
    }
}

fn get_config() -> EZLogConfig {
    let key = b"an example very very secret key.";
    let nonce = b"unique nonce";
    EZLogConfigBuilder::new()
        .level(Level::Trace)
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
        .cipher(CipherKind::AES256GCMSIV)
        .cipher_key(key.to_vec())
        .cipher_nonce(nonce.to_vec())
        .extra("this is an plaintext extra infomation insert in the first of log file".to_string())
        .build()
}

#[allow(dead_code)]
fn read_log_file_rewrite() {
    let log_config = get_config();
    let (path, _mmap) = log_config.create_mmap_file().unwrap();
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    let mut br = BufReader::new(&file);

    let mut buffer = Vec::new();
    br.read_to_end(&mut buffer).unwrap();

    let mut cursor = Cursor::new(buffer);

    let plaintext_log_path = path.with_extension("ez.log");
    let plaintext_log = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(plaintext_log_path)
        .unwrap();

    let mut writer = BufWriter::new(plaintext_log);

    let compression = ezlog::create_compress(&log_config);
    let cryptor = ezlog::create_cryptor(&log_config).unwrap();
    let header = Header::decode(&mut cursor).unwrap();

    let my_closure = move |data: &Vec<u8>, flag: bool| {
        writer.write_all(data).unwrap();
        writer.write_all(b"\n").unwrap();
        if flag {
            writer.flush().unwrap();
        }
    };

    ezlog::decode::decode_with_fn(
        &mut cursor,
        header.version(),
        &compression,
        &cryptor,
        &header,
        my_closure,
    );
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
