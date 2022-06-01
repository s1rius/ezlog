#[allow(non_snake_case)]

use crate::{
    CipherKind, CompressKind, CompressLevel, EZLogConfigBuilder, EZRecordBuilder, Level,
};
use jni::{
    objects::{JClass, JString},
    sys::{jbyteArray, jint},
    JNIEnv,
};
use time::Duration;

#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_init(_: JNIEnv, _: JClass) {
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
        .unwrap()
        .into();
    let log_level: Level =
        Level::from_usize(j_level as usize).unwrap_or_else(|| Level::Trace);
    let log_dir: String = env
        .get_string(j_dir_path)
        .expect("Couldn't get dir path")
        .into();
    let duration: Duration = Duration::days(j_keep_days as i64);
    let compress: CompressKind = CompressKind::from(j_compress as u8);
    let compress_level: CompressLevel = CompressLevel::from(j_compress_level as u8);
    let cipher: CipherKind = CipherKind::from(j_cipher as u8);
    let cipher_key = env
        .convert_byte_array(j_cipher_key)
        .expect("Couldn't get cipher key");
    let cipher_nonce = env
        .convert_byte_array(j_cipher_nonce)
        .expect("Couldn't get nonce");

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
        .expect("Couldn't get java string!")
        .into();

    let log_level: Level =
        Level::from_usize(j_level as usize).expect("Couldn't get java int level");

    let target: String = env
        .get_string(j_target)
        .expect("Couldn't get java string!")
        .into();

    let content: String = env
        .get_string(j_content)
        .expect("Couldn't get java string!")
        .into();

    let record = EZRecordBuilder::new()
        .log_name(log_name)
        .level(log_level)
        .target(target)
        .content(content)
        .build();

    crate::log(record);
}

#[no_mangle]
pub unsafe extern "C" fn Java_wtf_s1_ezlog_EZLog_flushAll(_: JNIEnv, _: JClass) {
    crate::flush_all();
}