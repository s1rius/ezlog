use core::fmt;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::{
    fs,
    io,
};

use byteorder::ReadBytesExt;
use byteorder::{
    BigEndian,
    WriteBytesExt,
};
use integer_encoding::VarIntWriter;
use time::OffsetDateTime;

#[cfg(feature = "decode")]
use crate::crypto::{
    Aes128Gcm,
    Aes256Gcm,
};
use crate::crypto::{
    Aes128GcmSiv,
    Aes256GcmSiv,
};
use crate::events::Event::{
    self,
};
use crate::{
    appender::EZAppender,
    compress::ZlibCodec,
    errors::LogError,
    CipherKind,
    Compress,
    CompressKind,
    Cryptor,
    EZLogConfig,
    EZRecord,
    RECORD_SIGNATURE_END,
    RECORD_SIGNATURE_START,
};
use crate::{
    errors,
    event,
    NonceGenFn,
    V1_LOG_HEADER_SIZE,
};
use crate::{
    Version,
    V2_LOG_HEADER_SIZE,
};

type Result<T> = std::result::Result<T, LogError>;

#[inline]
pub(crate) fn create_size_chunk(size: usize) -> Result<Vec<u8>> {
    let mut chunk: Vec<u8> = Vec::new();
    chunk.write_varint(size)?;
    Ok(chunk)
}

#[inline]
pub(crate) fn encode_content(mut buf: Vec<u8>) -> Result<Vec<u8>> {
    let mut chunk: Vec<u8> = Vec::new();
    chunk.push(RECORD_SIGNATURE_START);
    let size = buf.len();
    let mut size_chunk = create_size_chunk(size)?;
    chunk.append(&mut size_chunk);
    chunk.append(&mut buf);
    chunk.push(RECORD_SIGNATURE_END);
    Ok(chunk)
}

