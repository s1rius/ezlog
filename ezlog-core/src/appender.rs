use std::{
    fs::OpenOptions,
    io::{
        BufReader,
        BufWriter,
        ErrorKind,
    },
    path::PathBuf,
};

use time::OffsetDateTime;

use crate::{
    events::event,
    logger::Header,
    *,
};

pub trait AppenderInner: Write + Send + Sync {
    /// check have enough space to write record
    fn is_oversize(&self, buf_size: usize) -> bool;

    /// Write to the file's path
    fn file_path(&self) -> &PathBuf;

    /// Log file length
    fn file_len(&self) -> usize;

    /// Get the header
    fn header(&self) -> &Header;

    /// Write header bytes to log file
    fn write_header_to_log(&mut self) -> std::result::Result<(), std::io::Error>;

    /// appender is overtime
    fn is_overtime(&self, time: OffsetDateTime) -> bool {
        self.header()
            .rotate_time
            .map(|rotate_time| time > rotate_time)
            .unwrap_or(true)
    }

    /// Log file init then write header and extra
    fn write_init(&mut self, config: &EZLogConfig) -> std::result::Result<(), std::io::Error> {
        if self.header().is_empty() {
            self.write_header_to_log()?;
            self.write_extra(config)?;
        }
        Ok(())
    }

    fn write_extra(&mut self, config: &EZLogConfig) -> std::result::Result<(), std::io::Error> {
        if let Some(extra) = config.extra() {
            if extra.is_empty() {
                return Ok(());
            }
            let content = logger::encode_content((extra.as_bytes()).to_vec()).unwrap_or_default();
            self.write_all(&content)?;
        }
        Ok(())
    }
}

/// # Appender 的实现
pub struct EZAppender {
    pub(crate) inner: RwLock<Box<dyn AppenderInner>>,
}

impl EZAppender {
    pub fn create_inner(config: &EZLogConfig) -> Result<Box<dyn AppenderInner>> {
        event!(Event::MapFile);
        match Self::create_mmap(config) {
            Ok(i) => {
                event!(Event::MapFileEnd);
                Ok(i)
            }
            Err(e) => {
                event!(Event::MapFileError, "mmap appender new", &e);
                Ok(Box::new(ByteArrayAppenderInner::new(config)?))
            }
        }
    }

    pub fn create_mmap(config: &EZLogConfig) -> Result<Box<dyn AppenderInner>> {
        MmapAppendInner::new(config).map(|inner| Box::new(inner) as Box<dyn AppenderInner>)
    }

    pub fn new(config: &EZLogConfig) -> Result<Self> {
        let inner = EZAppender::create_inner(config)?;
        Ok(Self {
            inner: RwLock::new(inner),
        })
    }

