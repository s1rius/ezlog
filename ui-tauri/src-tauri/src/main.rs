// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fs::File,
    io::{
        Cursor,
        Read,
    },
    sync::mpsc::channel,
};

use ezlog::EZLogConfigBuilder;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            parse_header_and_extra,
            parse_log_file_to_records,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn parse_header_and_extra(file_path: String) -> Result<String, String> {
    println!("get file");
    let mut file = File::open(file_path).unwrap();
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).unwrap();
    let mut cursor = Cursor::new(contents);
    ezlog::decode::decode_header_and_extra(&mut cursor)
        .map(|header_and_extra| {
            let extra_tuple = header_and_extra.1.unwrap_or_default();
            let json_string = format!(
                "{{\"{}\":{},\"{}\":{},\"{}\":{},\"{}\":\"{}\",\"{}\":\"{}\"}}",
                "timestamp",
                header_and_extra.0.timestamp().unix_timestamp(),
                "version",
                Into::<u8>::into(*header_and_extra.0.version()),
                "encrypt",
                if header_and_extra.0.is_encrypt() {
                    1
                } else {
                    0
                },
                "extra",
                extra_tuple.0,
                "extra_encode",
                extra_tuple.1
            );
            json_string
        })
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn parse_log_file_to_records(
    file_path: String,
    key: String,
    nonce: String,
) -> Result<String, String> {
    let mut file = File::open(file_path).map_err(|e| e.to_string())?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).map_err(|e| e.to_string())?;
    let mut cursor = Cursor::new(contents);

    let (header, _extra) =
        ezlog::decode::decode_header_and_extra(&mut cursor).map_err(|e| e.to_string())?;

    let mut array: Vec<String> = Vec::new();
    let (tx, rx) = channel();

    let json_closure = |data: &Vec<u8>, is_end: bool| {
        if !data.is_empty() {
            match ezlog::decode::decode_record(data) {
                Ok(r) => array.push(serde_json::to_string(&r).unwrap_or_default()),
                Err(e) => {
                    println!("{}", e)
                }
            }
        }
        if is_end {
            tx.send(()).expect("Could not send signal on channel.");
            return None;
        }
        Some(0)
    };

    let config = EZLogConfigBuilder::new()
        .from_header(&header)
        .cipher_key(key.into())
        .cipher_nonce(nonce.into())
        .build();

    let compression = ezlog::create_compress(&config);
    let decryptor = ezlog::create_cryptor(&config).map_err(|e| e.to_string())?;

    ezlog::decode::decode_with_fn(&mut cursor, &compression, &decryptor, &header, json_closure);
    rx.recv().map_err(|e| e.to_string())?;
    serde_json::to_string(&array).map_err(|e| e.to_string())
}