#[allow(deprecated)]
pub fn create_cryptor(config: &EZLogConfig) -> Result<Option<Box<dyn Cryptor + Send + Sync>>> {
    if let Some(key) = &config.cipher_key() {
        if let Some(nonce) = &config.cipher_nonce() {
            #[warn(unreachable_patterns)]
            match config.cipher_kind() {
                #[cfg(feature = "decode")]
                CipherKind::AES128GCM => {
                    let encryptor = Aes128Gcm::new(key, nonce)?;
                    Ok(Some(Box::new(encryptor)))
                }
                #[cfg(feature = "decode")]
                CipherKind::AES256GCM => {
                    let encryptor = Aes256Gcm::new(key, nonce)?;
                    Ok(Some(Box::new(encryptor)))
                }
                CipherKind::AES128GCMSIV => {
                    let encryptor = Aes128GcmSiv::new(key, nonce)?;
                    Ok(Some(Box::new(encryptor)))
                }
                CipherKind::AES256GCMSIV => {
                    let encryptor = Aes256GcmSiv::new(key, nonce)?;
                    Ok(Some(Box::new(encryptor)))
                }
                CipherKind::NONE => Ok(None),
                unknown => Err(LogError::Crypto(format!("unknown cryption {}", unknown))),
            }
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

pub fn create_compress(config: &EZLogConfig) -> Option<Box<dyn Compress + Send + Sync>> {
    match config.compress_kind() {
        CompressKind::ZLIB => Some(Box::new(ZlibCodec::new(&config.compress_level()))),
        CompressKind::NONE => None,
        CompressKind::UNKNOWN => None,
    }
}

pub struct EZLogger {
    pub(crate) config: EZLogConfig,
    pub(crate) appender: EZAppender,
    pub(crate) compression: Option<Box<dyn Compress + Send + Sync>>,
    pub(crate) cryptor: Option<Box<dyn Cryptor + Send + Sync>>,
}

/// log result
#[derive(Debug)]
pub(crate) enum AppendSuccess {
    Success,
    RotatedAndRetried,
}

impl EZLogger {
    pub fn new(config: EZLogConfig) -> Result<Self> {
        let appender = EZAppender::new(&config)?;
        appender.check_config_rolling(&config)?;
        let compression = create_compress(&config);
        let cryptor = create_cryptor(&config)?;
        Ok(Self {
            config,
            appender,
            compression,
            cryptor,
        })
    }

    /// TODO buggy add test case
    pub(crate) fn append(&self, record: EZRecord) -> Result<AppendSuccess> {
        let mut rotate = false;
        let splits = if record.content().len() > self.config.max_size() as usize / 2 {
            record.trunks(&self.config)
        } else {
            vec![record]
        };
        for record in splits.iter() {
            let id = record.t_id();
            let buf = self.encode_as_block(record)?;
            let result = { self.appender.get_inner_mut()?.append(&buf) };
            match result {
                Ok(_) => {
                    event!(Event::RecordEnd, &id);
                }
                Err(e) => {
                    event!(!Event::RecordError; &e);
                    // Check if the error is an appender error (e.g., file is full or needs rotation)
                    if let Some(is_rotation_error) = self.is_rotation_needed(&e) {
                        if is_rotation_error {
                            // Rotate the appender and retry
                            self.appender.rotate(&self.config).inspect_err(
                                |e| event!(!Event::RotateFileError, "rotate error"; e),
                            )?;
                            // Retry write once after rotation
                            let retry_result = {
                                let mut inner = self.appender.get_inner_mut()?;
                                inner.append(&buf)
                            };
                            match retry_result {
                                Ok(_) => {
                                    event!(Event::RecordEnd, &id);
                                    rotate = true;
                                }
                                Err(e) => {
                                    event!(!Event::RecordError; &e);
                                    return Err(e.into());
                                }
                            }
                        }
                    }
                }
            }
        }
        if rotate {
            Ok(AppendSuccess::RotatedAndRetried)
        } else {
            Ok(AppendSuccess::Success)
        }
    }

    #[inline]
    fn encode(&self, record: &EZRecord) -> Result<Vec<u8>> {
        let nonce_fn: NonceGenFn = self.gen_nonce()?;
        let mut buf = self.format(record)?;
        if buf.is_empty() {
            return Ok(buf);
        }
        if self.config.version() == Version::V1 {
            if let Some(encryptor) = &self.cryptor {
                event!(Event::Encrypt, &record.t_id());
                buf = encryptor.encrypt(&buf, nonce_fn)?;
                event!(Event::EncryptEnd, &record.t_id());
            }
            if let Some(compression) = &self.compression {
                event!(Event::Compress, &record.t_id());
                buf = compression.compress(&buf).map_err(LogError::Compress)?;
                event!(Event::CompressEnd, &record.t_id());
            }
        } else {
            let len = buf.len();
            if let Some(compression) = &self.compression {
                event!(Event::Compress, &record.t_id());
                buf = compression.compress(&buf).map_err(LogError::Compress)?;
                event!(
                    Event::CompressEnd,
                    "{} compress ratio = {} ",
                    &record.t_id(),
                    buf.len() as f64 / len as f64
                );
            }
            if let Some(encryptor) = &self.cryptor {
                event!(Event::Encrypt, &record.t_id());
                buf = encryptor.encrypt(&buf, nonce_fn)?;
                event!(
                    Event::EncryptEnd,
                    "{} process ratio = {} ",
                    &record.t_id(),
                    buf.len() as f64 / len as f64
                );
            }
        }
        Ok(buf)
    }

    /// Generates a nonce generation function for the current `EZLogger`.
    ///
    /// The nonce generation function XORs each input slice with a unique nonce that is generated based on the current
    /// timestamp and recorder position of the `EZAppender`.
    ///
    /// # Returns
    ///
    /// A `NonceGenFn` closure that be used in encode and decode.
    fn gen_nonce(&self) -> crate::Result<NonceGenFn> {
        let inner = self.appender.inner.read()?;
        let timestamp = inner.header().timestamp.unix_timestamp();
        let position = inner.header().recorder_position;
        let combine = combine_time_position(timestamp, position.into());

        // create and return a closure that XORs each input slice with the count
        Ok(Box::new(move |input| xor_slice(input, &combine)))
    }

    #[inline]
    pub fn encode_as_block(&self, record: &EZRecord) -> Result<Vec<u8>> {
        let buf = self.encode(record)?;
        encode_content(buf)
    }

    fn format(&self, record: &EZRecord) -> Result<Vec<u8>> {
        crate::formatter().format(record)
    }

    pub(crate) fn flush(&self) -> crate::Result<()> {
        self.appender
            .get_inner_mut()?
            .flush()
            .map_err(|e| errors::LogError::IoError(io::Error::other(e)))
    }

    pub(crate) fn trim(&self) {
        match fs::read_dir(self.config.dir_path()) {
            Ok(dir) => {
                for file in dir {
                    match file {
                        Ok(file) => {
                            if let Some(name) = file.file_name().to_str() {
                                match self.config.is_file_out_of_date(name) {
                                    Ok(out_of_date) => {
                                        if out_of_date {
                                            fs::remove_file(file.path()).unwrap_or_else(|e| {
                                                event!(
                                                    !Event::TrimError,
                                                    "remove file err";
                                                    &e.into()
                                                )
                                            });
                                        }
                                    }
                                    Err(e) => {
                                        event!(!Event::TrimError, "judge file out of date error"; &e)
                                    }
                                }
                            };
                        }
                        Err(e) => {
                            event!(!Event::TrimError, "traversal file error"; &e.into())
                        }
                    }
                }
            }
            Err(e) => event!(!Event::TrimError, "read dir error"; &e.into()),
        }
    }

    pub fn query_log_files_for_date(&self, date: OffsetDateTime) -> Vec<PathBuf> {
        self.config.query_log_files_for_date(date)
    }

    pub(crate) fn rotate_if_not_empty(&self) -> Result<()> {
        if self
            .appender
            .get_inner()?
            .header()
            .has_record_exclude_extra(&self.config)
        {
            self.appender.rotate(&self.config)
        } else {
            Ok(())
        }
    }

    fn is_rotation_needed(&self, e: &io::Error) -> Option<bool> {
        if e.kind() == io::ErrorKind::Other {
            e.get_ref().and_then(|inner| {
                inner
                    .downcast_ref::<crate::appender::AppenderError>()
                    .map(|appender_err| {
                        matches!(
                            appender_err,
                            crate::appender::AppenderError::SizeExceeded { .. }
                                | crate::appender::AppenderError::RotateTimeExceeded { .. }
                        )
                    })
            })
        } else {
            None
        }
    }
}

pub(crate) fn combine_time_position(timestamp: i64, position: u64) -> Vec<u8> {
    let position_bytes = position.to_be_bytes();
    let time_bytes = timestamp.to_be_bytes();
    let mut vec = time_bytes.to_vec();
    vec.extend(position_bytes);
    vec
}

pub(crate) fn xor_slice<'a>(slice: &'a [u8], vec: &'a [u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(slice.len());
    for (i, byte) in slice.iter().enumerate() {
        if let Some(vec_byte) = vec.get(i) {
            result.push(byte ^ vec_byte);
        } else {
            result.push(*byte);
        }
    }
    result
}

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
    pub(crate) struct Flags: u8 {
        const NONE = 0b0000_0000;
        const HAS_EXTRA = 0b0000_0001;
    }
}

/// EZLog file Header
///
/// every log file starts with a header,
/// which is used to describe the version, log length, compress type, cipher kind and so on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
pub struct Header {
    /// version code
    pub(crate) version: Version,
    /// flag
    pub(crate) flag: Flags,
    /// current log file write position
    pub(crate) recorder_position: u32,
    /// compress type
    pub(crate) compress: CompressKind,
    /// cipher kind
    pub(crate) cipher: CipherKind,
    // config key and nonce hash
    pub(crate) cipher_hash: u32,
    /// timestamp
    #[cfg_attr(feature = "json", serde(serialize_with = "crate::serialize_time"))]
    #[cfg_attr(feature = "json", serde(deserialize_with = "crate::deserialize_time"))]
    pub(crate) timestamp: OffsetDateTime,
    /// rotate time
    #[cfg_attr(feature = "json", serde(skip))]
    pub(crate) rotate_time: Option<OffsetDateTime>,
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Header {{ ... }}")
    }
}

