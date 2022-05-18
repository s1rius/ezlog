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

    pub fn refresh_inner(&mut self, time: OffsetDateTime, buf_size: usize) -> Result<(), LogError> {
        if let Some(_) = self.inner.should_rollover(time) {
            self.flush()?;
            *(&mut self.inner) = EZMmapAppendInner::new(self.config, time)?;
        }
        Ok(())
    }
}

pub struct EZMmapAppendInner {
    header: Header,
    mmap: MmapMut,
    next_date: AtomicUsize,
}

impl EZMmapAppendInner {
    pub fn new(config: &EZLogConfig, time: OffsetDateTime) -> Result<EZMmapAppendInner, LogError> {
        let file_name = config.now_file_name(time);
        let log_file = create_mmap_file(&config.dir_path, &file_name)?;
        let mut mmap = unsafe { MmapOptions::new().map_mut(&log_file)? };
        let mut c = Cursor::new(&mut mmap[0..V1_LOG_HEADER_SIZE]);
        let mut header = Header::decode(&mut c)?;
        if !header.is_valid() {
            header = Header::create(config);
        }
        let next_date = next_date(time);

        let inner = EZMmapAppendInner {
            header,
            mmap,
            next_date: AtomicUsize::new(next_date.unix_timestamp() as usize),
        };
        Ok(inner)
    }

    pub fn new_now(config: &EZLogConfig) -> Result<EZMmapAppendInner, LogError> {
        EZMmapAppendInner::new(config, OffsetDateTime::now_utc())
    }

    fn is_oversize(&self, buf_size: usize) -> Option<bool> {
        let max_len = self.mmap.len();
        let is_size_over =
            V1_LOG_HEADER_SIZE + self.header.recorder_size as usize + buf_size > max_len;
        Some(is_size_over)
    }

    fn should_rollover(&self, date: OffsetDateTime) -> Option<(usize)> {
        let next_date = self.next_date.load(Ordering::Acquire);
        // if the next date is 0, this appender *never* rotates log files.
        if next_date == 0 {
            return None;
        }

        if date.unix_timestamp() as usize >= next_date {
            return Some((next_date));
        }
        None
    }

    fn advance_date(&self, now: OffsetDateTime, current: usize) -> bool {
        let next_date = next_date(now).unix_timestamp() as usize;
        self.next_date
            .compare_exchange(current, next_date, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

impl Write for EZMmapAppender<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
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
            .build()
    }

    #[test]
    fn test_appender_inner_create() {
        let config = create_config();
        let inner = EZMmapAppendInner::new_now(&config).unwrap();
        assert_eq!(inner.should_rollover(OffsetDateTime::now_utc()), None);
        assert_ne!(
            inner.should_rollover(OffsetDateTime::now_utc() + Duration::days(1)),
            None
        );
    }

    #[test]
    fn test_appender_rollover() {
        let config = create_config();
        let mut appender = EZMmapAppender::new(&config).unwrap();
        appender
            .refresh_inner(OffsetDateTime::now_utc() + Duration::days(1), 0)
            .unwrap();
    }
}