    #[inline]
    pub(crate) fn check_write_rolling(
        &self,
        buf_size: usize,
    ) -> std::result::Result<(), AppenderError> {
        let inner = self
            .get_inner()
            .map_err(|e| AppenderError::LockError(format!("Failed to acquire read lock: {}", e)))?;

        let now = OffsetDateTime::now_utc();
        if inner.is_overtime(now) {
            let rotate_time = inner.header().rotate_time;
            return Err(AppenderError::RotateTimeExceeded {
                current: now,
                rotate_time: rotate_time.unwrap_or_else(OffsetDateTime::now_utc),
            });
        }
        if inner.is_oversize(buf_size) {
            return Err(AppenderError::SizeExceeded {
                current: inner.header().recorder_position as usize,
                append: buf_size,
                max: inner.file_len(),
            });
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn check_config_rolling(&self, config: &EZLogConfig) -> Result<()> {
        // only hold the read-lock long enough to decide if we need to rotate
        let needs_rotation = {
            let inner = self.get_inner()?; // RwLockReadGuard<'_, _>
            inner.file_len() != config.max_size() as usize || !inner.header().is_match(config)
        };

        if needs_rotation {
            self.rotate(config)?;
        }
        Ok(())
    }

    pub(crate) fn rotate(&self, config: &EZLogConfig) -> Result<()> {
        // Acquire write lock and extract information needed for file rotation
        let mut inner = self.get_inner_mut().map_err(|e| {
            LogError::IoError(io::Error::other(format!(
                "Failed to acquire write lock for rotation: {e}"
            )))
        })?;

        // Save file path and timestamp before replacement
        let file_path = inner.file_path().to_owned();
        let header_time = inner.header().timestamp;

        let empty_inner = NopInner::empty();
        let old_inner = std::mem::replace(&mut *inner, Box::new(empty_inner));

        // flush the old one
        drop(old_inner);

        // Rename the old log file (now that we've released the lock)
        EZAppender::rename_current_file(config, &file_path, header_time).inspect_err(|e| {
            event!(Event::RotateFileError, "rename file error", e);
        })?;

        // Create a new inner appender before acquiring any locks
        let new_inner = Self::create_inner(config).inspect_err(|e| {
            event!(Event::RotateFileError, "create inner error", e);
        })?;

        // Replace the inner with the new one in a single operation
        let empty_inner = std::mem::replace(&mut *inner, new_inner);

        drop(empty_inner);
        event!(Event::RotateFile);
        Ok(())
    }

    // get the inner appender with read lock
    pub(crate) fn get_inner(
        &self,
    ) -> Result<std::sync::RwLockReadGuard<'_, Box<dyn AppenderInner>>> {
        let inner = self
            .inner
            .read()
            .map_err(|_| errors::LogError::IoError(io::Error::other("get appender inner error")))?;
        Ok(inner)
    }

    // get the inner appender with write lock
    pub(crate) fn get_inner_mut(
        &self,
    ) -> Result<std::sync::RwLockWriteGuard<'_, Box<dyn AppenderInner>>> {
        self.inner.write().map_err(|_| {
            errors::LogError::IoError(io::Error::other("get appender inner write lock error"))
        })
    }

    pub fn rename_current_file(
        config: &EZLogConfig,
        file_path: &PathBuf,
        time: OffsetDateTime,
    ) -> Result<()> {
        let mut count = 1;
        if !file_path.is_file() {
            return Err(errors::LogError::IoError(io::Error::new(
                ErrorKind::InvalidData,
                "current file is not valid!",
            )));
        }

        loop {
            let new_name = config.file_name_with_date(time, count)?;
            let new_path = file_path.with_file_name(new_name);
            if !new_path.exists() {
                std::fs::rename(file_path, &new_path)?;
                return Ok(());
            }
            count += 1;
        }
    }
}

impl Write for EZAppender {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.check_write_rolling(buf.len())
            .map_err(io::Error::other)?;

        let mut inner = self
            .get_inner_mut()
            .map_err(|e| io::Error::other(format!("Failed to acquire write lock: {}", e)))?;

        inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut inner = self
            .get_inner_mut()
            .map_err(|e| io::Error::other(format!("Failed to acquire write lock: {}", e)))?;
        inner.flush()
    }
}

pub(crate) struct MmapAppendInner {
    header: Header,
    file_path: PathBuf,
    mmap: MmapMut,
}

impl MmapAppendInner {
    pub(crate) fn new(config: &EZLogConfig) -> Result<Self> {
        let (mut file_path, mut mmap) = config.create_mmap_file()?;

        if mmap.len() < Header::max_length() {
            EZAppender::rename_current_file(config, &file_path, OffsetDateTime::now_utc())?;
            (file_path, mmap) = config.create_mmap_file()?;
        }

        let mmap_header = &mmap
            .get(0..Header::max_length())
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "mmap get header vec error"))?;
        let mut c = Cursor::new(mmap_header);
        let mut header = Header::decode(&mut c)?;

        let mut write_init = false;
        if header.is_none() {
            header = Header::create(config);
            write_init = true;
        }

        let mut inner = MmapAppendInner {
            header,
            file_path,
            mmap,
        };

        if write_init {
            inner.write_init(config)?;
        }

        Ok(inner)
    }

    fn write_buf(&mut self, buf: &[u8], start: usize) -> std::io::Result<usize> {
        let mmap_len = self.mmap.len();
        let m = self.mmap.get_mut(start..start + buf.len()).ok_or_else(|| {
            io::Error::new(
                ErrorKind::InvalidData,
                format!(
                    "invalid data write len = {}, start = {}, buf len = {}",
                    mmap_len,
                    start,
                    buf.len()
                ),
            )
        })?;
        let mut c = Cursor::new(m);
        c.write(buf)
    }
}