impl Header {
    #[allow(deprecated)]
    pub fn new() -> Self {
        Header {
            version: Version::V2,
            flag: Flags::NONE,
            recorder_position: 0,
            compress: CompressKind::ZLIB,
            cipher: CipherKind::AES128GCM,
            cipher_hash: 0,
            timestamp: OffsetDateTime::now_utc().replace_nanosecond(0).unwrap_or_else(|_| OffsetDateTime::now_utc()),
            rotate_time: None,
        }
    }

    pub fn empty() -> Self {
        Header {
            version: Version::NONE,
            flag: Flags::NONE,
            recorder_position: 0,
            compress: CompressKind::NONE,
            cipher: CipherKind::NONE,
            cipher_hash: 0,
            timestamp: OffsetDateTime::UNIX_EPOCH,
            rotate_time: None,
        }
    }

    pub fn create(config: &EZLogConfig) -> Self {
        let time = OffsetDateTime::now_utc();
        let rotate_time = config.rotate_time(&time);

        let flag = if config.has_extra() {
            Flags::HAS_EXTRA
        } else {
            Flags::NONE
        };
        Header {
            version: config.version(),
            flag,
            recorder_position: 0,
            compress: config.compress_kind(),
            cipher: config.cipher_kind(),
            cipher_hash: config.cipher_hash(),
            timestamp: time,
            rotate_time: Some(rotate_time),
        }
    }

