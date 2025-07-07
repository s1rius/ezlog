use std::{
    collections::hash_map::DefaultHasher,
    fmt::Display,
    hash::{
        Hash,
        Hasher,
    },
    thread,
};

#[cfg(feature = "log")]
use log::Record;
use time::{
    format_description::well_known::Rfc3339,
    OffsetDateTime,
};

use crate::{
    EZLogConfig,
    Level,
    DEFAULT_LOG_NAME,
};

/// Single Log record
#[derive(Debug, Clone)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
pub struct EZRecord {
    id: u64,
    #[cfg_attr(feature = "json", serde(rename = "n"))]
    log_name: String,
    #[cfg_attr(feature = "json", serde(rename = "l"))]
    level: Level,
    #[cfg_attr(feature = "json", serde(rename = "g"))]
    target: String,
    #[cfg_attr(feature = "json", serde(rename = "t"))]
    #[cfg_attr(feature = "json", serde(serialize_with = "crate::serialize_time"))]
    #[cfg_attr(feature = "json", serde(deserialize_with = "crate::deserialize_time"))]
    time: OffsetDateTime,
    #[cfg_attr(feature = "json", serde(rename = "d"))]
    thread_id: usize,
    #[cfg_attr(feature = "json", serde(rename = "m"))]
    thread_name: String,
    #[cfg_attr(feature = "json", serde(rename = "c"))]
    content: String,
    #[cfg_attr(feature = "json", serde(rename = "f"))]
    file: Option<String>,
    #[cfg_attr(feature = "json", serde(rename = "y"))]
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
            .log_name(DEFAULT_LOG_NAME)
            .level(r.metadata().level().into())
            .target(r.target())
            .time(OffsetDateTime::now_utc())
            .thread_id(t_id)
            .thread_name(t_name)
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
        let content_bytes = self.content.as_bytes();
        let max_size = config.max_size() as usize / 2;
        let mut start = 0;

        while start < content_bytes.len() {
            // Find the end index, making sure not to split in the middle of a UTF-8 character
            let mut end = usize::min(start + max_size, content_bytes.len());
            while end < content_bytes.len() && !self.content.is_char_boundary(end) {
                end -= 1;
            }
            let chunk = &self.content[start..end];
            let ez = self.to_trunk_builder().content(chunk).build();
            trunks.push(ez);
            start = end;
        }

        trunks
    }
}

impl PartialEq for EZRecord {
    fn eq(&self, other: &Self) -> bool {
        self.log_name == other.log_name
            && self.level == other.level
            && self.target == other.target
            && self.time == other.time
            && self.thread_id == other.thread_id
            && self.thread_name == other.thread_name
            && self.content == other.content
            && self.file == other.file
            && self.line == other.line
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

    pub fn target(&mut self, target: impl AsRef<str>) -> &mut Self {
        self.record.target = target.as_ref().to_owned();
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

    pub fn thread_name(&mut self, thread_name: impl AsRef<str>) -> &mut Self {
        self.record.thread_name = thread_name.as_ref().to_owned();
        self
    }

    pub fn content(&mut self, content: impl AsRef<str>) -> &mut Self {
        self.record.content = content.as_ref().to_owned();
        self
    }

    pub fn log_name(&mut self, name: impl AsRef<str>) -> &mut Self {
        self.record.log_name = name.as_ref().to_owned();
        self
    }

    #[cfg(feature = "log")]
    pub fn line(&mut self, line: u32) -> &mut Self {
        self.record.line = Some(line);
        self
    }

    #[cfg(feature = "log")]
    pub fn file(&mut self, file: impl AsRef<str>) -> &mut Self {
        self.record.file = Some(file.as_ref().to_owned());
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
                target: "default".to_string(),
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

impl Display for EZRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}] {} {} {} {} {}",
            self.time.format(&Rfc3339).unwrap_or("".to_string()),
            self.level,
            self.target,
            self.thread_id,
            self.thread_name,
            self.content
        )
    }
}
