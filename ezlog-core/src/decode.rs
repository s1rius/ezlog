use std::io::{
    self,
    BufRead,
    Cursor,
    Write,
};

use byteorder::{
    BigEndian,
    ReadBytesExt,
};
use integer_encoding::VarIntReader;

use crate::{
    errors::LogError,
    Compress,
    Cryptor,
    EZLogger,
    Header,
    NonceGenFn,
    Result,
    Version,
    RECORD_SIGNATURE_END,
    RECORD_SIGNATURE_START,
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
                LogError::Illegal(_) => break,
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
                writer.write_all(b"\n")?;
            }
            Err(e) => match e {
                LogError::IoError(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof {
                        break;
                    }
                }
                LogError::Illegal(_) => break,
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
    if header.has_record() && position != header.length() as u64 {
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
        return Err(LogError::Illegal(
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
        Version::UNKNOWN => Err(LogError::Illegal(format!("unknow version {:?}", version))),
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{
        BufReader,
        Cursor,
        Read,
    };

    use crate::decode::decode_logs_count;
    use crate::{
        decode,
        EZLogger,
        EZRecordBuilder,
        Header,
    };

    #[cfg(feature = "decode")]
    fn create_all_feature_config() -> crate::EZLogConfig {
        use crate::CipherKind;
        use crate::CompressKind;

        let key = b"an example very very secret key.";
        let nonce = b"unique nonce";
        crate::EZLogConfigBuilder::new()
            .dir_path(
                dirs::cache_dir()
                    .unwrap()
                    .join("ezlog_test")
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(String::from("all_feature"))
            .file_suffix(String::from("mmap"))
            .max_size(150 * 1024)
            .compress(CompressKind::ZLIB)
            .cipher(CipherKind::AES256GCMSIV)
            .cipher_key(key.to_vec())
            .cipher_nonce(nonce.to_vec())
            .build()
    }

    #[cfg(feature = "decode")]
    #[test]
    fn test_record_len() {
        use crate::logger::create_size_chunk;

        let chunk = create_size_chunk(1000).unwrap();
        let mut cursor = Cursor::new(chunk);
        let size = decode::decode_record_size(&mut cursor, &crate::Version::V2).unwrap();
        assert_eq!(1000, size)
    }

    #[cfg(feature = "decode")]
    #[test]
    fn teset_encode_decode_trunk() {
        use crate::logger::encode_content;

        let vec = "hello world".as_bytes();
        let encode = encode_content(vec.to_owned()).unwrap();
        let mut cursor = Cursor::new(encode);
        let decode = decode::decode_record_to_content(&mut cursor, &crate::Version::V2).unwrap();
        assert_eq!(vec, decode)
    }

    #[cfg(feature = "decode")]
    #[test]
    fn teset_encode_decode_file() {
        let config = create_all_feature_config();
        fs::remove_dir_all(&config.dir_path).unwrap_or_default();
        let mut logger = EZLogger::new(config.clone()).unwrap();

        let log_count = 10;
        for i in 0..log_count {
            logger
                .append(
                    &EZRecordBuilder::default()
                        .content(format!("hello world {}", i))
                        .build(),
                )
                .unwrap();
        }
        logger.flush().unwrap();

        let (path, _mmap) = &config.create_mmap_file().unwrap();
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        let mut buf = Vec::<u8>::new();
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut buf).unwrap();
        let mut cursor = Cursor::new(buf);
        let mut header = Header::decode(&mut cursor).unwrap();
        header.recorder_position = header.length().try_into().unwrap();
        let mut new_header = Header::create(&logger.config);
        new_header.timestamp = header.timestamp.clone();
        new_header.rotate_time = header.rotate_time.clone();
        new_header.recorder_position = Header::length_compat(&config.version) as u32;
        assert_eq!(header, new_header);
        let count = decode_logs_count(&mut logger, &mut cursor, &header).unwrap();
        assert_eq!(count, log_count);
        fs::remove_dir_all(&config.dir_path).unwrap_or_default();
    }
}
