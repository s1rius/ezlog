use std::{ffi::NulError, io};

use crossbeam_channel::{RecvError, TrySendError};
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
    FFi(String),
    #[error("unknown error: {0}")]
    Unknown(String),
    #[error("log init error")]
    NotInit,
}

impl LogError {
    pub fn unknown(error: &str) -> Self {
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
        LogError::FFi(format!("{:?}", e))
    }
}

#[cfg(target_os = "android")]
impl From<jni::errors::Error> for LogError {
    fn from(e: jni::errors::Error) -> Self {
        LogError::FFi(format!("{:?}", e))
    }
}

impl From<RecvError> for LogError {
    fn from(e: RecvError) -> Self {
        LogError::Unknown(format!("{:?}", e))
    }
}

impl<T> From<TrySendError<T>> for LogError {
    fn from(_: TrySendError<T>) -> Self {
        todo!()
    }
}

mod tests {

    #[test]
    fn test_error() {
        use crate::errors::LogError;
        use std::io;

        let err = LogError::IoError(io::Error::new(io::ErrorKind::Other, "test"));
        assert_eq!(err.to_string(), "io error: test");
    }
}
