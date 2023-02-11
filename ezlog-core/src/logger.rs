use byteorder::{BigEndian, WriteBytesExt};
use integer_encoding::VarIntWriter;
#[cfg(feature = "decode")]
use std::io::BufRead;
use std::io::Read;
use std::path::PathBuf;

use std::{fs, io};
use std::{io::Write, rc::Rc};

use crate::events::Event::{self};
use crate::{
    appender::EZAppender,
    compress::ZlibCodec,
    crypto::{Aes128Gcm, Aes256Gcm},
    errors::LogError,
    CipherKind, Compress, CompressKind, Cryptor, EZLogConfig, EZRecord, RECORD_SIGNATURE_END,
    RECORD_SIGNATURE_START,
};
use crate::{errors, event, V1_LOG_HEADER_SIZE};
use crate::{Version, V2_LOG_HEADER_SIZE};
use byteorder::ReadBytesExt;
#[cfg(feature = "decode")]
use integer_encoding::VarIntReader;
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime};

type Result<T> = std::result::Result<T, LogError>;

pub struct EZLogger {
    pub config: Rc<EZLogConfig>,
    pub appender: Box<dyn Write>,
    compression: Option<Box<dyn Compress>>,
    cryptor: Option<Box<dyn Cryptor>>,
}

impl EZLogger {
    pub fn new(config: EZLogConfig) -> Result<Self> {
        let rc_conf = Rc::new(config);
        let appender = Box::new(EZAppender::new(Rc::clone(&rc_conf))?);
        let compression = EZLogger::create_compress(&rc_conf);
        let cryptor = EZLogger::create_cryptor(&rc_conf)?;

        Ok(Self {
            config: Rc::clone(&rc_conf),
            appender,
            compression,
            cryptor,
        })
    }

