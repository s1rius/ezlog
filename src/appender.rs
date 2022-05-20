use std::path::PathBuf;

use time::OffsetDateTime;

use crate::*;

/// mmap 实现的[EZLog]
pub struct EZMmapAppender<'a> {
    config: &'a EZLogConfig,
    inner: EZMmapAppendInner,
}

impl<'a> EZMmapAppender<'a> {
    pub fn new(config: &'a EZLogConfig) -> Result<Self, LogError> {
        let inner = EZMmapAppendInner::new_now(config)?;
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
            self.inner = EZMmapAppendInner::new(self.config, time)?;
        }

        if self.inner.is_oversize(buf_size) {
            self.flush().ok();
            self.inner.rename_current_file()?;
            self.inner = EZMmapAppendInner::new(self.config, time)?;
        }
        Ok(())
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
        let file_name = config.now_file_name(time);
        let log_file =
            EZLogConfig::create_mmap_file(&config.dir_path, &file_name, config.max_size)?;
        let mut mmap = unsafe { MmapOptions::new().map_mut(&log_file)? };
        let mut c = Cursor::new(&mut mmap[0..V1_LOG_HEADER_SIZE]);
        let mut header = Header::decode(&mut c)?;
        if !header.is_valid(config) {
            header = Header::create(config);
        }
        let next_date = next_date(time);
        let file_path = Path::new(&config.dir_path).join(&file_name);

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
        return V1_LOG_HEADER_SIZE + self.header.recorder_size as usize + buf_size > max_len;
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

impl Write for EZMmapAppender<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.check_rolling(buf.len())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.mmap.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

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
    fn test_appender_inner_create() {
        let config = create_config();
        let inner = EZMmapAppendInner::new_now(&config).unwrap();
        assert!(!inner.is_overtime(OffsetDateTime::now_utc()));
        assert!(inner.is_overtime(OffsetDateTime::now_utc() + Duration::days(1)));
    }

    #[test]
    fn test_appender_rollover() {
        let config = create_config();
        let mut appender = EZMmapAppender::new(&config).unwrap();
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
}
