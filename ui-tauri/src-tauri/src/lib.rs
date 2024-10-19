// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{Seek, SeekFrom};
use std::sync::Mutex;
use std::{
    fs::File,
    io::{
        Cursor,
        Read,
    },
    path::{
        PathBuf,
    },
    str::FromStr,
    sync::mpsc::channel,
};

use ezlog::{
    EZLogConfigBuilder,
    Header,
};
use serde::{Serialize};
use serde_json::json;
use tauri::{
    Emitter,
    Manager,
    State,
};
use tauri_plugin_android_fs::{
    AndroidFsExt,
    FileAccessMode,
};
use tauri_plugin_dialog::DialogExt;

pub struct AppState {
    pub data: Mutex<Option<File>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            data: Mutex::new(None),
        })
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_android_fs::init())
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
        .map(|mut data| *data = File::open(&path).ok())
        .inspect_err(|e| log::error!("lock error {e}"));

    let mut file = File::open(path).map_err(|e| e.to_string())?;
    inner_parse_header_and_extra(&mut file).map(|header_and_extra| {
        let extra_tuple: (String, String) = header_and_extra.1.unwrap_or_default();
        json!({"header": header_and_extra.0, "extra": extra_tuple.0, "extra_encode": extra_tuple.1})
            .to_string()
    })
}

fn inner_parse_header_and_extra(
    file: &mut File,
) -> Result<(Header, Option<(String, String)>), String> {
    debug_file_content(file)?;
    
    file.seek(SeekFrom::Start(0)).map_err(|e| {
        log::error!("Failed to seek to start: {}", e);
        e.to_string()
    })?;

    let mut contents = Vec::new();
    match file.read_to_end(&mut contents) {
        Ok(bytes_read) => {
            log::info!("Successfully read {} bytes from file", bytes_read);
            if bytes_read == 0 {
                return Err("No bytes read from file".to_string());
            }
        }
        Err(e) => {
            log::error!("Failed to read file: {}", e);
            log::error!("Error kind: {:?}", e.kind());
            
            // 尝试部分读取来诊断问题
            let mut buffer = vec![0u8; 1024]; // 读取前1KB
            file.seek(SeekFrom::Start(0)).ok();
            match file.read(&mut buffer) {
                Ok(n) => log::info!("Partial read successful: {} bytes", n),
                Err(partial_err) => log::error!("Even partial read failed: {}", partial_err),
            }
            
            return Err(format!("Failed to read file: {} (kind: {:?})", e, e.kind()));
        }
    }

    let mut cursor = Cursor::new(contents);
    log::info!("Cursor created with {} bytes", cursor.get_ref().len());
    log::debug!("First 4 bytes at position {}: {:?}", cursor.position(), &cursor.get_ref()[0..4]);
    ezlog::decode::decode_header_and_extra(&mut cursor).map_err(|e| e.to_string())
}

fn debug_file_content(file: &mut File) -> Result<(), String> {
    let current_pos = file.stream_position().map_err(|e| e.to_string())?;
    log::info!("Current file position: {}", current_pos);
    
    // 读取前 32 字节来检查文件头
    let mut header_bytes = vec![0u8; 32];
    match file.read(&mut header_bytes) {
        Ok(n) => {
            log::info!("Read {} bytes from file start", n);
            log::info!("First 32 bytes: {:?}", &header_bytes[..n.min(32)]);
            
            // 重置文件位置
            file.seek(SeekFrom::Start(current_pos)).map_err(|e| e.to_string())?;
        }
        Err(e) => {
            log::error!("Failed to read header bytes: {}", e);
            return Err(e.to_string());
        }
    }
    
    Ok(())
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
        .map(|data| {
            if let Some(file) = data.as_ref() {
                file.try_clone()
                    .map(|f| inner_parse_log_file(&f, key, nonce))
                    .unwrap_or_else(|e| Err(format!("could not clone file: {}", e)))
            } else {
                Err("No file is currently loaded".to_string())
            }
        })
        .map_err(|e| e.to_string())?
}

fn inner_parse_log_file<U: AsRef<str>>(
    mut file: &File,
    key: U,
    nonce: U,
) -> Result<String, String> {
    if let Err(e) = file.seek(SeekFrom::Start(0)) {
        log::warn!("Cannot seek to start: {}", e);
    }
    
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).map_err(|e| {
        e.to_string()
    } )?;
    let mut cursor = Cursor::new(contents);

    let (header, _extra) =
        ezlog::decode::decode_header_and_extra(&mut cursor).map_err(|e| {
            log::error!("decode header fail,{:?}", e);
            e.to_string()
        })?;

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
            tx.send(()).unwrap_or_else(|e| log::error!("could not send signal on channel: {}", e));
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
    if matches!(tauri_plugin_os::type_(), tauri_plugin_os::OsType::Android) {
        let app_clone = app.clone();
        std::thread::spawn(|| {
            let app2 = app_clone.clone();
            pick_file_on_android(app_clone).unwrap_or_else(move |e| {
                let message = format!("could not pick file: {}", e);
                toast_error(&app2, &message);
            });
        });
    } else {
        app.dialog()
        .file()
        .pick_file(move |file_path: Option<tauri_plugin_dialog::FilePath>| {
            let file_path_desc = format!("{:?}", file_path);
            if let Some(file_path) = file_path {
                match file_path.clone().into_path() {
                    Ok(path_buf) => {
                        log::info!("file path: {:?}", path_buf);
                        app.emit("file-get", format!("{:?}", path_buf.file_name()))
                            .unwrap_or_else(|e| {
                                log::error!("could not emit file-get event: {}", e)
                            });
                        File::open(path_buf)
                            .map(|mut f| parse_log_file_and_emit_result(&mut f, &app))
                            .unwrap_or_else(|e| {
                                toast_error(&app, &format!("could not open file: {}, {}", file_path_desc, e));
                            });
                    }
                    Err(e) => {
                        toast_error(&app, &format!("invalid file path: {}, {}", file_path_desc, e));
                    }
                }
            } else {
                toast_error(&app, "no file selected");
            }
        });
    }
    Ok(format!("seletc file"))
}