    pub fn create_cryptor(config: &EZLogConfig) -> Result<Option<Box<dyn Cryptor>>> {
        if let Some(key) = &config.cipher_key {
            if let Some(nonce) = &config.cipher_nonce {
                match config.cipher {
                    CipherKind::AES128GCM => {
                        let encryptor = Aes128Gcm::new(key, nonce)?;
                        Ok(Some(Box::new(encryptor)))
                    }
                    CipherKind::AES256GCM => {
                        let encryptor = Aes256Gcm::new(key, nonce)?;
                        Ok(Some(Box::new(encryptor)))
                    }
                    CipherKind::NONE => Ok(None),
                    CipherKind::UNKNOWN => Ok(None),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn create_compress(config: &EZLogConfig) -> Option<Box<dyn Compress>> {
        match config.compress {
            CompressKind::ZLIB => Some(Box::new(ZlibCodec::new(&config.compress_level))),
            CompressKind::NONE => None,
            CompressKind::UNKNOWN => None,
        }
    }

    pub(crate) fn append(&mut self, record: &EZRecord) -> Result<()> {
        let mut e: Option<LogError> = None;
        if record.content().len() > self.config.max_size as usize / 2 {
            record.trunks(&self.config).iter().for_each(|record| {
                match self.encode_as_block(record) {
                    Ok(buf) => match self.appender.write_all(&buf) {
                        Ok(_) => {}
                        Err(err) => e = Some(LogError::IoError(err)),
                    },
                    Err(err) => {
                        e = Some(err);
                    }
                }
            })
        } else {
            let buf = self.encode_as_block(record)?;
            self.appender.write_all(&buf)?;
        }
        if let Some(err) = e {
            Err(err)
        } else {
            Ok(())
        }
    }

    #[inline]
    fn encode(&mut self, record: &EZRecord) -> Result<Vec<u8>> {
        let mut buf = self.format(record);
        if self.config.version == Version::V1 {
            if let Some(encryptor) = &self.cryptor {
                event!(Event::Encrypt, &record.t_id());
                buf = encryptor.encrypt(&buf)?;
                event!(Event::EncryptEnd, &record.t_id());
            }
            if let Some(compression) = &self.compression {
                event!(Event::Compress, &record.t_id());
                buf = compression.compress(&buf).map_err(LogError::Compress)?;
                event!(Event::CompressEnd, &record.t_id());
            }
        } else {
            if let Some(compression) = &self.compression {
                event!(Event::Compress, &record.t_id());
                buf = compression.compress(&buf).map_err(LogError::Compress)?;
                event!(Event::CompressEnd, &record.t_id());
            }
            if let Some(encryptor) = &self.cryptor {
                event!(Event::Encrypt, &record.t_id());
                buf = encryptor.encrypt(&buf)?;
                event!(Event::EncryptEnd, &record.t_id());
            }
        }
        Ok(buf)
    }

    ///
    #[inline]
    pub fn encode_as_block(&mut self, record: &EZRecord) -> Result<Vec<u8>> {
        let buf = self.encode(record)?;
        Self::encode_content(buf)
    }

    #[inline]
    pub(crate) fn encode_content(mut buf: Vec<u8>) -> Result<Vec<u8>> {
        let mut chunk: Vec<u8> = Vec::new();
        chunk.push(RECORD_SIGNATURE_START);
        let size = buf.len();
        let mut size_chunk = Self::create_size_chunk(size)?;
        chunk.append(&mut size_chunk);
        chunk.append(&mut buf);
        chunk.push(RECORD_SIGNATURE_END);
        Ok(chunk)
    }

    #[inline]
    pub(crate) fn create_size_chunk(size: usize) -> Result<Vec<u8>> {
        let mut chunk: Vec<u8> = Vec::new();
        chunk.write_varint(size)?;
        Ok(chunk)
    }

    #[inline]
    #[cfg(feature = "decode")]
    pub fn decode_record(&mut self, reader: &mut dyn BufRead) -> Result<Vec<u8>> {
        Self::decode_record_from_read(
            reader,
            &self.config.version,
            &self.compression,
            &self.cryptor,
        )
    }

    #[inline]
    #[cfg(feature = "decode")]
    pub fn decode_body_and_write(
        reader: &mut dyn BufRead,
        writer: &mut dyn Write,
        version: &Version,
        compression: &Option<Box<dyn Compress>>,
        cryptor: &Option<Box<dyn Cryptor>>,
    ) -> io::Result<()> {
        loop {
            match Self::decode_record_from_read(reader, version, compression, cryptor) {
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

    #[cfg(feature = "decode")]
    pub(crate) fn decode_record_from_read(
        reader: &mut dyn BufRead,
        version: &Version,
        compression: &Option<Box<dyn Compress>>,
        cryptor: &Option<Box<dyn Cryptor>>,
    ) -> Result<Vec<u8>> {
        let chunk = Self::decode_record_to_content(reader, version)?;
        Self::decode_record_content(version, &chunk, compression, cryptor)
    }

    #[inline]
    #[cfg(feature = "decode")]
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
        let content_size: usize = Self::decode_record_size(reader, version)?;
        let mut chunk = vec![0u8; content_size];
        reader.read_exact(&mut chunk)?;
        let end_sign = reader.read_u8()?;
        if RECORD_SIGNATURE_END != end_sign {
            return Err(LogError::Parse("record end sign error".to_string()));
        }
        Ok(chunk)
    }

    #[inline]
    #[cfg(feature = "decode")]
    pub(crate) fn decode_record_size(
        mut reader: &mut dyn BufRead,
        version: &Version,
    ) -> Result<usize> {
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
    #[cfg(feature = "decode")]
    pub fn decode_record_content(
        version: &Version,
        chunk: &[u8],
        compression: &Option<Box<dyn Compress>>,
        cryptor: &Option<Box<dyn Cryptor>>,
    ) -> Result<Vec<u8>> {
        let mut buf = chunk.to_vec();

        if *version == Version::V1 {
            if let Some(decompression) = compression {
                buf = decompression.decompress(&buf)?;
            }

            if let Some(decryptor) = cryptor {
                buf = decryptor.decrypt(&buf)?;
            }
        } else {
            if let Some(decryptor) = cryptor {
                buf = decryptor.decrypt(&buf)?;
            }

            if let Some(decompression) = compression {
                buf = decompression.decompress(&buf)?;
            }
        }

        Ok(buf)
    }

    fn format(&self, record: &EZRecord) -> Vec<u8> {
        let time = record
            .time()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string());
        format!(
            "\n{} {} {} [{}:{}] {}",
            time,
            record.level(),
            record.target(),
            record.thread_name(),
            record.thread_id(),
            record.content()
        )
        .into_bytes()
    }

    pub(crate) fn flush(&mut self) -> std::result::Result<(), io::Error> {
        self.appender.flush()
    }

    pub(crate) fn trim(&self) {
        match fs::read_dir(&self.config.dir_path) {
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
                                                    Event::TrimError,
                                                    "remove file err",
                                                    &e.into()
                                                )
                                            });
                                        }
                                    }
                                    Err(e) => {
                                        event!(Event::TrimError, "judge file out of date error", &e)
                                    }
                                }
                            };
                        }
                        Err(e) => {
                            event!(Event::TrimError, "traversal file error", &e.into())
                        }
                    }
                }
            }
            Err(e) => event!(Event::TrimError, "read dir error", &e.into()),
        }
    }

    pub fn query_log_files_for_date(&self, date: Date) -> Vec<PathBuf> {
        self.config.query_log_files_for_date(date)
    }
}

