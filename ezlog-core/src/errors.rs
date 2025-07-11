use std::{
    ffi::NulError,
    io,
    sync::PoisonError,
};

use crossbeam_channel::{
    RecvError,
    TrySendError,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LogError {
    #[error("io error: {0}")]
    IoError(#[source] io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("compress error: {0}")]
    Compress(#[source] io::Error),
    #[error("illegal argument or state {0}")]
    Illegal(String),
    #[error("ffi error: {0}")]
    FFI(String),
    #[error("unknown error: {0}")]
    Unknown(String),
    #[error("log init error")]
    NotInit,
    #[error("{0}")]
    Poison(String),
}

impl LogError {
    pub fn unknown(error: impl Into<String>) -> Self {
        LogError::Unknown(error.into())
    }
}

impl From<io::Error> for LogError {
    fn from(err: io::Error) -> LogError {
        LogError::IoError(err)
    }
}

impl From<NulError> for LogError {
    fn from(e: NulError) -> Self {
        LogError::FFI(format!("{:?}", e))
    }
}

#[cfg(target_os = "android")]
impl From<jni::errors::Error> for LogError {
    fn from(e: jni::errors::Error) -> Self {
        LogError::FFI(format!("{:?}", e))
    }
}

impl From<RecvError> for LogError {
    fn from(e: RecvError) -> Self {
        LogError::Unknown(format!("{:?}", e))
    }
}

impl<T> From<TrySendError<T>> for LogError {
    fn from(e: TrySendError<T>) -> Self {
        LogError::Illegal(e.to_string())
    }
}

impl<T> From<PoisonError<T>> for LogError {
    fn from(e: PoisonError<T>) -> Self {
        LogError::Poison(format!("{:?}", e))
    }
}

mod tests {

    #[test]
    fn test_error() {
        use std::io;

        use crate::errors::LogError;

        let err = LogError::IoError(io::Error::other("test"));
        assert_eq!(err.to_string(), "io error: test");
    }
}
