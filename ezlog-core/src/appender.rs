use std::{path::PathBuf, rc::Rc};

use std::fs::{File, OpenOptions};
use time::OffsetDateTime;

use crate::*;

/// mmap 实现的[EZLog]
pub struct EZMmapAppender {
    config: Rc<EZLogConfig>,
    inner: EZMmapAppendInner,
}

impl EZMmapAppender {
    pub fn new(config: Rc<EZLogConfig>) -> Result<Self> {
        let inner = EZMmapAppendInner::new_now(&config)?;
        Ok(Self { config, inner })
    }

    pub fn check_rolling(&mut self, buf_size: usize) -> Result<()> {
        self.check_refresh_inner(OffsetDateTime::now_utc(), buf_size)
    }

    pub fn check_refresh_inner(&mut self, time: OffsetDateTime, buf_size: usize) -> Result<()> {
        if self.inner.is_overtime(time) {
            self.flush().ok();
            self.inner = EZMmapAppendInner::new(&self.config, time)?;
        }

        if self.inner.is_oversize(buf_size) {
            self.flush().ok();
            rename_current_file(&self.inner.file_path)?;
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
    pub fn new(config: &EZLogConfig, time: OffsetDateTime) -> Result<EZMmapAppendInner> {
        let (mut file_path, mut mmap) = config.create_mmap_file(time)?;
        let mut c = Cursor::new(&mut mmap[0..V1_LOG_HEADER_SIZE]);
        let mut header = Header::decode(&mut c).unwrap_or_else(|_| Header::new());
        let next_date = next_date(time);

        if header.is_empty() {
            header = Header::create(config);
            encode_header(&header, &mut mmap)?
        } else if !header.is_empty() && !header.is_valid(config) {
            rename_current_file(&file_path)?;
            (file_path, mmap) = config.create_mmap_file(time)?;
            header = Header::create(config);
            encode_header(&header, &mut mmap)?
        }

        let inner = EZMmapAppendInner {
            header,
            file_path,
            mmap,
            next_date: next_date.unix_timestamp(),
        };
        Ok(inner)
    }

    pub fn new_now(config: &EZLogConfig) -> Result<EZMmapAppendInner> {
        EZMmapAppendInner::new(config, OffsetDateTime::now_utc())
    }

    fn is_oversize(&self, buf_size: usize) -> bool {
        let max_len = self.mmap.len();
        V1_LOG_HEADER_SIZE + self.header.recorder_position as usize + buf_size > max_len
    }

    fn is_overtime(&self, time: OffsetDateTime) -> bool {
        time.unix_timestamp() > self.next_date
    }

    #[allow(dead_code)]
    fn current_file(&self) -> std::result::Result<File, errors::LogError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(&self.file_path)?;
        Ok(file)
    }
}

impl Write for EZMmapAppendInner {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let start = self.header.recorder_position as usize;
        let end = start + buf.len();
        self.header.recorder_position += buf.len() as u32;
        encode_header(&self.header, &mut self.mmap)?;
        let mut cursor = Cursor::new(&mut self.mmap[start..end]);
        cursor.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        encode_header(&self.header, &mut self.mmap)?;
        self.mmap.flush()
    }
}

pub fn rename_current_file(file_path: &PathBuf) -> Result<()> {
    let mut count = 1;
    loop {
        if let Some(ext) = file_path.extension() {
            let new_ext = format!("{}.{}", count, ext.to_str().unwrap_or("mmap"));
            let new_path = file_path.with_extension(new_ext);
            if !new_path.exists() {
                std::fs::rename(file_path, &new_path)?;
                return Ok(());
            }
        }
        count += 1;
    }
}

fn encode_header(header: &Header, mmap: &mut MmapMut) -> std::result::Result<(), std::io::Error> {
    let mut c = Cursor::new(&mut mmap[0..V1_LOG_HEADER_SIZE]);
    header.encode(&mut c)
}

#[cfg(test)]
mod tests {

    use std::io::BufReader;

    use crate::config::EZLogConfigBuilder;

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

    #[test]
    fn create_all_feature_config() {
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
            .build();
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
