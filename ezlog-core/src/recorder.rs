use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    thread,
};

use crate::{EZLogConfig, Level, DEFAULT_LOG_NAME};
#[cfg(feature = "log")]
use log::Record;
use time::OffsetDateTime;

/// Single Log record
#[derive(Debug, Clone)]
pub struct EZRecord {
    id: u64,
    log_name: String,
    level: Level,
    target: String,
    time: OffsetDateTime,
    thread_id: usize,
    thread_name: String,
    content: String,
    file: Option<String>,
    line: Option<u32>,
}

impl EZRecord {
    #[inline]
    pub fn builder() -> EZRecordBuilder {
        EZRecordBuilder::new()
    }

    #[inline]
    pub fn level(&self) -> Level {
        self.level
    }

    #[inline]
    pub fn target(&self) -> &str {
        self.target.as_str()
    }

    #[inline]
    pub fn timestamp(&self) -> i64 {
        self.time.unix_timestamp()
    }

    #[inline]
    pub fn thread_id(&self) -> usize {
        self.thread_id
    }

    #[inline]
    pub fn thread_name(&self) -> &str {
        self.thread_name.as_str()
    }

    #[inline]
    pub fn content(&self) -> &str {
        self.content.as_str()
    }

    pub fn log_name(&self) -> &str {
        &self.log_name
    }

    pub fn time(&self) -> &OffsetDateTime {
        &self.time
    }

    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }

    pub fn line(&self) -> Option<u32> {
        self.line
    }

    #[inline]
    pub fn to_builder(&self) -> EZRecordBuilder {
        EZRecordBuilder {
            record: EZRecord {
                id: self.id,
                log_name: self.log_name.clone(),
                level: self.level,
                target: self.target.clone(),
                time: self.time,
                thread_id: self.thread_id,
                thread_name: self.thread_name.clone(),
                content: self.content.clone(),
                file: self.file.clone(),
                line: self.line,
            },
        }
    }

    #[inline]
    pub fn to_trunk_builder(&self) -> EZRecordBuilder {
        EZRecordBuilder {
            record: EZRecord {
                id: 0,
                log_name: self.log_name.clone(),
                level: self.level,
                target: self.target.clone(),
                time: self.time,
                thread_id: self.thread_id,
                thread_name: self.thread_name.clone(),
                content: "".into(),
                file: self.file.clone(),
                line: self.line,
            },
        }
    }

    #[cfg(feature = "log")]
    pub(crate) fn from(r: &Record) -> Self {
        let t = thread::current();
        let t_id = thread_id::get();
        let t_name = t.name().unwrap_or_default();
        EZRecordBuilder::new()
            .log_name(DEFAULT_LOG_NAME.to_string())
            .level(r.metadata().level().into())
            .target(r.target().to_string())
            .time(OffsetDateTime::now_utc())
            .thread_id(t_id)
            .thread_name(t_name.to_string())
            .content(format!("{}", r.args()))
            .line(r.line().unwrap_or(0))
            .file(r.file().unwrap_or_default())
            .build()
    }

    pub fn t_id(&self) -> String {
        format!("{}_{}", self.log_name, &self.id)
    }

    /// get EZRecord unique id
    pub fn id(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.content.hash(&mut hasher);
        self.time.hash(&mut hasher);
        hasher.finish()
    }

    pub fn trunks(&self, config: &EZLogConfig) -> Vec<EZRecord> {
        let mut trunks: Vec<EZRecord> = Vec::new();
        let mut split_content: Vec<char> = Vec::new();
        let mut size = 0;
        let chars = self.content.chars();
        chars.for_each(|c| {
            size += c.len_utf8();
            if size > config.max_size as usize / 2 {
                let ez = self
                    .to_trunk_builder()
                    .content(split_content.iter().collect::<String>())
                    .build();
                trunks.push(ez);
                split_content.clear();
                size = c.len_utf8();
                split_content.push(c)
            } else {
                split_content.push(c);
            }
        });
        if !split_content.is_empty() {
            let ez = self
                .to_trunk_builder()
                .content(String::from_iter(&split_content))
                .build();
            trunks.push(ez);
        }
        trunks
    }
}

/// [EZRecord]'s builder
#[derive(Debug)]
pub struct EZRecordBuilder {
    record: EZRecord,
}

impl EZRecordBuilder {
    pub fn new() -> EZRecordBuilder {
        EZRecordBuilder::default()
    }

    pub fn level(&mut self, level: Level) -> &mut Self {
        self.record.level = level;
        self
    }

    pub fn target(&mut self, target: String) -> &mut Self {
        self.record.target = target;
        self
    }

    pub fn timestamp(&mut self, timestamp: i64) -> &mut Self {
        let time = OffsetDateTime::from_unix_timestamp(timestamp)
            .unwrap_or_else(|_| OffsetDateTime::now_utc());
        self.record.time = time;
        self
    }

    pub fn time(&mut self, time: OffsetDateTime) -> &mut Self {
        self.record.time = time;
        self
    }

    pub fn thread_id(&mut self, thread_id: usize) -> &mut Self {
        self.record.thread_id = thread_id;
        self
    }

    pub fn thread_name(&mut self, thread_name: String) -> &mut Self {
        self.record.thread_name = thread_name;
        self
    }

    pub fn content(&mut self, content: String) -> &mut Self {
        self.record.content = content;
        self
    }

    pub fn log_name(&mut self, name: String) -> &mut Self {
        self.record.log_name = name;
        self
    }

    #[cfg(feature = "log")]
    fn line(&mut self, line: u32) -> &mut Self {
        self.record.line = Some(line);
        self
    }

    #[cfg(feature = "log")]
    fn file(&mut self, file: &str) -> &mut Self {
        self.record.file = Some(file.to_string());
        self
    }

    pub fn build(&mut self) -> EZRecord {
        self.record.id = self.record.id();
        self.record.clone()
    }
}

impl Default for EZRecordBuilder {
    fn default() -> Self {
        EZRecordBuilder {
            record: EZRecord {
                id: 0,
                log_name: DEFAULT_LOG_NAME.to_string(),
                level: Level::Info,
                target: "".to_string(),
                time: OffsetDateTime::now_utc(),
                thread_id: thread_id::get(),
                thread_name: thread::current().name().unwrap_or("unknown").to_string(),
                content: "".to_string(),
                file: None,
                line: None,
            },
        }
    }
}

#[cfg(feature = "log")]
impl From<&Record<'_>> for EZRecord {
    fn from(record: &Record) -> Self {
        EZRecord::from(record)
    }
}
