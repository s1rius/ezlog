use std::{
    io::{
        self,
        BufRead,
        Cursor,
        Write,
    },
    str::FromStr,
    sync::mpsc::channel,
    time::Duration,
};

use byteorder::{
    BigEndian,
    ReadBytesExt,
};
use integer_encoding::VarIntReader;
use log::error;
use once_cell::sync::OnceCell;
use regex::Regex;
use time::{
    format_description::well_known::Rfc3339,
    OffsetDateTime,
};

use crate::{
    errors::LogError,
    Compress,
    Cryptor,
    EZRecord,
    Header,
    Level,
    NonceGenFn,
    Result,
    Version,
    RECORD_SIGNATURE_START,
};

pub fn decode_record(vec: &[u8]) -> Result<crate::EZRecord> {
    static RE: OnceCell<std::result::Result<Regex, regex::Error>> = OnceCell::new();
    let regex = RE.get_or_init(|| Regex::new(r"\[(.*?)\]"));
    let record_str =
        String::from_utf8(vec.to_vec()).map_err(|e| LogError::Parse(format!("{}", e)))?;
    let mut record_builder = EZRecord::builder();
    if let Ok(regex) = regex {
        // Search for the first match
        if let Some(caps) = regex.captures(&record_str) {
            if let Some(matched) = caps.get(1) {
                let header = matched.as_str();
                let split: Vec<&str> = header.split_whitespace().collect();

                let time = split
                    .first()
                    .map(|x| {
                        OffsetDateTime::parse(x, &Rfc3339).unwrap_or(OffsetDateTime::now_utc())
                    })
                    .unwrap_or(OffsetDateTime::now_utc());
                record_builder.time(time);

                let level = split
                    .get(1)
                    .and_then(|x| Level::from_str(x).ok())
                    .unwrap_or(Level::Trace);
                record_builder.level(level);

                let target = split.get(2).unwrap_or(&"").to_string();
                record_builder.target(target);

                let thread_str = split.get(3).unwrap_or(&"");
                let mut thread_info: Vec<&str> = thread_str.split(':').collect();
                if !thread_info.is_empty() {
                    let thread_id = thread_info.last().unwrap_or(&"").to_string();
                    record_builder.thread_id(thread_id.parse::<usize>().unwrap_or(0));
                    thread_info.pop();
                    let thread_name = thread_info.join(":");
                    record_builder.thread_name(thread_name);
                }

                #[cfg(feature = "log")]
                {
                    if split.len() == 5 {
                        let file_info: Vec<&str> = split.last().unwrap_or(&"").split(':').collect();
                        if !file_info.is_empty() {
                            let file_name = file_info.first().unwrap_or(&"");
                            let line = file_info.get(1).unwrap_or(&"");

                            record_builder.file(file_name);
                            record_builder.line(line.parse::<u32>().unwrap_or(0));
                        }
                    };
                }
                // header not contains square brackets
                // two square brackets and one space
                if record_str.len() > header.len() + 3 {
                    let content = &record_str[header.len() + 3..];
                    record_builder.content(content);
                }
            }
        }
    }

    Ok(record_builder.build())
}

