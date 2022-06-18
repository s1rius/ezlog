use std::{mem::MaybeUninit, sync::Once};

#[allow(non_snake_case)]
use crate::{
    thread_name, CipherKind, CompressKind, CompressLevel, EZLogConfigBuilder, EZRecordBuilder,
    Level,
};
use android_logger::Config;
use jni::{
    errors::JniError,
    objects::{GlobalRef, JClass, JMethodID, JObject, JString, JValue},
    signature::Primitive,
    strings::JNIString,
    sys::{jbyteArray, jint, jobjectArray, JNI_VERSION_1_6},
    JNIEnv, JavaVM,
};
use libc::c_void;
use log::debug;
use time::Duration;

static ONLOAD: Once = Once::new();
static mut JVM: MaybeUninit<JavaVM> = MaybeUninit::uninit();
static mut ON_FETCH_SUCCESS_METHOD: Option<JMethodID> = None;
static mut ON_FETCH_FAIL_METHOD: Option<JMethodID> = None;
static mut CALL_BACK: Option<JObject> = None;

#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _: *mut c_void) -> jint {
    ONLOAD.call_once(|| {
        let env = &vm.get_env().expect("Cannot get reference to the JNIEnv");
        unsafe {
            ON_FETCH_SUCCESS_METHOD = get_method_id(
                &env,
                "wtf/s1/ezlog/Callback",
                "onLogFetch",
                "(Ljava/lang/String,[Ljava/lang/String)V",
            );
            JVM.as_mut_ptr().write(vm);
        }
    });

    JNI_VERSION_1_6
}

#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_init(_: JNIEnv, _: JClass) {
    android_logger::init_once(
        Config::default()
            .with_min_level(log::Level::Trace)
            .with_tag("ezlog"), // logs will show under mytag tag
    );
    crate::init();
    debug!("ezlog_init");
}

#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_createLogger(
    env: JNIEnv,
    _jclass: JClass,
    j_log_name: JString,
    j_level: jint,
    j_dir_path: JString,
    j_keep_days: jint,
    j_compress: jint,
    j_compress_level: jint,
    j_cipher: jint,
    j_cipher_key: jbyteArray,
    j_cipher_nonce: jbyteArray,
) {
    let log_name: String = env.get_string(j_log_name).unwrap().into();
    let log_level: Level = Level::from_usize(j_level as usize).unwrap_or(Level::Trace);
    let log_dir = env
        .get_string(j_dir_path)
        .map(|dir| dir.into())
        .unwrap_or_else(|_| "".to_string());
    let duration: Duration = Duration::days(j_keep_days as i64);
    let compress: CompressKind = CompressKind::from(j_compress as u8);
    let compress_level: CompressLevel = CompressLevel::from(j_compress_level as u8);
    let cipher: CipherKind = CipherKind::from(j_cipher as u8);
    let cipher_key = env.convert_byte_array(j_cipher_key).unwrap_or_default();
    let cipher_nonce = env.convert_byte_array(j_cipher_nonce).unwrap_or_default();

    let config = EZLogConfigBuilder::new()
        .name(log_name)
        .level(log_level)
        .dir_path(log_dir)
        .duration(duration)
        .compress(compress)
        .compress_level(compress_level)
        .cipher(cipher)
        .cipher_key(cipher_key)
        .cipher_nonce(cipher_nonce)
        .build();

    crate::create_log(config);
}

#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_log(
    env: JNIEnv,
    _jclass: JClass,
    j_log_name: JString,
    j_level: jint,
    j_target: JString,
    j_content: JString,
) {
    let log_name: String = env
        .get_string(j_log_name)
        .map(|name| name.into())
        .unwrap_or_else(|_| "".to_string());

    let log_level: Level = Level::from_usize(j_level as usize).unwrap_or(Level::Trace);

    let target = env
        .get_string(j_target)
        .map(|jstr| jstr.into())
        .unwrap_or_else(|_| "".to_string());

    let content = env
        .get_string(j_content)
        .map(|jstr| jstr.into())
        .unwrap_or_else(|_| "".to_string());

    let record = EZRecordBuilder::new()
        .log_name(log_name)
        .level(log_level)
        .target(target)
        .content(content)
        .thread_id(thread_id::get())
        .thread_name(thread_name::get())
        .build();

    crate::log(record);
}

#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_flushAll(_: JNIEnv, _: JClass) {
    crate::flush_all();
}

#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_flush(
    env: JNIEnv,
    _: JClass,
    j_log_name: JString,
) {
    let log_name: String = env
        .get_string(j_log_name)
        .map(|name| name.into())
        .unwrap_or_else(|_| "".to_string());
    crate::flush(&log_name);
}

#[no_mangle]
pub unsafe extern "C" fn Java_wft_s1_ezlog_EZLog_fetchLog(
    env: JNIEnv,
    _: JClass,
    j_log_name: JString,
    j_date: JString,
) {
    let log_name: String = env
        .get_string(j_log_name)
        .map(|name| name.into())
        .unwrap_or_else(|_| "".to_string());

    let date = env
        .get_string(j_date)
        .map(|jstr| jstr.into())
        .unwrap_or_else(|_| "".to_string());

    crate::request_log_files_for_date(&log_name, &date).unwrap_or_else(|e| {
        let err = env
            .new_string(&format!("{}", e))
            .unwrap_or_else(|_| j_log_name);

        internal_call_on_fetch_fail(&env, j_log_name, j_date, err).unwrap_or_else(|_| {
            //todo
        });
    });
}

