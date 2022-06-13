#[allow(non_snake_case)]
use crate::{
    thread_name, CipherKind, CompressKind, CompressLevel, EZLogConfigBuilder, EZRecordBuilder,
    Level,
};
use android_logger::Config;
use jni::{
    objects::{JClass, JString},
    sys::{jbyteArray, jint},
    JNIEnv,
};
use log::debug;
use time::Duration;

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
        .unwrap_or("".to_string());
    let duration: Duration = Duration::days(j_keep_days as i64);
    let compress: CompressKind = CompressKind::from(j_compress as u8);
    let compress_level: CompressLevel = CompressLevel::from(j_compress_level as u8);
    let cipher: CipherKind = CipherKind::from(j_cipher as u8);
    let cipher_key = env.convert_byte_array(j_cipher_key).unwrap_or(vec![]);
    let cipher_nonce = env.convert_byte_array(j_cipher_nonce).unwrap_or(vec![]);

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
        .unwrap_or("".to_string());

    let log_level: Level = Level::from_usize(j_level as usize).unwrap_or(Level::Trace);

    let target = env
        .get_string(j_target)
        .map(|jstr| jstr.into())
        .unwrap_or("".to_string());

    let content = env
        .get_string(j_content)
        .map(|jstr| jstr.into())
        .unwrap_or("".to_string());

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
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_flush(env: JNIEnv, _: JClass, j_log_name: JString) {
    let log_name: String = env
        .get_string(j_log_name)
        .map(|name| name.into())
        .unwrap_or("".to_string());
    crate::flush(&log_name);
}
