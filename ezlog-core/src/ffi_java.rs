use std::sync::Arc;

use jni::objects::{
    JByteArray,
    JObjectArray,
    JValueGen,
};
use jni::sys::jlong;
use jni::{
    errors::JniError,
    objects::{
        GlobalRef,
        JClass,
        JObject,
        JString,
        JValue,
    },
    strings::JNIString,
    sys::{
        jboolean,
        jint,
        JNI_VERSION_1_6,
    },
    JNIEnv,
    JavaVM,
};
use libc::c_void;
use once_cell::sync::OnceCell;
use time::{
    Duration,
    OffsetDateTime,
};

use crate::errors::LogError;
use crate::events::Event::{
    self,
    *,
};
use crate::{
    event,
    set_boxed_callback,
    thread_name,
    CipherKind,
    CompressKind,
    CompressLevel,
    EZLogConfigBuilder,
    EZRecordBuilder,
    Level,
};

static JVM: OnceCell<Arc<JavaVM>> = OnceCell::new();

#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _: *mut c_void) -> jint {
    JVM.set(Arc::new(vm))
        .map_err(|_| event!(Event::FFIError, "set jvm error"))
        .unwrap_or(());
    JNI_VERSION_1_6
}

#[no_mangle]
pub extern "C" fn Java_wtf_s1_ezlog_EZLog_nativeInit(
    _: JNIEnv<'_>,
    _: JClass,
    j_enable_trace: jboolean,
) {
    let enable_trace = j_enable_trace;
    crate::InitBuilder::new().debug(enable_trace > 0).init();
}

#[no_mangle]
pub extern "C" fn Java_wtf_s1_ezlog_EZLog_nativeCreateLogger<'local>(
    mut env: JNIEnv<'local>,
    _jclass: JClass<'local>,
    j_log_name: JString<'local>,
    j_level: jint,
    j_dir_path: JString<'local>,
    j_keep_days: jint,
    j_compress: jint,
    j_compress_level: jint,
    j_cipher: jint,
    j_cipher_key: JByteArray<'local>,
    j_cipher_nonce: JByteArray<'local>,
    j_rotate_hours: jint,
    j_extra: JString<'local>,
) {
    let log_name: String = env
        .get_string(&j_log_name)
        .map(|s| s.into())
        .unwrap_or_default();
    let log_level: Level = Level::from_usize(j_level as usize).unwrap_or(Level::Trace);
    let log_dir: String = env
        .get_string(&j_dir_path)
        .map(|dir| dir.into())
        .unwrap_or_default();
    let duration: Duration = Duration::days(j_keep_days as i64);
    let rotate_duration: Duration = Duration::hours(j_rotate_hours as i64);
    let compress: CompressKind = CompressKind::from(j_compress as u8);
    let compress_level: CompressLevel = CompressLevel::from(j_compress_level as u8);
    let cipher: CipherKind = CipherKind::from(j_cipher as u8);
    let cipher_key = &env.convert_byte_array(j_cipher_key).unwrap_or_default();
    let cipher_nonce = &env.convert_byte_array(j_cipher_nonce).unwrap_or_default();
    let extra: String = env
        .get_string(&j_extra)
        .map(|s| s.into())
        .unwrap_or_default();

    let config = EZLogConfigBuilder::new()
        .name(log_name)
        .level(log_level)
        .dir_path(log_dir)
        .trim_duration(duration)
        .rotate_duration(rotate_duration)
        .compress(compress)
        .compress_level(compress_level)
        .cipher(cipher)
        .cipher_key(cipher_key.to_owned())
        .cipher_nonce(cipher_nonce.to_owned())
        .extra(extra)
        .build();

    if !config.is_valid() {
        event!(
            CreateLoggerError,
            "create logger config error, config: {:?}",
            config
        );
        return;
    }

    crate::create_log(config);
}

