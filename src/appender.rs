use std::{path::PathBuf, rc::Rc};

use time::OffsetDateTime;

use crate::*;

/// mmap 实现的[EZLog]
pub struct EZMmapAppender {
    config: Rc<EZLogConfig>,
    inner: EZMmapAppendInner,
}

impl EZMmapAppender {
    pub fn new(config: Rc<EZLogConfig>) -> Result<Self, LogError> {
        let inner = EZMmapAppendInner::new_now(&config)?;
        Ok(Self { config, inner })
    }

    pub fn check_rolling(&mut self, buf_size: usize) -> Result<(), LogError> {
        self.check_refresh_inner(OffsetDateTime::now_utc(), buf_size)
    }

    pub fn check_refresh_inner(
        &mut self,
        time: OffsetDateTime,
        buf_size: usize,
    ) -> Result<(), LogError> {
        if self.inner.is_overtime(time) {
            self.flush().ok();
            self.inner = EZMmapAppendInner::new(&self.config, time)?;
        }

        if self.inner.is_oversize(buf_size) {
            self.flush().ok();
            self.inner.rename_current_file()?;
            self.inner = EZMmapAppendInner::new(&self.config, time)?;
        }
        Ok(())
    }
}

impl Write for EZMmapAppender {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.check_rolling(buf.len())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub struct EZMmapAppendInner {
    header: Header,
    file_path: PathBuf,
    mmap: MmapMut,
    next_date: i64,
}

impl EZMmapAppendInner {
    pub fn new(config: &EZLogConfig, time: OffsetDateTime) -> Result<EZMmapAppendInner, LogError> {
        let (log_file, file_path) = config.create_mmap_file(time)?;
        let mut mmap = unsafe { MmapOptions::new().map_mut(&log_file)? };
        let mut c = Cursor::new(&mut mmap[0..V1_LOG_HEADER_SIZE]);
        let mut header = Header::decode(&mut c).unwrap_or(Header::new());
        if !header.is_valid(&config) {
            // todo
            // if not match create new file?
            header = Header::create(&config);
        }
        let next_date = next_date(time);

        let inner = EZMmapAppendInner {
            header,
            file_path,
            mmap,
            next_date: next_date.unix_timestamp(),
        };
        Ok(inner)
    }

    pub fn new_now(config: &EZLogConfig) -> Result<EZMmapAppendInner, LogError> {
        EZMmapAppendInner::new(config, OffsetDateTime::now_utc())
    }

    fn is_oversize(&self, buf_size: usize) -> bool {
        let max_len = self.mmap.len();
        return V1_LOG_HEADER_SIZE + self.header.recorder_position as usize + buf_size > max_len;
    }

    fn is_overtime(&self, time: OffsetDateTime) -> bool {
        return time.unix_timestamp() > self.next_date;
    }

    fn advance_date(&mut self, now: OffsetDateTime) {
        self.next_date = next_date(now).unix_timestamp();
    }

    fn current_file(&self) -> Result<File, errors::LogError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(&self.file_path)?;
        Ok(file)
    }

    pub fn rename_current_file(&self) -> Result<(), errors::LogError> {
        let mut count = 1;
        loop {
            if let Some(ext) = self.file_path.extension() {
                let new_ext = format!("{}.{}", count, ext.to_str().unwrap_or_else(|| { "mmap" }));
                let new_path = self.file_path.with_extension(new_ext);
                if !new_path.exists() {
                    std::fs::rename(&self.file_path, &new_path)?;
                    return Ok(());
                }
            }
            count += 1;
        }
    }
}

impl Write for EZMmapAppendInner {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let start = self.header.recorder_position as usize;
        let end = start + buf.len();
        self.header.recorder_position += buf.len() as u32;
        let mut cursor = Cursor::new(&mut self.mmap[start..end]);
        cursor.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut c = Cursor::new(&mut self.mmap[0..V1_LOG_HEADER_SIZE]);
        self.header.encode(&mut c).or(Err(io::Error::new(
            io::ErrorKind::Other,
            "header encode error",
        )))?;
        self.mmap.flush_async()
    }
}

#[cfg(test)]
mod tests {

    use std::io::BufReader;

    use super::*;

    fn create_config() -> EZLogConfig {
        EZLogConfigBuilder::new()
            .dir_path(
                dirs::desktop_dir()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(String::from("test"))
            .file_suffix(String::from("mmap"))
            .max_size(1024)
            .build()
    }

    fn create_all_feature_config() -> EZLogConfig {
        let key = b"an example very very secret key.";
        let nonce = b"unique nonce";
        EZLogConfigBuilder::new()
            .dir_path(
                dirs::desktop_dir()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(String::from("all_feature"))
            .file_suffix(String::from("mmap"))
            .max_size(1024)
            .compress(CompressKind::ZLIB)
            .cipher(CipherKind::AES128GCM)
            .cipher_key(key.to_vec())
            .cipher_nonce(nonce.to_vec())
            .build()
    }

    #[test]
    fn test_appender_inner_create() {
        let config = create_config();
        let inner = EZMmapAppendInner::new_now(&config).unwrap();
        assert!(!inner.is_overtime(OffsetDateTime::now_utc()));
        assert!(inner.is_overtime(OffsetDateTime::now_utc() + Duration::days(1)));
    }

    #[test]
    fn test_appender_rollover() {
        let config = Rc::new(create_config());
        let mut appender = EZMmapAppender::new(Rc::clone(&config)).unwrap();
        appender
            .check_refresh_inner(OffsetDateTime::now_utc() + Duration::days(1), 0)
            .unwrap();

        appender
            .check_refresh_inner(OffsetDateTime::now_utc() + Duration::days(1), 1025)
            .unwrap();

        appender
            .check_refresh_inner(OffsetDateTime::now_utc() + Duration::days(1), 1025)
            .unwrap();
    }

    #[test]
    fn test_write() {
        let buf = b"hello an other log, let's go";
        let config = Rc::new(create_config());
        let mut appender = EZMmapAppender::new(Rc::clone(&config)).unwrap();
        appender.write(buf).unwrap();
        appender.flush().unwrap();

        let mut read_buf = vec![0u8; buf.len()];
        let file = appender.inner.current_file().unwrap();
        let mut reader = BufReader::new(file);
        reader.seek_relative(V1_LOG_HEADER_SIZE as i64).unwrap();
        reader.read(&mut read_buf).unwrap();
        assert_eq!(read_buf, buf);
    }
}
