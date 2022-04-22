use std::{error::Error, io, fmt::{self, Display}};

#[derive(Debug)]
pub enum LogError {
    Encoding(EncodingError),
    IoError(io::Error),
    Parse(ParseError),
}

#[derive(Debug)]
pub struct EncodingError {
    underlying: Option<Box<dyn Error + Send + Sync>>,
}

impl EncodingError {
    /// Create an `EncodingError` that stems from an arbitrary error of an underlying encoder.
    pub fn new(err: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        EncodingError {
            underlying: Some(err.into()),
        }
    }
}

impl Display for EncodingError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match &self.underlying {
            Some(underlying) => write!(
                fmt,
                "Format error encoding :\n{}",
                underlying,
            ),
            None => write!(fmt, "Format error encoding"),
        }
    }
}

impl Error for EncodingError {
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
            LogError::Encoding(err) => err.source(),
            LogError::IoError(err) => err.source(),
            LogError::Parse(err) => err.source(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub fn new(message: String) -> Self {
        ParseError {
            message
        }
    }
}

impl Display for ParseError {
    
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error: {}", self.message)
    }
}

impl Error for ParseError {}
    

impl Display for LogError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            LogError::Encoding(err) => err.fmt(fmt),
            LogError::IoError(err) => err.fmt(fmt),
            LogError::Parse(err) => err.fmt(fmt),
        }
    }
}

impl From<io::Error> for LogError {
    fn from(err: io::Error) -> LogError {
        LogError::IoError(err)
    }
}