#[no_mangle]
pub extern "C" fn Java_wtf_s1_ezlog_EZLog_nativeLog<'local>(
    mut env: JNIEnv<'local>,
    _jclass: JClass,
    j_log_name: JString<'local>,
    j_level: jint,
    j_target: JString<'local>,
    j_content: JString<'local>,
) {
    let log_name: String = env
        .get_string(&j_log_name)
        .map(|name| name.into())
        .unwrap_or_default();

    let log_level: Level = Level::from_usize(j_level as usize).unwrap_or(Level::Trace);

    let target: String = env
        .get_string(&j_target)
        .map(|jstr| jstr.into())
        .unwrap_or_default();

    let content: String = env
        .get_string(&j_content)
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
pub extern "C" fn Java_wtf_s1_ezlog_EZLog_nativeFlushAll(_: JNIEnv, _: JClass) {
    crate::flush_all();
}

#[no_mangle]
pub extern "C" fn Java_wtf_s1_ezlog_EZLog_nativeFlush(
    mut env: JNIEnv,
    _: JClass,
    j_log_name: JString,
) {
    let log_name: String = env
        .get_string(&j_log_name)
        .map(|name| name.into())
        .unwrap_or_default();
    crate::flush(&log_name);
}

#[no_mangle]
pub extern "C" fn Java_wtf_s1_ezlog_EZLog_nativeTrim(_env: JNIEnv, _: JClass) {
    crate::trim();
}

// TODO: thread safe
#[no_mangle]
pub extern "C" fn Java_wtf_s1_ezlog_EZLog_nativeRegisterCallback(
    env: JNIEnv,
    _jclass: JClass,
    j_callback: JObject,
) {
    match env.new_global_ref(j_callback) {
        Ok(gloableCallback) => {
            set_boxed_callback(Box::new(AndroidCallback::new(gloableCallback)));
        }
        Err(e) => event!(
            !Event::FFIError,
            "register callback error";
            &LogError::FFI(e.to_string())
        ),
    }
}

#[no_mangle]
pub extern "C" fn Java_wtf_s1_ezlog_EZLog_nativeRequestLogFilesForDate(
    mut env: JNIEnv,
    _: JClass,
    j_log_name: JString,
    j_start: jlong,
    j_end: jlong,
) {
    let log_name: String = env
        .get_string(&j_log_name)
        .map(|name| name.into())
        .unwrap_or_default();

    let start = match OffsetDateTime::from_unix_timestamp_nanos((j_start as i128) * 1_000_000) {
        Ok(time) => time,
        Err(_) => {
            event!(Event::RequestLogError, "start time illegal {}", j_start);
            return;
        }
    };
    let end = match OffsetDateTime::from_unix_timestamp_nanos((j_end as i128) * 1_000_000) {
        Ok(time) => time,
        Err(_) => {
            event!(Event::RequestLogError, "end time illegal {}", j_end);
            return;
        }
    };

    crate::request_log_files_for_date(&log_name, start, end);
}

struct AndroidCallback {
    callback: GlobalRef,
}

impl AndroidCallback {
    fn new(callback: GlobalRef) -> Self {
        Self { callback }
    }

    fn internal_fetch_success(
        &self,
        name: &str,
        date: &str,
        logs: &[&str],
    ) -> Result<(), jni::errors::Error> {
        match get_env() {
            Ok(mut env) => {
                let name = &env.new_string(name)?;
                let date = &env.new_string(date)?;
                let log = env.new_string("")?;
                let j_logs: JObjectArray<'_> = vec_to_jobjectArray(
                    &mut env,
                    logs,
                    "java/lang/String",
                    |x, env| env.new_string(x),
                    log,
                )?;
                let args: [JValue; 3] = [
                    JValueGen::Object(name),
                    JValueGen::Object(date),
                    JValueGen::Object(&j_logs),
                ];

                env.call_method(
                    self.callback.as_obj(),
                    "onSuccess",
                    "(Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;)V",
                    &args,
                )?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    #[inline]
    fn internal_fetch_fail(
        &self,
        name: &str,
        date: &str,
        err_msg: &str,
    ) -> Result<(), jni::errors::Error> {
        match get_env() {
            Ok(mut env) => {
                let name = env.new_string(name)?;
                let date = env.new_string(date)?;
                let err_msg = env.new_string(err_msg)?;

                let args: [JValue; 3] = [
                    JValueGen::Object(&name),
                    JValueGen::Object(&date),
                    JValueGen::Object(&err_msg),
                ];

                env.call_method(
                    &self.callback,
                    "onFail",
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
                    &args,
                )?;

                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

impl crate::EZLogCallback for AndroidCallback {
    fn on_fetch_success(&self, name: &str, date: &str, logs: &[&str]) {
        self.internal_fetch_success(name, date, logs)
            .unwrap_or_else(|e| event!(!Event::FFIError, "on fetch success"; &e.into()));
    }

    fn on_fetch_fail(&self, name: &str, date: &str, err: &str) {
        self.internal_fetch_fail(name, date, err)
            .unwrap_or_else(|e| event!(!Event::FFIError, "on fetch fail"; &e.into()));
    }
}

#[inline]
fn get_env<'a>() -> Result<jni::AttachGuard<'a>, jni::errors::Error> {
    if let Some(jvm) = JVM.get() {
        return jvm.attach_current_thread();
    }
    Err(jni::errors::Error::JniCall(JniError::Unknown))
}

#[inline]
fn vec_to_jobjectArray<'a, T, C, F, U>(
    env: &mut JNIEnv<'a>,
    vec: &[T],
    element_class_name: C,
    op: F,
    initial_element: U,
) -> Result<JObjectArray<'a>, jni::errors::Error>
where
    C: Into<JNIString>,
    F: Fn(&T, &JNIEnv<'a>) -> Result<U, jni::errors::Error>,
    U: AsRef<JObject<'a>>,
{
    let cls_arraylist = env.find_class(element_class_name)?;
    let jobjArray = env.new_object_array(vec.len() as i32, cls_arraylist, initial_element)?;
    for (i, log) in vec.iter().enumerate() {
        env.set_object_array_element(&jobjArray, i as i32, op(log, env)?)?;
    }
    Ok(jobjArray)
}
