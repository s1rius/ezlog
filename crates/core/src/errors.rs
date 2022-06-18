use std::{
    error::Error,
    fmt::{self, Display},
    io,
};

use crossbeam_channel::TrySendError;

#[derive(Debug)]
pub enum LogError {
    IoError(io::Error),
    Parse(ParseError),
    Crypto(CryptoError),
    Compress(CompressError),
    IllegalArgument(IllegalArgumentError),
    State(StateError),
}

#[derive(Debug)]
pub struct CompressError {
    underlying: Option<Box<dyn Error + Send + Sync>>,
}

impl CompressError {
    /// Create an `EncodingError` that stems from an arbitrary error of an underlying encoder.
    pub fn new(err: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        CompressError {
            underlying: Some(err.into()),
        }
    }
}

impl Display for CompressError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match &self.underlying {
            Some(underlying) => write!(fmt, "compress error :\n{}", underlying,),
            None => write!(fmt, "compress error unknown"),
        }
    }
}

impl Error for CompressError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.underlying {
            None => None,
            Some(source) => Some(&**source),
        }
    }
}

impl Error for LogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LogError::Compress(err) => err.source(),
            LogError::IoError(err) => err.source(),
            LogError::Parse(err) => err.source(),
            LogError::Crypto(err) => err.source(),
            LogError::IllegalArgument(err) => err.source(),
            LogError::State(err) => err.source(),
        }
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub fn new(message: String) -> Self {
        ParseError { message }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error: {}", self.message)
    }
}

impl Error for ParseError {}

#[derive(Debug)]
pub struct CryptoError {
    underlying: Option<Box<dyn Error + Send + Sync>>,
}

impl CryptoError {
    pub fn new(err: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        CryptoError {
            underlying: Some(err.into()),
        }
    }
}

impl Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.underlying)
    }
}

impl Error for CryptoError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.underlying {
            None => None,
            Some(source) => Some(&**source),
        }
    }
}

impl From<aead::Error> for CryptoError {
    fn from(err: aead::Error) -> Self {
        CryptoError::new(format!("{:?}", err))
    }
}

#[derive(Debug)]
pub struct IllegalArgumentError {
    pub err_msg: String,
}

impl IllegalArgumentError {
    pub fn new(err_msg: String) -> Self {
        IllegalArgumentError { err_msg }
    }
}

impl Display for IllegalArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.err_msg)
    }
}

impl Error for IllegalArgumentError {}

#[derive(Debug)]
pub struct StateError {
    pub err_msg: String,
}

impl StateError {
    fn new(err_msg: String) -> StateError {
        StateError { err_msg }
    }
}

impl Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.err_msg)
    }
}

impl Error for StateError {}

impl Display for LogError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            LogError::Compress(err) => err.fmt(fmt),
            LogError::IoError(err) => err.fmt(fmt),
            LogError::Parse(err) => err.fmt(fmt),
            LogError::Crypto(err) => err.fmt(fmt),
            LogError::IllegalArgument(err) => err.fmt(fmt),
            LogError::State(err) => err.fmt(fmt),
        }
    }
}

impl From<io::Error> for LogError {
    fn from(err: io::Error) -> LogError {
        LogError::IoError(err)
    }
}

impl From<CryptoError> for LogError {
    fn from(err: CryptoError) -> LogError {
        LogError::Crypto(err)
    }
}

impl From<IllegalArgumentError> for LogError {
    fn from(err: IllegalArgumentError) -> Self {
        LogError::IllegalArgument(err)
    }
}

pub fn channel_send_err<T>(err: TrySendError<T>) -> LogError {
    match err {
        TrySendError::Full(_) => LogError::State(StateError::new("channel is full".to_string())),
        TrySendError::Disconnected(_) => {
            LogError::State(StateError::new("channel is disconnected".to_string()))
        }
    }
}
