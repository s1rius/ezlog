use libc::c_void;

use crate::*;
use core::ffi::c_char;
use core::ffi::c_uchar;
use core::ffi::c_uint;
use core::ffi::CStr;
use core::slice;
use std::ffi::CString;
use std::ffi::NulError;

static mut CALL_BACK: MaybeUninit<Callback> = MaybeUninit::uninit();
static mut ONCE_REGISTER: std::sync::Once = Once::new();

/// Init ezlog, must call before any other function
#[no_mangle]
pub extern "C" fn ezlog_init() {
    crate::init();
}

/// Flush target log which name is `c_log_name`
#[no_mangle]
pub extern "C" fn ezlog_flush(c_log_name: *const c_char) {
    unsafe {
        let name: String = CStr::from_ptr(c_log_name).to_string_lossy().into_owned();
        crate::flush(&name);
    }
}

/// Flush all logger
#[no_mangle]
pub extern "C" fn ezlog_flush_all() {
    crate::flush_all();
}

/// Create a new log wtih config options
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

/// Write log to file
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
        .thread_id(thread_id::get())
        .thread_name(thread_name::get())
        .build();
    log(record)
}

/// Register callback function for get logger's file path asynchronously
/// todo thread safe
#[no_mangle]
pub unsafe extern "C" fn ezlog_register_callback(callback: Callback) {
    ONCE_REGISTER.call_once(|| {
        CALL_BACK.as_mut_ptr().write(callback);
    });
    set_boxed_callback(Box::new(AppleCallback));
}

/// map to c Callback stuct
#[repr(C)]
pub struct Callback {
    successPoint: *mut c_void,
    onLogsFetchSuccess:
        extern "C" fn(*mut c_void, *const c_char, *const c_char, *const *const c_char, i32),
    failPoint: *mut c_void,
    onLogsFetchFail: extern "C" fn(*mut c_void, *const c_char, *const c_char, *const c_char),
}

unsafe impl Send for Callback {}

impl Callback {
    pub fn success(
        &self,
        log_name: &str,
        date: &str,
        logs: &[&str],
    ) -> std::result::Result<(), NulError> {
        let c_log_name = CString::new(log_name)?.into_raw();
        let c_target = CString::new(date)?.into_raw();
        let c_args = logs
            .iter()
            .map(|s| CString::new(*s).unwrap_or_default().into_raw())
            .collect::<Vec<_>>();
        let c_args_ptr = c_args.as_ptr();
        let c_args_len = c_args.len();

        (self.onLogsFetchSuccess)(
            self.successPoint as *mut _,
            c_log_name,
            c_target,
            c_args_ptr as *const *const i8,
            c_args_len as i32,
        );

        // release c string
        unsafe {
            let _ = CString::from_raw(c_log_name);
            let _ = CString::from_raw(c_target);

            for c_arg in c_args {
                let _ = CString::from_raw(c_arg);
            }
        }

        // todo std::mem::forget_ref(self);
        Ok(())
    }

    pub fn fail(&self, log_name: &str, date: &str, err: &str) -> std::result::Result<(), NulError> {
        let c_log_name = CString::new(log_name)?.into_raw();
        let c_date = CString::new(date)?.into_raw();
        let c_err = CString::new(err)?.into_raw();
        (self.onLogsFetchFail)(self.failPoint as *mut _, c_log_name, c_date, c_err);

        unsafe {
            let _ = CString::from_raw(c_log_name);
            let _ = CString::from_raw(c_date);
            let _ = CString::from_raw(c_err);
        }
        // todo std::mem::forget(self);
        Ok(())
    }
}

impl Drop for Callback {
    fn drop(&mut self) {
        panic!("CompletedCallback must have explicit succeeded or failed call")
    }
}

fn call_on_fetch_success(name: &str, date: &str, logs: &[&str]) {
    unsafe {
        if ONCE_REGISTER.is_completed() {
            let callback = &*CALL_BACK.as_ptr();
            callback
                .success(name, date, logs)
                .unwrap_or_else(|e| event!(ffi_call_err e));
        } else {
            event!(ffi_call_err format!("callback not registered"));
        }
    }
}

fn call_on_fetch_fail(name: &str, date: &str, err_msg: &str) {
    unsafe {
        if ONCE_REGISTER.is_completed() {
            let callback = &*CALL_BACK.as_ptr();
            callback
                .fail(name, date, err_msg)
                .unwrap_or_else(|e| event!(ffi_call_err e));
        } else {
            event!(ffi_call_err format!("callback not registered"));
        }
    }
}

/// Request logger's files path array by specified date
/// before call this function, you should register a callback
/// call
/// ```
/// ezlog_register_callback(callback);
/// ```
#[no_mangle]
pub unsafe extern "C" fn ezlog_request_log_files_for_date(
    c_log_name: *const c_char,
    c_date: *const c_char,
) {
    let log_name = CStr::from_ptr(c_log_name).to_string_lossy().into_owned();
    let date = CStr::from_ptr(c_date).to_string_lossy().into_owned();
    crate::request_log_files_for_date(&log_name, &date);
}

/// Callback impl EZLogCallback
struct AppleCallback;

impl EZLogCallback for AppleCallback {
    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]) {
        call_on_fetch_success(name, date, logs);
    }

    fn on_fetch_fail(&self, name: &str, date: &str, err_msg: &str) {
        call_on_fetch_fail(name, date, err_msg)
    }
}
