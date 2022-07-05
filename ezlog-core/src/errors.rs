use std::io;

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
    #[error("illegal argument {0}")]
    IllegalArgument(String),
}

impl From<io::Error> for LogError {
    fn from(err: io::Error) -> LogError {
        LogError::IoError(err)
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