impl Write for MmapAppendInner {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let start = self.header.recorder_position as usize;
        self.header.recorder_position += buf.len() as u32;
        self.write_header_to_log()?;
        self.write_buf(buf, start)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.write_header_to_log()?;
        self.mmap.flush()
    }
}

impl AppenderInner for MmapAppendInner {
    fn is_oversize(&self, buf_size: usize) -> bool {
        let max_len = self.mmap.len();
        self.header.recorder_position as usize + buf_size > max_len
    }

    fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    fn header(&self) -> &Header {
        &self.header
    }

    fn write_header_to_log(&mut self) -> std::result::Result<(), std::io::Error> {
        if self.header.is_empty() {
            self.header.init_record_position();
        }
        let mmap_header = self.mmap.get_mut(0..self.header.length()).ok_or_else(|| {
            io::Error::new(ErrorKind::InvalidData, "mmap write header vec get error")
        })?;

        let mut c = Cursor::new(mmap_header);
        self.header.encode(&mut c)
    }

    fn file_len(&self) -> usize {
        self.mmap.len()
    }
}

impl Drop for MmapAppendInner {
    fn drop(&mut self) {
        self.flush().ok();
    }
}

struct ByteArrayAppenderInner {
    header: Header,
    file_path: PathBuf,
    byte_array: Vec<u8>,
}

impl ByteArrayAppenderInner {
    pub(crate) fn new(config: &EZLogConfig) -> Result<Self> {
        let (mut file, mut file_path) = config.create_or_open_log_file()?;
        if file.metadata()?.len() < config.max_size() {
            EZAppender::rename_current_file(config, &file_path, OffsetDateTime::now_utc())?;
            (file, file_path) = config.create_or_open_log_file()?;
        }
        let mut byte_array = vec![0u8; config.max_size() as usize];
        BufReader::new(&file).read_exact(&mut byte_array)?;

        let mut c = Cursor::new(byte_array.get(0..Header::max_length()).ok_or_else(|| {
            io::Error::new(ErrorKind::InvalidData, "byte array get header vec error")
        })?);
        let mut write_init = false;
        let mut header = Header::decode(&mut c)?;
        if header.is_none() {
            header = Header::create(config);
            write_init = true;
        }

        let mut inner = ByteArrayAppenderInner {
            header,
            file_path,
            byte_array,
        };

        if write_init {
            inner.write_init(config)?;
        }

        Ok(inner)
    }

    fn write_buf(&mut self, buf: &[u8], start: usize) -> std::io::Result<usize> {
        let byte_array_len = self.byte_array.len();
        let buf_write = self
            .byte_array
            .get_mut(start..start + buf.len())
            .ok_or_else(|| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "invalid data write len = {}, start = {}, buf len = {}",
                        byte_array_len,
                        start,
                        buf.len()
                    ),
                )
            })?;
        Cursor::new(buf_write).write(buf)
    }
}

impl Write for ByteArrayAppenderInner {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let start = self.header.recorder_position as usize;
        self.header.recorder_position += buf.len() as u32;
        self.write_header_to_log()?;
        self.write_buf(buf, start)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.write_header_to_log()?;
        let file = OpenOptions::new().write(true).open(self.file_path())?;
        let mut write = BufWriter::new(file);
        write.write_all(&self.byte_array)?;
        write.flush()
    }
}

