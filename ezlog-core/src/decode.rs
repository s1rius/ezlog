use std::io::{self, BufRead, Cursor, Write};

use byteorder::{BigEndian, ReadBytesExt};
use integer_encoding::VarIntReader;

use crate::{
    errors::LogError, Compress, Cryptor, EZLogger, Header, NonceGenFn, Result, Version,
    RECORD_SIGNATURE_END, RECORD_SIGNATURE_START,
};

#[inline]
pub fn decode_logs_count(
    logger: &mut EZLogger,
    reader: &mut Cursor<Vec<u8>>,
    header: &Header,
) -> Result<i32> {
    let mut count = 0;
    loop {
        let position = reader.position();
        match decode_record_from_read(
            reader,
            &logger.config.version,
            &logger.compression,
            &logger.cryptor,
            header,
            position,
        ) {
            Ok(_) => {
                count += 1;
            }
            Err(e) => match e {
                LogError::IoError(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof {
                        break;
                    }
                }
                LogError::IllegalArgument(_) => break,
                _ => continue,
            },
        }
    }
    Ok(count)
}

#[inline]
pub fn decode_body_and_write(
    reader: &mut Cursor<Vec<u8>>,
    writer: &mut dyn Write,
    version: &Version,
    compression: &Option<Box<dyn Compress>>,
    cryptor: &Option<Box<dyn Cryptor>>,
    header: &Header,
) -> io::Result<()> {
    loop {
        let position: u64 = reader.position();
        match decode_record_from_read(reader, version, compression, cryptor, header, position) {
            Ok(buf) => {
                if buf.is_empty() {
                    break;
                }
                writer.write_all(&buf)?;
            }
            Err(e) => match e {
                LogError::IoError(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof {
                        break;
                    }
                }
                LogError::IllegalArgument(_) => break,
                _ => continue,
            },
        }
    }
    writer.flush()
}

#[inline]
pub(crate) fn decode_record_from_read(
    reader: &mut Cursor<Vec<u8>>,
    version: &Version,
    compression: &Option<Box<dyn Compress>>,
    cryptor: &Option<Box<dyn Cryptor>>,
    header: &Header,
    position: u64,
) -> Result<Vec<u8>> {
    let chunk = decode_record_to_content(reader, version)?;
    let combine = crate::logger::combine_time_position(header.timestamp.unix_timestamp(), position);

    let op = Box::new(move |input: &[u8]| crate::logger::xor_slice(input, &combine));
    if header.has_record() && position != header.length().try_into().unwrap_or_default() {
        decode_record_content(version, &chunk, compression, cryptor, op)
    } else {
        Ok(chunk)
    }
}

#[inline]
pub(crate) fn decode_record_to_content(
    reader: &mut dyn BufRead,
    version: &Version,
) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let nums = reader.read_until(RECORD_SIGNATURE_START, &mut buf)?;
    if nums == 0 {
        return Err(LogError::IllegalArgument(
            "has no record start signature".to_string(),
        ));
    }
    let content_size: usize = decode_record_size(reader, version)?;
    let mut chunk = vec![0u8; content_size];
    reader.read_exact(&mut chunk)?;
    let end_sign = reader.read_u8()?;
    if RECORD_SIGNATURE_END != end_sign {
        return Err(LogError::Parse("record end sign error".to_string()));
    }
    Ok(chunk)
}

#[inline]
pub(crate) fn decode_record_size(mut reader: &mut dyn BufRead, version: &Version) -> Result<usize> {
    match version {
        Version::V1 => {
            let size_of_size = reader.read_u8()?;
            let content_size: usize = match size_of_size {
                1 => reader.read_u8()? as usize,
                2 => reader.read_u16::<BigEndian>()? as usize,
                _ => reader.read_u32::<BigEndian>()? as usize,
            };
            Ok(content_size)
        }
        Version::V2 => {
            let size: usize = reader.read_varint()?;
            Ok(size)
        }
        Version::UNKNOWN => Err(LogError::IllegalArgument(format!(
            "unknow version {:?}",
            version
        ))),
    }
}

#[inline]
pub fn decode_record_content(
    version: &Version,
    chunk: &[u8],
    compression: &Option<Box<dyn Compress>>,
    cryptor: &Option<Box<dyn Cryptor>>,
    op: NonceGenFn,
) -> Result<Vec<u8>> {
    let mut buf = chunk.to_vec();

    if *version == Version::V1 {
        if let Some(decompression) = compression {
            buf = decompression.decompress(&buf)?;
        }

        if let Some(decryptor) = cryptor {
            buf = decryptor.decrypt(&buf, op)?;
        }
    } else {
        if let Some(decryptor) = cryptor {
            buf = decryptor.decrypt(&buf, op)?;
        }

        if let Some(decompression) = compression {
            buf = decompression.decompress(&buf)?;
        }
    }

    Ok(buf)
}