/// Produces `JMethodID` for a particular method dealing with its lifetime.
///
/// Always returns `Some(method_id)`, panics if method not found.
#[inline]
fn get_method_id(env: &JNIEnv, class: &str, name: &str, sig: &str) -> Option<JMethodID<'static>> {
    let method_id = env
        .get_method_id(class, name, sig)
        // we need this line to erase lifetime in order to save underlying raw pointer in static
        .map(|mid| mid.into_inner().into())
        .unwrap_or_else(|_| {
            panic!(
                "Method {} with signature {} of class {} not found",
                name, sig, class
            )
        });
    Some(method_id)
}

/// Returns cached class reference.
///
/// Always returns Some(class_ref), panics if class not found.
#[inline]
fn get_class(env: &JNIEnv, class: &str) -> Option<GlobalRef> {
    let class = env
        .find_class(class)
        .unwrap_or_else(|_| panic!("Class {} not found", class));
    Some(env.new_global_ref(class).unwrap())
}

pub(crate) fn call_on_fetch_success(
    name: &str,
    date: &str,
    logs: Vec<&str>,
) -> Result<(), jni::errors::Error> {
    unsafe {
        match get_env() {
            Ok(env) => {
                let name = env.new_string(name)?;
                let date = env.new_string(date)?;
                let j_logs = vec_to_jobjectArray(
                    &env,
                    logs,
                    "Ljava/lang/String",
                    |x| env.new_string(x),
                    env.new_string("")?,
                )?;

                let args: &[JValue] = &[name.into(), date.into(), j_logs.into()];
                if let Some(method) = ON_FETCH_SUCCESS_METHOD {
                    if let Some(callback) = CALL_BACK {
                        env.call_method_unchecked(
                            callback,
                            method,
                            jni::signature::JavaType::Primitive(Primitive::Void),
                            &args,
                        )?;
                    }
                }
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

pub(crate) fn call_on_fetch_fail(
    name: &str,
    date: &str,
    err_msg: &str,
) -> Result<(), jni::errors::Error> {
    unsafe {
        match get_env() {
            Ok(env) => {
                let name = env.new_string(name)?;
                let date = env.new_string(date)?;
                let err = env.new_string(err_msg)?;

                internal_call_on_fetch_fail(&env, name, date, err)
            }
            Err(e) => Err(e),
        }
    }
}

unsafe fn internal_call_on_fetch_fail<'a>(
    env: &JNIEnv<'a>,
    name: JString,
    date: JString,
    err: JString,
) -> Result<(), jni::errors::Error> {
    let args: &[JValue] = &[name.into(), date.into(), err.into()];
    if let Some(method) = ON_FETCH_FAIL_METHOD {
        if let Some(callback) = CALL_BACK {
            env.call_method_unchecked(
                callback,
                method,
                jni::signature::JavaType::Primitive(Primitive::Void),
                &args,
            )?;
        }
    }
    Ok(())
}

#[inline]
unsafe fn get_env<'a>() -> Result<JNIEnv<'a>, jni::errors::Error> {
    if ONLOAD.is_completed() {
        let jvm = &*(JVM.as_ptr());
        match jvm.get_env() {
            Ok(env) => {
                return Ok(env);
            }
            Err(err) => match err {
                jni::errors::Error::JniCall(e) => match e {
                    jni::errors::JniError::ThreadDetached => {
                        let env = *jvm.attach_current_thread()?;
                        return Ok(env);
                    }
                    _e => return Err(jni::errors::Error::JniCall(_e)),
                },
                _e => return Err(_e),
            },
        }
    }
    Err(jni::errors::Error::JniCall(JniError::Unknown))
}

#[inline]
unsafe fn vec_to_jobjectArray<'a, T, U, F, C>(
    env: &JNIEnv<'a>,
    vec: Vec<T>,
    element_class_name: C,
    op: F,
    initial_element: U,
) -> Result<jobjectArray, jni::errors::Error>
where
    C: Into<JNIString>,
    F: Fn(&T) -> Result<U, jni::errors::Error>,
    U: Into<JObject<'a>>,
{
    let cls_arraylist = env.find_class(element_class_name)?;
    let jobjArray = env.new_object_array(vec.len() as i32, cls_arraylist, initial_element)?;
    for (i, log) in vec.iter().enumerate() {
        env.set_object_array_element(jobjArray, i as i32, op(log)?)?;
    }
    Ok(jobjArray)
}

mod tests {
    use std::{ffi::CString, path::PathBuf};

    use jni::JNIEnv;

    fn test_jni_path<'a>(env: &JNIEnv<'a>, p: PathBuf) {
        let a = p.to_str().unwrap();
        env.new_string(a).unwrap();
    }
}
