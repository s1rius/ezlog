use crate::{
    event, events::EventPrinter, set_boxed_callback, set_event_listener, thread_name, CipherKind,
    CompressKind, CompressLevel, EZLogConfigBuilder, EZRecordBuilder, Level,
};
use jni::{
    errors::JniError,
    objects::{GlobalRef, JClass, JMethodID, JObject, JString, JValue},
    signature::Primitive,
    strings::JNIString,
    sys::{jboolean, jbyteArray, jint, jobject, jobjectArray, jvalue, JNI_VERSION_1_6},
    JNIEnv, JavaVM, AttachGuard,
};
use libc::c_void;
use once_cell::sync::OnceCell;
use time::Duration;

static JVM: OnceCell<JavaVM> = OnceCell::new();
static CALL_BACK_CLASS: OnceCell<GlobalRef> = OnceCell::new();

static EVENT_LISTENER: EventPrinter = EventPrinter {};

#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _: *mut c_void) -> jint {
    if let Ok(env) = vm.get_env() {
        if let Some(c) = get_class(&env, "wtf/s1/ezlog/EZLogCallback") {
            CALL_BACK_CLASS
                .set(c)
                .map_err(|_| event!(ffi_call_err "find callback err"))
                .unwrap_or(());
        }
    }

    JVM.set(vm)
        .map_err(|_| event!(ffi_call_err "set jvm error"))
        .unwrap_or(());
    JNI_VERSION_1_6
}

#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_init(
    _: JNIEnv,
    _: JClass,
    j_enable_trace: jboolean,
) {
    let enable_trace = j_enable_trace as u8;
    if enable_trace > 0 {
        set_event_listener(&EVENT_LISTENER);
    }
    crate::init();
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
    let log_name: String = env
        .get_string(j_log_name)
        .map(|s| s.into())
        .unwrap_or_default();
    let log_level: Level = Level::from_usize(j_level as usize).unwrap_or(Level::Trace);
    let log_dir = env
        .get_string(j_dir_path)
        .map(|dir| dir.into())
        .unwrap_or_default();
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

    if !config.is_valid() {
        event!(ffi_call_err & format!("create logger config error {config:?}"));
        return;
    }

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
        .unwrap_or_default();

    let log_level: Level = Level::from_usize(j_level as usize).unwrap_or(Level::Trace);

    let target = env
        .get_string(j_target)
        .map(|jstr| jstr.into())
        .unwrap_or_default();

    let content = env
        .get_string(j_content)
        .map(|jstr| jstr.into())
        .unwrap_or_default();

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
        .unwrap_or_default();
    crate::flush(&log_name);
}

#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_trim(_env: JNIEnv, _: JClass) {
    crate::trim();
}

// todo thread safe
#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_registerCallback(
    env: JNIEnv,
    _jclass: JClass,
    j_callback: JObject,
) {
    match env.new_global_ref(j_callback) {
        Ok(gloableCallback) => {
            set_boxed_callback(Box::new(AndroidCallback::new(&env, gloableCallback)));
        }
        Err(e) => event!(ffi_call_err & format!("register callback error: {}", e)),
    }
}

#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_requestLogFilesForDate(
    env: JNIEnv,
    _: JClass,
    j_log_name: JString,
    j_date: JString,
) {
    let log_name: String = env
        .get_string(j_log_name)
        .map(|name| name.into())
        .unwrap_or_default();

    let date: String = env
        .get_string(j_date)
        .map(|jstr| jstr.into())
        .unwrap_or_default();

    crate::request_log_files_for_date(&log_name, &date);
}

/// Produces `JMethodID` for a particular method dealing with its lifetime.
///
/// Always returns `Some(method_id)`, panics if method not found.
#[inline]
fn get_method_id(env: &JNIEnv, class: &str, name: &str, sig: &str) -> Option<JMethodID> {
    let method_id = env
        .get_method_id(class, name, sig)
        // we need this line to erase lifetime in order to save underlying raw pointer in static
        .map(|mid| mid.into())
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
    env.new_global_ref(class).map_err(ffi_fail).ok()
}

struct AndroidCallback {
    callback: GlobalRef,
    fail_method_id: JMethodID,
    success_method_id: JMethodID,
}

impl AndroidCallback {
    fn new(env: &JNIEnv, callback: GlobalRef) -> Self {
        let fail_method_id = get_method_id(
            env,
            "wtf/s1/ezlog/EZLogCallback",
            "onFail",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
        )
        .unwrap();
        let success_method_id = get_method_id(
            env,
            "wtf/s1/ezlog/EZLogCallback",
            "onSuccess",
            "(Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;)V",
        )
        .unwrap();
        Self {
            callback,
            fail_method_id,
            success_method_id,
        }
    }

    fn internal_fetch_success(
        &self,
        name: &str,
        date: &str,
        logs: &[&str],
    ) -> Result<(), jni::errors::Error> {
        unsafe {
            match get_env() {
                Ok(env) => {
                    let name = env.new_string(name)?;
                    let date = env.new_string(date)?;
                    let j_logs: jobject = vec_to_jobjectArray(
                        &env,
                        logs,
                        "java/lang/String",
                        |x| env.new_string(x),
                        env.new_string("")?,
                    )?;
                    let args: &[JValue] =
                        &[name.into(), date.into(), JObject::from_raw(j_logs).into()];
                    let args: Vec<jvalue> = args.iter().map(|v| v.to_jni()).collect();
                    env.call_method_unchecked(
                        &self.callback,
                        self.success_method_id,
                        jni::signature::ReturnType::Primitive(Primitive::Void),
                        &args,
                    )?;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    }

    #[inline]
    fn internal_fetch_fail(
        &self,
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

                    let args: &[JValue] = &[name.into(), date.into(), err.into()];
                    let args: Vec<jvalue> = args.iter().map(|v| v.to_jni()).collect();

                    env.call_method_unchecked(
                        &self.callback,
                        self.fail_method_id,
                        jni::signature::ReturnType::Primitive(Primitive::Void),
                        &args,
                    )?;

                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
    }
}

impl crate::EZLogCallback for AndroidCallback {
    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]) {
        self.internal_fetch_success(name, date, logs)
            .unwrap_or_else(ffi_fail);
    }

    fn on_fetch_fail(&self, name: &str, date: &str, err: &str) {
        self.internal_fetch_fail(name, date, err)
            .unwrap_or_else(ffi_fail);
    }
}

#[inline]
unsafe fn get_env<'a>() -> Result<AttachGuard<'a>, jni::errors::Error> {
    if let Some(jvm) = JVM.get() {
        return jvm.attach_current_thread();
    }
    Err(jni::errors::Error::JniCall(JniError::Unknown))
}

#[inline]
unsafe fn vec_to_jobjectArray<'a, T, C, F, U>(
    env: &JNIEnv<'a>,
    vec: &[T],
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

#[inline]
fn ffi_fail(e: jni::errors::Error) {
    event!(ffi_call_err & e.to_string());
}

mod tests {}
