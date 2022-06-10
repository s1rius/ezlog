use crate::*;
use core::ffi::c_char;
use core::ffi::c_uchar;
use core::ffi::c_uint;
use core::ffi::CStr;
use core::slice;

/// init
#[no_mangle]
pub extern "C" fn ezlog_init() {
    crate::init();
}

/// # Safety
///
#[no_mangle]
pub unsafe extern "C" fn ezlog_flush(c_log_name: *const c_char) {
    let name: String = CStr::from_ptr(c_log_name).to_string_lossy().into_owned();
    crate::flush(&name);
}

/// # Safety
///
#[no_mangle]
pub extern "C" fn ezlog_flush_all() {
    crate::flush_all();
}

/// # Safety
///
#[no_mangle]
pub unsafe extern "C" fn ezlog_create_log(
    c_log_name: *const c_char,
    c_level: c_uchar,
    c_dir_path: *const c_char,
    c_keep_days: c_uint,
    c_compress: c_uchar,
    c_compress_level: c_uchar,
    c_cipher: c_uchar,
    c_cipher_key: *const c_uchar,
    c_key_len: usize,
    c_cipher_nonce: *const c_uchar,
    c_nonce_len: usize,
) {
    let log_name = CStr::from_ptr(c_log_name).to_string_lossy().into_owned();
    let level = Level::from_usize(c_level as usize).unwrap_or(Level::Trace);
    let dir_path = CStr::from_ptr(c_dir_path).to_string_lossy().into_owned();
    let duration = Duration::days(c_keep_days as i64);
    let compress = CompressKind::from(c_compress);
    let compress_level = CompressLevel::from(c_compress_level);
    let cipher = CipherKind::from(c_cipher);
    let key_bytes = slice::from_raw_parts(c_cipher_key, c_key_len);
    let cipher_key: Vec<u8> = Vec::from(key_bytes);
    let nonce_bytes = slice::from_raw_parts(c_cipher_nonce, c_nonce_len);
    let cipher_nonce: Vec<u8> = Vec::from(nonce_bytes);

    let config = EZLogConfigBuilder::new()
        .name(log_name)
        .dir_path(dir_path)
        .level(level)
        .duration(duration)
        .compress(compress)
        .compress_level(compress_level)
        .cipher(cipher)
        .cipher_key(cipher_key)
        .cipher_nonce(cipher_nonce)
        .build();

    create_log(config);
}

/// # Safety
///
#[no_mangle]
pub unsafe extern "C" fn ezlog_log(
    c_log_name: *const c_char,
    c_level: c_uchar,
    c_target: *const c_char,
    c_content: *const c_char,
) {
    let log_name = CStr::from_ptr(c_log_name).to_string_lossy().into_owned();
    let level = Level::from_usize(c_level as usize).unwrap_or(Level::Trace);
    let target = CStr::from_ptr(c_target).to_string_lossy().into_owned();
    let content = CStr::from_ptr(c_content).to_string_lossy().into_owned();
    let record = EZRecordBuilder::new()
        .log_name(log_name)
        .level(level)
        .target(target)
        .content(content)
        .build();
    log(record)
}