/// EZLog file Header
///
/// every log file starts with a header,
/// which is used to describe the version, log length, compress type, cipher kind and so on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Header {
    /// version code
    pub(crate) version: Version,
    /// unused flag
    pub(crate) flag: u8,
    /// current log file write position
    pub(crate) recorder_position: u32,
    /// compress type
    pub(crate) compress: CompressKind,
    /// cipher kind
    pub(crate) cipher: CipherKind,
    /// timestamp
    pub(crate) timestamp: OffsetDateTime,
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

impl Header {
    pub fn new() -> Self {
        Header {
            version: Version::V2,
            flag: 0,
            recorder_position: V2_LOG_HEADER_SIZE as u32,
            compress: CompressKind::ZLIB,
            cipher: CipherKind::AES128GCM,
            timestamp: OffsetDateTime::now_utc(),
        }
    }

    pub fn create(config: &EZLogConfig) -> Self {
        Header {
            version: config.version,
            flag: 0,
            recorder_position: Header::length_compat(&config.version) as u32,
            compress: config.compress,
            cipher: config.cipher,
            timestamp: OffsetDateTime::now_utc(),
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
            Version::UNKNOWN => 0,
        }
    }

    pub fn length(&self) -> usize {
        Self::length_compat(&self.version)
    }

    pub fn encode(&self, writer: &mut dyn Write) -> std::result::Result<(), io::Error> {
        match self.version {
            Version::V1 => self.encode_v1(writer),
            Version::V2 => self.encode_v2(writer),
            Version::UNKNOWN => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown version",
            )),
        }
    }

    pub fn encode_v1(&self, writer: &mut dyn Write) -> std::result::Result<(), io::Error> {
        writer.write_all(crate::FILE_SIGNATURE)?;
        writer.write_u8(self.version.into())?;
        writer.write_u8(self.flag)?;
        writer.write_u32::<BigEndian>(self.recorder_position)?;
        writer.write_u8(self.compress.into())?;
        writer.write_u8(self.cipher.into())
    }

    pub fn encode_v2(&self, writer: &mut dyn Write) -> std::result::Result<(), io::Error> {
        self.encode_v1(writer)?;
        writer.write_i64::<BigEndian>(self.timestamp.unix_timestamp())
    }

    pub fn decode(reader: &mut dyn Read) -> std::result::Result<Self, errors::LogError> {
        let mut signature = [0u8; 2];
        reader.read_exact(&mut signature)?;
        let version = Version::from(reader.read_u8()?);
        let flag = reader.read_u8()?;
        let mut recorder_size = reader.read_u32::<BigEndian>()?;
        if recorder_size < Header::length_compat(&version) as u32 {
            recorder_size = Header::length_compat(&version) as u32;
        }

        let compress = reader.read_u8()?;
        let cipher = reader.read_u8()?;
        let timestamp = if version == Version::V2 {
            reader.read_i64::<BigEndian>()?
        } else {
            OffsetDateTime::now_utc().unix_timestamp()
        };
        Ok(Header {
            version,
            flag,
            recorder_position: recorder_size,
            compress: CompressKind::from(compress),
            cipher: CipherKind::from(cipher),
            timestamp: OffsetDateTime::from_unix_timestamp(timestamp)
                .unwrap_or_else(|_| OffsetDateTime::now_utc()),
        })
    }

    pub fn is_valid(&self, config: &EZLogConfig) -> bool {
        self.version == config.version
            && self.compress == config.compress
            && self.cipher == config.cipher
    }

    pub fn is_empty(&self) -> bool {
        self.version == Version::UNKNOWN || self.recorder_position <= self.length() as u32
    }

    pub fn has_record(&self) -> bool {
        self.recorder_position > self.length() as u32
    }

    pub fn version(&self) -> &Version {
        &self.version
    }
}