#[inline]
pub(crate) fn decode_record_from_read(
    reader: &mut Cursor<Vec<u8>>,
    compression: &Option<Box<dyn Compress + Send + Sync>>,
    cryptor: &Option<Box<dyn Cryptor + Send + Sync>>,
    header: &Header,
    position: u64,
) -> Result<Vec<u8>> {
    let chunk = decode_record_to_content(reader, &header.version)?;
    let combine = crate::logger::combine_time_position(header.timestamp.unix_timestamp(), position);

    let op = Box::new(move |input: &[u8]| crate::logger::xor_slice(input, &combine));
    if header.has_record() && !header.is_extra_index(position) {
        decode_record_content(&header.version, &chunk, compression, cryptor, op)
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
    // ignore the end sign check
    let _ = reader.read_u8()?;
    Ok(chunk)
}

#[inline]
pub(crate) fn decode_record_size(mut reader: &mut dyn BufRead, version: &Version) -> Result<usize> {
    match version {
        Version::NONE => Ok(0),
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
    compression: &Option<Box<dyn Compress + Send + Sync>>,
    cryptor: &Option<Box<dyn Cryptor + Send + Sync>>,
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

pub fn decode_with_fn<F>(
    reader: &mut Cursor<Vec<u8>>,
    compression: &Option<Box<dyn Compress + Send + Sync>>,
    cryptor: &Option<Box<dyn Cryptor + Send + Sync>>,
    header: &Header,
    mut op: F,
) where
    F: for<'a> FnMut(&'a Vec<u8>, bool) -> Option<u64>,
{
    loop {
        let position: u64 = reader.position();
        match decode_record_from_read(reader, compression, cryptor, header, position) {
            Ok(buf) => match op(&buf, buf.is_empty()) {
                Some(skip) => {
                    if skip > 0 {
                        reader.set_position(reader.position() + skip);
                    }
                }
                None => break,
            },
            Err(e) => match e {
                LogError::IoError(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof {
                        op(&vec![], true);
                        break;
                    }
                }
                LogError::Illegal(e) => {
                    error!(target: "ezlog_decode", "{}", e);
                    break;
                }
                _ => {
                    error!(target: "ezlog_decode", "{}", e);
                }
            },
        }
    }
}

pub fn decode_with_writer(
    cursor: &mut Cursor<Vec<u8>>,
    writer: &mut io::BufWriter<std::fs::File>,
    compression: Option<Box<dyn Compress + Send + Sync>>,
    decryptor: Option<Box<dyn Cryptor + Send + Sync>>,
    header: &Header,
) -> Result<()> {
    let (tx, rx) = channel();
    let write_closure = move |data: &Vec<u8>, flag: bool| {
        writer
            .write_all(data)
            .unwrap_or_else(|e| error!(target: "ezlog_decode", "{}", e));
        writer
            .write_all(b"\n")
            .unwrap_or_else(|e| error!(target: "ezlog_decode", "{}", e));
        if flag {
            writer
                .flush()
                .unwrap_or_else(|e| error!(target: "ezlog_decode", "{}", e));
            tx.send(())
                .unwrap_or_else(|e| error!(target: "ezlog_decode", "{}", e));
            return None;
        }
        Some(0)
    };

    decode_with_fn(cursor, &compression, &decryptor, header, write_closure);
    rx.recv_timeout(Duration::from_secs(60 * 5))
        .map_err(|e| LogError::Parse(format!("{}", e)))
}

pub fn decode_header_and_extra(
    cursor: &mut Cursor<Vec<u8>>,
) -> Result<(Header, Option<(String, &str)>)> {
    let header = Header::decode(cursor)?;
    let mut extra: Option<(String, &str)> = None;
    if header.has_extra() {
        decode_with_fn(cursor, &None, &None, &header, |v, _| {
            extra = String::from_utf8(v.clone())
                .map(|x| (Some((x, "utf-8"))))
                .map_err(|_| Some((hex::decode(v), "hex")))
                .unwrap_or(None);
            None
        })
    }
    Ok((header, extra))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{
        BufReader,
        Cursor,
        Read,
    };
    use std::sync::mpsc::channel;
    use std::time::Duration;

    use time::OffsetDateTime;

    use super::decode_record;
    use crate::thread_name;
    use crate::{
        decode,
        EZLogger,
        EZRecord,
        EZRecordBuilder,
        Header,
    };

    #[cfg(feature = "decode")]
    fn create_all_feature_config(path: &str) -> crate::EZLogConfig {
        use crate::CipherKind;
        use crate::CompressKind;

        std::fs::create_dir_all(test_compat::test_path().join(path)).unwrap();

        let key = b"an example very very secret key.";
        let nonce = b"unique nonce";
        crate::EZLogConfigBuilder::new()
            .dir_path(
                test_compat::test_path()
                    .join(path)
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(String::from("all_feature"))
            .file_suffix(String::from("mmap"))
            .max_size(500 * 1024)
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

    #[inline]
    fn decode_logs_count(
        logger: &mut EZLogger,
        reader: &mut Cursor<Vec<u8>>,
        header: &Header,
    ) -> crate::Result<i32> {
        let (tx, rx) = channel();

        let mut count = 0;
        let my_closure = |data: &Vec<u8>, is_end: bool| {
            if !data.is_empty() {
                count += 1;
            }
            if is_end {
                tx.send(()).expect("Could not send signal on channel.");
                return None;
            }
            Some(0)
        };
        crate::decode::decode_with_fn(
            reader,
            &logger.compression,
            &logger.cryptor,
            header,
            my_closure,
        );
        rx.recv().expect("Could not receive from channel.");
        Ok(count)
    }

    #[cfg(feature = "decode")]
    #[test]
    fn teset_encode_decode_file() {
        let config = create_all_feature_config("test_file");

        let mut logger = EZLogger::new(config.clone()).unwrap();

        let log_count = 10;
        for i in 0..log_count {
            logger
                .append(
                    EZRecordBuilder::default()
                        .content(format!("hello world {}", i))
                        .build(),
                )
                .unwrap();
        }
        logger.flush().unwrap();

        let (path, _mmap) = &config.create_mmap_file().unwrap();
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        let mut buf = Vec::<u8>::new();
        file.read_to_end(&mut buf).unwrap();
        let mut cursor = Cursor::new(buf);
        let mut header = Header::decode(&mut cursor).unwrap();
        header.recorder_position = header.length().try_into().unwrap();
        let mut new_header = Header::create(&logger.config);
        new_header.timestamp = header.timestamp;
        new_header.rotate_time = header.rotate_time;
        new_header.recorder_position = Header::length_compat(&config.version()) as u32;
        assert_eq!(header, new_header);
        let count = decode_logs_count(&mut logger, &mut cursor, &header).unwrap();

        assert_eq!(count, log_count);
        drop(logger);
        fs::remove_dir_all(&config.dir_path()).unwrap_or_default();
    }

    #[inline]
    fn decode_array_record(
        logger: &mut EZLogger,
        reader: &mut Cursor<Vec<u8>>,
        header: &Header,
    ) -> crate::Result<Vec<EZRecord>> {
        let mut array: Vec<EZRecord> = Vec::new();
        let (tx, rx) = channel();

        let my_closure = |data: &Vec<u8>, is_end: bool| {
            if !data.is_empty() {
                match decode_record(data) {
                    Ok(r) => array.push(r),
                    Err(e) => {
                        println!("{}", e)
                    }
                }
            }
            if is_end {
                tx.send(()).expect("Could not send signal on channel.");
                return None;
            }
            Some(0)
        };
        crate::decode::decode_with_fn(
            reader,
            &logger.compression,
            &logger.cryptor,
            header,
            my_closure,
        );
        rx.recv().expect("Could not receive from channel.");
        Ok(array)
    }

    #[test]
    #[cfg(feature = "decode")]
    #[cfg(feature = "log")]
    pub fn test_decode_to_struct() {
        let record = EZRecordBuilder::default()
            .time(OffsetDateTime::now_utc())
            .file("demo.rs")
            .line(1)
            .content("test".to_string())
            .level(crate::Level::Trace)
            .target("target".to_string())
            .thread_name(thread_name::get())
            .thread_id(thread_id::get())
            .build();

        let s = crate::formatter().format(&record).unwrap();
        let record_decode = decode_record(&s).unwrap();

        assert_eq!(record, record_decode)
    }

    #[cfg(feature = "decode")]
    #[test]
    fn teset_decode_to_array() {
        use crate::EZRecord;

        let config = create_all_feature_config("test_array");

        let mut logger = EZLogger::new(config.clone()).unwrap();
        let log_count = 10;
        let mut array: Vec<EZRecord> = Vec::new();
        for i in 0..log_count {
            let item = EZRecordBuilder::default()
                .content(format!("hello world {}", i))
                .time(OffsetDateTime::now_utc() - Duration::from_secs(60 * 60))
                .target("target".to_string())
                .build();
            logger.append(item.clone()).unwrap();
            array.push(item);
        }
        logger.flush().unwrap();

        let (path, _mmap) = &config.create_mmap_file().unwrap();
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        let mut buf = Vec::<u8>::new();
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut buf).unwrap();
        assert!(!buf.is_empty());
        let mut cursor = Cursor::new(buf);
        let header = Header::decode(&mut cursor).unwrap();
        assert!(header.has_record());
        let decode_array = decode_array_record(&mut logger, &mut cursor, &header).unwrap();

        assert_eq!(array, decode_array);
        drop(logger);
        fs::remove_dir_all(test_compat::test_path().join("test_array")).unwrap_or_default();
    }
}