impl AppenderInner for ByteArrayAppenderInner {
    fn is_oversize(&self, buf_size: usize) -> bool {
        let max_len = self.byte_array.len();
        self.header.recorder_position as usize + buf_size > max_len
    }

    fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    fn header(&self) -> &Header {
        &self.header
    }

    fn write_header_to_log(&mut self) -> std::result::Result<(), std::io::Error> {
        if self.header.is_empty() {
            self.header.init_record_position();
        }
        let header = self
            .byte_array
            .get_mut(0..self.header.length())
            .ok_or_else(|| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    "byte array write header vec get error",
                )
            })?;
        let mut c = Cursor::new(header);
        self.header.encode(&mut c)
    }

    fn file_len(&self) -> usize {
        self.byte_array.len()
    }
}

impl Drop for ByteArrayAppenderInner {
    fn drop(&mut self) {
        self.flush().ok();
    }
}

struct NopInner {
    file_path: PathBuf,
    header: Header,
}

impl NopInner {
    pub(crate) fn empty() -> Self {
        NopInner {
            file_path: PathBuf::new(),
            header: Header::new(),
        }
    }
}

impl AppenderInner for NopInner {
    fn is_oversize(&self, _buf_size: usize) -> bool {
        false
    }

    fn is_overtime(&self, _time: OffsetDateTime) -> bool {
        false
    }

    fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    fn header(&self) -> &Header {
        &self.header
    }

    fn write_header_to_log(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }

    fn file_len(&self) -> usize {
        0
    }
}

impl Write for NopInner {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppenderError {
    #[error("current size: {current}, append size: {append}, max size: {max}")]
    SizeExceeded {
        current: usize,
        append: usize,
        max: usize,
    },
    #[error("current time: {current}, rotate time: {rotate_time}")]
    RotateTimeExceeded {
        current: OffsetDateTime,
        rotate_time: OffsetDateTime,
    },
    #[error("lock error: {0}")]
    LockError(String),
}

#[cfg(test)]
mod tests {

    use std::fs::{
        self,
        File,
        OpenOptions,
    };
    use std::io::{
        BufReader,
        Seek,
        SeekFrom,
    };

    use time::Duration;

    use super::*;
    use crate::config::EZLogConfigBuilder;
    const KEY: &[u8; 32] = b"an example very very secret key.";
    const NONCE: &[u8; 12] = b"unique nonce";

    fn create_all_feature_config() -> EZLogConfigBuilder {
        EZLogConfigBuilder::new()
            .dir_path(
                test_compat::test_path()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(String::from("all_feature"))
            .file_suffix(String::from("mmap"))
            .max_size(1024)
            .compress(CompressKind::ZLIB)
            .cipher(CipherKind::AES128GCMSIV)
            .cipher_key(KEY.to_vec())
            .cipher_nonce(NONCE.to_vec())
    }

    fn current_file(path: &PathBuf) -> std::result::Result<File, errors::LogError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(path)?;
        Ok(file)
    }

    #[test]
    fn test_appender_inner_rolling() {
        let config_builder = create_all_feature_config();
        let builder_clone = config_builder.clone();
        let config = config_builder.build();
        let inner = MmapAppendInner::new(&config).unwrap();
        test_inner_rolling(&inner, &builder_clone);
        let mut file_path = inner.file_path().to_owned();
        drop(inner);
        fs::remove_file(file_path).unwrap();

        let inner = ByteArrayAppenderInner::new(&config).unwrap();
        test_inner_rolling(&inner, &builder_clone);
        file_path = inner.file_path().to_owned();
        fs::remove_file(file_path).unwrap();
    }

