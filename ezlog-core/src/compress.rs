use std::io::{
    Read,
    Write,
};

use crate::{
    Compression,
    Decompression,
};

/// Compress type can be used to compress the log file.
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
pub enum CompressKind {
    /// ZLIB compression
    /// we use [flate2](https://github.com/rust-lang/flate2-rs) to implement this
    ZLIB,
    /// No compression
    NONE,
    /// Unknown compression
    UNKNOWN,
}

impl From<u8> for CompressKind {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => CompressKind::NONE,
            0x01 => CompressKind::ZLIB,
            _ => CompressKind::UNKNOWN,
        }
    }
}

impl From<CompressKind> for u8 {
    fn from(orig: CompressKind) -> Self {
        match orig {
            CompressKind::NONE => 0x00,
            CompressKind::ZLIB => 0x01,
            CompressKind::UNKNOWN => 0xff,
        }
    }
}

/// Compress level
///
/// can be define as one of the following: FAST, DEFAULT, BEST
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
pub enum CompressLevel {
    Fast,
    Default,
    Best,
}

impl From<u8> for CompressLevel {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => CompressLevel::Default,
            0x01 => CompressLevel::Fast,
            0x02 => CompressLevel::Best,
            _ => CompressLevel::Default,
        }
    }
}

impl From<CompressLevel> for u8 {
    fn from(orig: CompressLevel) -> Self {
        match orig {
            CompressLevel::Default => 0x00,
            CompressLevel::Fast => 0x01,
            CompressLevel::Best => 0x02,
        }
    }
}

pub struct ZlibCodec {
    level: flate2::Compression,
}

impl ZlibCodec {
    pub fn new(level: &CompressLevel) -> Self {
        match level {
            CompressLevel::Fast => Self {
                level: flate2::Compression::fast(),
            },
            CompressLevel::Default => Self {
                level: flate2::Compression::default(),
            },
            CompressLevel::Best => Self {
                level: flate2::Compression::best(),
            },
        }
    }
}

impl Compression for ZlibCodec {
    fn compress(&self, data: &[u8]) -> std::io::Result<Vec<u8>> {
        let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), self.level);
        zlib.write_all(data)?;
        zlib.finish()
    }
}

impl Decompression for ZlibCodec {
    fn decompress(&self, data: &[u8]) -> std::io::Result<Vec<u8>> {
        let mut zlib = flate2::read::ZlibDecoder::new(data);
        let mut out = Vec::new();
        zlib.read_to_end(&mut out)?;
        Ok(out)
    }
}
