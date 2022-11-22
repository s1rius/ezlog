#![warn(missing_docs)]
#![allow(dead_code)]

#[cfg(unix)]
use std::ffi::CStr;
#[cfg(unix)]
extern crate libc;

#[cfg(target_os = "redox")]
extern crate syscall;

/// Returns the name that is unique to the calling thread.
///
/// Calling this function twice from the same thread will return the same
/// name. Calling this function from a different thread will return a
/// different name.
#[inline]
pub fn get() -> String {
    get_name()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn get_name() -> String {
    use libc::c_char;

    // https://github.com/torvalds/linux/blob/master/include/uapi/linux/prctl.h#L57
    const PR_GET_NAME: libc::c_int = 16;

    let mut name = vec![0u8; 16];
    let name_ptr = name.as_mut_ptr() as *const c_char;

    // pthread wrapper only appeared in glibc 2.12, so we use syscall
    // directly.
    // https://man7.org/linux/man-pages/man2/prctl.2.html
    unsafe {
        libc::prctl(
            PR_GET_NAME,
            name_ptr,
            0 as libc::c_ulong,
            0 as libc::c_ulong,
            0 as libc::c_ulong,
        );
        CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn get_name() -> String {
    let mut name = vec![0i8; 16];
    let name_ptr = name.as_mut_ptr();
    unsafe {
        libc::pthread_getname_np(libc::pthread_self(), name_ptr, name.len());
        CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
    }
}

#[cfg(target_os = "windows")]
fn get_name() -> String {
    use std::thread;
    use windows::core::HRESULT;
    use windows::core::PWSTR;
    use windows_sys::Win32::System::Threading;

    unsafe {
        let raw = Box::into_raw(Box::new(PWSTR::null().as_ptr()));
        let hresult = HRESULT(Threading::GetThreadDescription(
            Threading::GetCurrentThread(),
            raw,
        ));
        let mut name: String = String::default();
        if hresult.is_ok() {
            let pwstr = PWSTR::from_raw(*raw);
            name = pwstr.to_string().unwrap_or_default();
        }

        if name.is_empty() {
            name = thread::current().name().unwrap_or("unknown").to_string();
        }

        let _ = Box::from_raw(raw);
        name
    }
}

#[cfg(test)]
mod tests {
    use crate::thread_name::get_name;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_thread_name() {
        use windows::core::HRESULT;
        use windows::core::HSTRING;
        use windows::core::PCWSTR;
        use windows_sys::Win32::System::Threading;

        unsafe {
            let cname = HSTRING::from("test 1234567890123456");
            let w_result = HRESULT(Threading::SetThreadDescription(
                Threading::GetCurrentThread(),
                PCWSTR::from(&cname).as_ptr(),
            ));
            assert!(w_result.is_ok());
            assert_eq!(get_name(), "test 1234567890123456".to_string());
        }
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_get_thread_name() {
        let j = std::thread::Builder::new()
            .name("test 1234567890123456".to_string())
            .spawn(|| {
                assert_eq!(get_name(), "test 1234567890");
            })
            .unwrap();
        j.join().unwrap();
    }
}
