use std::io::{Read, Write};

use crate::{CompressLevel, Compression, Decompression};

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

pub struct ZstdCodec {
    level: zstd::zstd_safe::CompressionLevel,
}

impl ZstdCodec {
    pub fn new(level: &CompressLevel) -> Self {
        match level {
            CompressLevel::Fast => Self {
                level: zstd::zstd_safe::min_c_level(),
            },
            CompressLevel::Default => Self {
                level: zstd::zstd_safe::CompressionLevel::default(),
            },
            CompressLevel::Best => Self {
                level: zstd::zstd_safe::max_c_level(),
            },
        }
    }
}

impl Compression for ZstdCodec {
    fn compress(&self, data: &[u8]) -> std::io::Result<Vec<u8>> {
        zstd::encode_all(data, self.level)
    }
}

impl Decompression for ZstdCodec {
    fn decompress(&self, data: &[u8]) -> std::io::Result<Vec<u8>> {
        zstd::decode_all(data)
    }
}