    pub fn max_length() -> usize {
        V2_LOG_HEADER_SIZE
    }

    #[inline]
    pub fn length_compat(version: &Version) -> usize {
        match version {
            Version::V1 => V1_LOG_HEADER_SIZE,
            Version::V2 => V2_LOG_HEADER_SIZE,
            _ => 0,
        }
    }

    pub fn length(&self) -> usize {
        Self::length_compat(&self.version)
    }

    pub fn encode(&self, writer: &mut dyn Write) -> std::result::Result<(), io::Error> {
        match self.version {
            Version::V1 => self.encode_v1(writer),
            Version::V2 => self.encode_v2(writer),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown version",
            )),
        }
    }

    pub fn encode_v1(&self, writer: &mut dyn Write) -> std::result::Result<(), io::Error> {
        writer.write_all(crate::FILE_SIGNATURE)?;
        writer.write_u8(self.version.into())?;
        writer.write_u8(self.flag.bits())?;
        writer.write_u32::<BigEndian>(self.recorder_position)?;
        writer.write_u8(self.compress.into())?;
        writer.write_u8(self.cipher.into())
    }

    pub fn encode_v2(&self, writer: &mut dyn Write) -> std::result::Result<(), io::Error> {
        writer.write_all(crate::FILE_SIGNATURE)?;
        writer.write_u8(self.version.into())?;
        writer.write_u8(self.flag.bits())?;
        writer.write_i64::<BigEndian>(self.timestamp.unix_timestamp())?;
        writer.write_u32::<BigEndian>(self.recorder_position)?;
        writer.write_u8(self.compress.into())?;
        writer.write_u8(self.cipher.into())?;
        writer.write_u32::<BigEndian>(self.cipher_hash)
    }

    pub fn decode_with_config(
        reader: &mut dyn Read,
        config: &EZLogConfig,
    ) -> std::result::Result<Self, errors::LogError> {
        let mut header = Self::decode(reader)?;
        header.rotate_time = Some(config.rotate_time(&header.timestamp));
        Ok(header)
    }

    pub fn decode(reader: &mut dyn Read) -> std::result::Result<Self, errors::LogError> {
        let mut signature = [0u8; 2];
        reader
            .read_exact(&mut signature)
            .map_err(|e| LogError::Parse(format!("sign read error {}", e)))?;
        let version = Version::from(reader.read_u8()?);
        let flag = Flags::from_bits(reader.read_u8()?).unwrap_or(Flags::NONE);
        let mut timestamp = OffsetDateTime::now_utc().unix_timestamp();
        if version == Version::V2 {
            timestamp = reader.read_i64::<BigEndian>()?
        }
        let recorder_size = reader.read_u32::<BigEndian>()?;

        let compress = reader.read_u8()?;
        let cipher = reader.read_u8()?;
        let mut hash: u32 = 0;

        if version == Version::V2 {
            hash = reader.read_u32::<BigEndian>()?;
        }
        Ok(Header {
            version,
            flag,
            recorder_position: recorder_size,
            compress: CompressKind::from(compress),
            cipher: CipherKind::from(cipher),
            cipher_hash: hash,
            timestamp: OffsetDateTime::from_unix_timestamp(timestamp)
                .unwrap_or_else(|_| OffsetDateTime::now_utc()),
            rotate_time: None,
        })
    }

    pub fn is_match(&self, config: &EZLogConfig) -> bool {
        self.version == config.version()
            && self.compress == config.compress_kind()
            && self.cipher == config.cipher_kind()
            && self.cipher_hash == config.cipher_hash()
    }

    pub fn is_none(&self) -> bool {
        self.version == Version::NONE
    }

    pub fn is_empty(&self) -> bool {
        self.recorder_position == 0
    }

    pub fn is_config(&self) -> bool {
        Into::<u8>::into(self.version) > Version::NONE.into()
    }

    pub fn has_record(&self) -> bool {
        self.recorder_position > self.length() as u32
    }

    pub fn has_record_exclude_extra(&self, config: &EZLogConfig) -> bool {
        // extra write as record
        self.recorder_position > (self.length() + self.extra_len(config)) as u32
    }

    pub fn has_extra(&self) -> bool {
        self.flag.contains(Flags::HAS_EXTRA)
    }

    #[inline]
    fn extra_len(&self, config: &EZLogConfig) -> usize {
        match &config.extra() {
            Some(e) => {
                let record = Vec::from(e.to_owned());
                encode_content(record).map(|r| r.len()).unwrap_or(0)
            }
            None => 0,
        }
    }

    pub fn is_extra_index(&self, position: u64) -> bool {
        self.flag.contains(Flags::HAS_EXTRA) && position == self.length() as u64
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub(crate) fn init_record_position(&mut self) {
        self.recorder_position = Self::length_compat(&self.version) as u32;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::*;

    #[test]
    fn test_header_encode_decode() {
        let header = Header::new();
        let mut buf = Vec::new();
        header.encode(&mut buf).unwrap();
        let decoded_header = Header::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(header, decoded_header);
    }

    #[test]
    fn test_header_encode_v2() {
        let header = Header::create(&EZLogConfig::default());
        let mut buf = Vec::new();
        header.encode_v2(&mut buf).unwrap();
        assert_eq!(buf.len(), V2_LOG_HEADER_SIZE);
    }

    #[test]
    fn test_ezlog_trim() {
        use std::fs;
        use std::time::Duration;

        use time::OffsetDateTime;

        // Initialize the logger
        crate::InitBuilder::new().debug(true).init();

        // Create a test log directory
        let test_dir = test_compat::test_path().join("ezlog_test");
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).unwrap();
        }
        fs::create_dir_all(&test_dir).unwrap();

        // Create a config with a very short trim duration (1 second)
        let config = crate::EZLogConfigBuilder::new()
            .dir_path(&test_dir)
            .name("trim_test")
            .trim_duration(time::Duration::days(1))
            .build();

        // Create the logger
        crate::create_log(config);

        // Create some test files with different dates
        let current_log = test_dir.join("trim_test.mmap");
        let old_log = test_dir.join("trim_test_2022_01_01.mmap");
        let recent_log = test_dir.join(format!(
            "trim_test_{}.mmap",
            OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Iso8601::DATE)
                .unwrap()
                .replace('-', "_")
        ));

        // Write some content to the files
        fs::write(&current_log, "current log content").unwrap();
        fs::write(&old_log, "old log content").unwrap();
        fs::write(&recent_log, "recent log content").unwrap();

        // Sleep to ensure the fs operations complete(windows ci sometimes need a moment to sync)
        std::thread::sleep(Duration::from_millis(1000));

        // Verify files exist before trimming
        assert!(current_log.exists(), "Current log file should exist");
        assert!(!old_log.exists(), "Old log file should not exist");
        assert!(recent_log.exists(), "Recent log file should exist");

        // Wait a bit to ensure trim duration passes
        std::thread::sleep(Duration::from_millis(10));

        // Perform the trim operation
        crate::trim();

        let (tx, rx) = channel::<()>();
        
        crate::post_msg(crate::EZMsg::Action(Box::new(move || {
           tx.send(()).unwrap_or_else(|e| eprintln!("Failed to send message: {:?}", e))
        })));

        rx.recv().unwrap_or_else(|e| eprintln!("Failed to receive message: {:?}", e));

        // Sleep to ensure the fs operations complete(windows ci sometimes need a moment to sync)
        std::thread::sleep(Duration::from_millis(1000));

        // Verify the results:
        // - Current log file should still exist (it's the active log)
        // - Old log file should be removed (it's out of date)
        // - Recent log file should still exist (it's not out of date)
        assert!(
            current_log.exists(),
            "Current log file should still exist after trim"
        );
        assert!(
            !old_log.exists(),
            "Old log file should be removed after trim"
        );
        assert!(
            recent_log.exists(),
            "Recent log file should still exist after trim"
        );

        // Clean up
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).unwrap();
        }
    }
}
