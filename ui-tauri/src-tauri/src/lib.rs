// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use std::{
    fs::File,
    io::{
        Cursor,
        Read,
    },
    path::{
        Path,
        PathBuf,
    },
    str::FromStr,
    sync::mpsc::channel,
};

use ezlog::{
    EZLogConfigBuilder,
    Header,
};
use serde_json::json;
use tauri::{
    Emitter,
    Manager,
    State,
};
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    pub data: Mutex<PathBuf>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            data: Mutex::new(PathBuf::new()),
        })
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            parse_header_and_extra,
            parse_log_file_to_records,
            pick_extenal_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn parse_header_and_extra(state: State<AppState>, file_path: String) -> Result<String, String> {
    log::debug!("parsing header and extra from file: {}", file_path);
    let path_str = file_path.strip_prefix("file://").unwrap_or(&file_path);
    let path = PathBuf::from_str(path_str).map_err(|e| e.to_string())?;

    let _ = state
        .data
        .try_lock()
        .map(|mut data| {
            *data = path.clone();
        })
        .inspect_err(|e| log::error!("lock error {e}"));
    inner_parse_header_and_extra_from_path(&path).map(|header_and_extra| {
        let extra_tuple: (String, String) = header_and_extra.1.unwrap_or_default();
        json!({"header": header_and_extra.0, "extra": extra_tuple.0, "extra_encode": extra_tuple.1})
            .to_string()
    })
}

fn inner_parse_header_and_extra_from_path(
    path: &PathBuf,
) -> Result<(Header, Option<(String, String)>), String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .map_err(|e| format!("fail read file {}", e))?;
    let mut cursor = Cursor::new(contents);
    ezlog::decode::decode_header_and_extra(&mut cursor).map_err(|e| e.to_string())
}

#[tauri::command]
fn parse_log_file_to_records(
    state: State<AppState>,
    key: String,
    nonce: String,
) -> Result<String, String> {
    state
        .data
        .try_lock()
        .map(|data| inner_parse_log_file_to_records(data.clone(), key, nonce))
        .map_err(|e| e.to_string())?
}

fn inner_parse_log_file_to_records<T: AsRef<Path>, U: AsRef<str>>(
    file_path: T,
    key: U,
    nonce: U,
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
                    log::error!("error while decoding record: {}", e);
                }
            }
        }
        if is_end {
            tx.send(()).expect("could not send signal on channel.");
            return None;
        }
        Some(0)
    };

    let config = EZLogConfigBuilder::new()
        .from_header(&header)
        .cipher_key(key.as_ref().as_bytes().to_vec())
        .cipher_nonce(nonce.as_ref().as_bytes().to_vec())
        .build();

    let compression = ezlog::create_compress(&config);
    let decryptor = ezlog::create_cryptor(&config).map_err(|e| e.to_string())?;

    ezlog::decode::decode_with_fn(&mut cursor, &compression, &decryptor, &header, json_closure);
    rx.recv()
        .map(|_| {
            log::debug!("decoded {} records", array.len());
        })
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&array).map_err(|e| e.to_string())
}

#[tauri::command]
fn pick_extenal_file(app: tauri::AppHandle) -> Result<String, String> {
    app.dialog().file().pick_file(move |file_path: Option<tauri_plugin_dialog::FilePath>| {
        if let Some(file_path) = file_path {
            match file_path.into_path() {
                Ok(path_buf) => {
                    log::info!("file path: {:?}", path_buf);
                    app.emit("file-get", format!("{:?}", path_buf)).unwrap_or_else(|e| {
                        log::error!("could not emit file-get event: {}", e)
                    });
                    match inner_parse_header_and_extra_from_path(&path_buf) {
                        Ok(header_and_extra) => {
                            let header = header_and_extra.0;
                            let extra = header_and_extra.1.unwrap_or_default();
                            if header.is_encrypt() {
                                log::info!("extra: {:?}", extra);
                                    let state: tauri::State<AppState> = match app.try_state() {
                                        Some(state) => state,
                                        None => return
                                    };
                                    let _ = state.data.try_lock().map(|mut data| {
                                        *data = path_buf;
                                    }).inspect_err(|e| log::error!("lock error {e}"));

                                    let json = json!({"header": header, "extra": extra.0, "extra_encode": extra.1}).to_string();
                                    app.emit("header-parsed", json).unwrap_or_else(|e| {
                                        log::error!("could not emit file-selected event: {}", e)
                                    });
                            } else {
                                log::info!("parse records");
                                match inner_parse_log_file_to_records(&path_buf, "", "") {
                                    Ok(records) => {
                                        app.emit("records-parsed", records).unwrap_or_else(|e| {
                                            log::error!("could not emit file-selected event: {}", e)
                                        });
                                    },
                                    Err(e) => {
                                        log::error!("could not parse log file: {}", e);
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            log::error!("could not parse header and extra: {}", e);
                        },
                    }
                }
                Err(e) => {
                    log::error!("invalid file path: {}", e);
                }
            }
        } else {
            log::error!("could not get file path");
        }
    });
    Ok(format!("seletc file"))
}