fn pick_file_on_android(app: tauri::AppHandle) -> tauri_plugin_android_fs::Result<()> {
    let api = app.android_fs();

    // pick files to read and write
    let selected_files = api.show_open_file_dialog(
        None,     // Initial location
        &["*/*"], // Target MIME types
        false,     // Allow multiple files
    )?;

    if selected_files.is_empty() {
        toast_error(&app, "no file select")
    } else {
        selected_files.get(0).map(|uri| {
            log::info!("selected file uri: {:?}", uri);
            api.take_persistable_uri_permission(uri).unwrap_or_else(|e| {
                log::error!("could not take persistable uri permission: {}", e);
            });

            api.get_name(uri).map(|name| {
                log::info!("file name: {}", name);
                app.emit("file-get", name)
                    .unwrap_or_else(|e| {
                        log::error!("could not emit file-selected event: {}", e)
                    });
            }).unwrap_or_else(|e| {
                log::error!("could not get file name: {}", e);
            });
            
            match api.open_file(&uri, FileAccessMode::ReadWrite) {
                Ok(mut file) => {
                    log::info!("file opened successfully");
                    parse_log_file_and_emit_result(&mut file, &app);
                }
                Err(e) => {
                    log::error!("could not open file: {}", e);
                }
            };
                
        });
    }
    Ok(())
}

fn parse_log_file_and_emit_result(file: &mut File, app: &tauri::AppHandle) {
    match inner_parse_header_and_extra(file) {
        Ok(header_and_extra) => {
            let header = header_and_extra.0;
            let extra = header_and_extra.1.unwrap_or_default();
            if header.is_encrypt() {
                log::info!("extra: {:?}", extra);
                let state: tauri::State<AppState> = match app.try_state() {
                    Some(state) => state,
                    None => return,
                };
                let _ = state
                    .data
                    .try_lock()
                    .map(|mut data| {
                        file.try_clone()
                            .map(|f| {
                                *data = Some(f);
                            })
                            .unwrap_or_else(|e| {
                                log::error!("could not clone file: {}", e);
                            });
                    })
                    .inspect_err(|e| log::error!("lock error {e}"));

                let json = json!({"header": header, "extra": extra.0, "extra_encode": extra.1})
                    .to_string();
                app.emit("header-parsed", json)
                    .unwrap_or_else(|e| log::error!("could not emit file-selected event: {}", e));
            } else {
                log::info!("parse records");
                match inner_parse_log_file(&file, "", "") {
                    Ok(records) => {
                        app.emit("records-parsed", records).unwrap_or_else(|e| {
                            log::error!("could not emit file-selected event: {}", e)
                        });
                    }
                    Err(e) => {
                        toast_error(app, &format!("could not parse log file: {}", e));
                    }
                }
            }
        }
        Err(e) => {
            toast_error(app, &format!("{}", e));
        }
    }
}

#[derive(Clone, Serialize)]
struct ToastPayload<'a> {
  message: &'a str,
  #[serde(rename = "type")]
  message_type: &'a str,
}

enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl Into<&'static str> for ToastLevel {
    fn into(self) -> &'static str {
        match self {
            ToastLevel::Info => "info",
            ToastLevel::Success => "success",
            ToastLevel::Warning => "warning",
            ToastLevel::Error => "error",
        }
    }
}

impl <'a> From<&'a str> for ToastLevel {
    fn from(level: &'a str) -> Self {
        match level {
            "info" => ToastLevel::Info,
            "success" => ToastLevel::Success,
            "warning" => ToastLevel::Warning,
            "error" => ToastLevel::Error,
            _ => ToastLevel::Info, // Default to Info if unknown
        }
    }    
}

pub(crate) fn toast_error(app: &tauri::AppHandle, message: &str) {
    toast(app, message, ToastLevel::Error)
}

pub(crate) fn toast(
    app: &tauri::AppHandle,
    message: &str,
    message_type: ToastLevel,
) {
    match message_type {
        ToastLevel::Info => log::info!("toast: {}", message),
        ToastLevel::Success => log::info!("toast: {}", message),
        ToastLevel::Warning => log::warn!("toast: {}", message),
        ToastLevel::Error => log::error!("toast: {}", message),
    }
    let payload = ToastPayload {
        message,
        message_type: message_type.into(),
    };
    app.emit("toast", payload)
        .unwrap_or_else(|e| log::error!("could not emit toast event: {}", e))
}