    fn test_inner_rolling(inner: &dyn AppenderInner, config_builder: &EZLogConfigBuilder) {
        let config = config_builder.clone().build();
        let max_size: usize = config.max_size() as usize;
        assert!(inner.is_oversize(max_size - inner.header().length() + 1));
        assert!(!inner.is_oversize(max_size - inner.header().length()));
        assert!(
            inner.is_overtime(inner.header().timestamp + Duration::days(1) + Duration::seconds(1))
        );
        assert!(!inner.is_overtime(inner.header().timestamp + Duration::hours(23)));
        assert!(inner.header().is_match(&config));

        let no_cipher_config = config_builder.clone().cipher(CipherKind::NONE).build();
        assert!(!inner.header().is_match(&no_cipher_config));

        let diff_nonce_config = config_builder.clone().cipher_key(vec![1]).build();
        assert!(!inner.header().is_match(&diff_nonce_config));

        let diff_version_config = config_builder.clone().version(Version::V1).build();
        assert!(!inner.header().is_match(&diff_version_config));

        let diff_compress_config = config_builder.clone().compress(CompressKind::NONE).build();
        assert!(!inner.header().is_match(&diff_compress_config));
    }

    #[test]
    fn test_appender_write() {
        let buf = b"hello an other log, let's go";

        let config = EZLogConfigBuilder::new()
            .dir_path(
                test_compat::test_path()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(String::from("test_write"))
            .file_suffix(String::from("mmap"))
            .max_size(1024)
            .build();

        let mut appender = MmapAppendInner::new(&config).unwrap();
        appender.write(buf).unwrap();
        appender.flush().unwrap();

        let mut read_buf = vec![0u8; buf.len()];
        let file = current_file(appender.file_path()).unwrap();
        let mut reader: BufReader<File> = BufReader::new(file);
        reader
            .seek(SeekFrom::Start(
                Header::length_compat(&config.version()) as u64
            ))
            .unwrap();
        reader.read(&mut read_buf).unwrap();

        assert_eq!(read_buf, buf);
        let p = appender.file_path().clone();
        drop(appender);
        fs::remove_file(p).unwrap();

        let c = EZLogConfigBuilder::new()
            .dir_path(
                test_compat::test_path()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(String::from("test_write1"))
            .file_suffix(String::from("mmap"))
            .max_size(1024)
            .build();

        let mut appender = ByteArrayAppenderInner::new(&c).unwrap();
        appender.write(buf).unwrap();
        appender.flush().unwrap();

        let log_path = appender.file_path().clone();

        let mut read_buf = vec![0u8; buf.len()];
        let file = current_file(&log_path).unwrap();
        let mut reader = BufReader::new(file);
        reader
            .seek(SeekFrom::Start(
                Header::length_compat(&config.version()) as u64
            ))
            .unwrap();
        reader.read_exact(&mut read_buf).unwrap();
        assert_eq!(read_buf, buf);
        fs::remove_file(appender.file_path()).unwrap();
    }

    #[test]
    fn test_appender_rotate() {
        let config = EZLogConfigBuilder::new()
            .dir_path(
                test_compat::test_path()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(String::from("rorate"))
            .file_suffix(String::from("mmap"))
            .max_size(1024)
            .compress(CompressKind::ZLIB)
            .cipher(CipherKind::AES128GCMSIV)
            .cipher_key(KEY.to_vec())
            .cipher_nonce(NONCE.to_vec())
            .build();

        let appender: EZAppender = EZAppender {
            inner: EZAppender::create_mmap(&config).unwrap().into(),
        };

        for _i in 0..9 {
            appender.rotate(&config).unwrap();
        }

        let mut count = 0;
        for entry in fs::read_dir(test_compat::test_path()).unwrap() {
            let entry = &entry.unwrap();
            if entry.path().is_file() {
                let file_name_os_str = entry.file_name();
                let file_name = file_name_os_str.to_string_lossy();
                if file_name.contains("rorate") {
                    count += 1;
                    fs::remove_file(entry.path()).unwrap();
                }
            }
        }
        assert!(count == 10);
    }
}
