use std::io::{Read, Write};

use crate::{Compression, Decompression};

pub struct ZlibCodec {
    level: flate2::Compression,
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
