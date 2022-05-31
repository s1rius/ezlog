
#[derive(Debug, Clone)]
pub struct EZLogConfig {
    /// log等级
    level: Level,
    /// 版本号
    version: Version,
    /// 文件夹目录
    dir_path: String,
    /// 文件的前缀名
    name: String,
    /// 文件的后缀名
    file_suffix: String,
    /// 文件缓存的时间
    duration: Duration,
    /// 日志文件的最大大小
    max_size: u64,
    // 压缩方式
    compress: CompressKind,
    /// 压缩等级
    compress_level: CompressLevel,
    /// 加密方式
    cipher: CipherKind,
    /// 加密的密钥
    cipher_key: Option<Vec<u8>>,
    /// 加密的nonce
    cipher_nonce: Option<Vec<u8>>,
}

impl EZLogConfig {
    pub fn new(
        level: Level,
        version: Version,
        dir_path: String,
        name: String,
        file_suffix: String,
        duration: Duration,
        max_size: u64,
        compress: CompressKind,
        compress_level: CompressLevel,
        cipher: CipherKind,
        cipher_key: Option<Vec<u8>>,
        cipher_nonce: Option<Vec<u8>>,
    ) -> Self {
        EZLogConfig {
            level,
            version,
            dir_path,
            name,
            file_suffix,
            duration,
            max_size,
            compress,
            compress_level,
            cipher,
            cipher_key,
            cipher_nonce,
        }
    }

    pub fn now_file_name(&self, now: OffsetDateTime) -> String {
        let format = format_description::parse("[year]_[month]_[day]")
            .expect("Unable to create a formatter; this is a bug in tracing-appender");
        let date = now
            .format(&format)
            .expect("Unable to format OffsetDateTime; this is a bug in tracing-appender");
        let str = format!("{}_{}.{}", self.name, date, self.file_suffix);
        str
    }

    pub fn create_mmap_file(&self, time: OffsetDateTime) -> io::Result<(File, PathBuf)> {
        let file_name = self.now_file_name(time);
        let max_size = self.max_size;
        let path = Path::new(&self.dir_path).join(file_name);

        if let Some(p) = path.parent() {
            if !p.exists() {
                fs::create_dir_all(p)?;
            }
        }

        // create file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        // check file lenth ok or set len
        let mut len = file.metadata()?.len();
        if len == 0 {
            println!("set file len");
            len = max_size;
            if len == 0 {
                len = DEFAULT_MAX_LOG_SIZE;
            }
            file.set_len(len)?;
        }

        Ok((file, path))
    }

    fn log_id(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for EZLogConfig {
    fn default() -> Self {
        EZLogConfigBuilder::new()
            .dir_path(
                env::current_dir()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            )
            .name(DEFAULT_LOG_NAME.to_string())
            .file_suffix(String::from("mmap"))
            .max_size(1024)
            .build()
    }
}

impl Hash for EZLogConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.version.hash(state);
        self.dir_path.hash(state);
        self.name.hash(state);
        self.compress.hash(state);
        self.cipher.hash(state);
        self.cipher_key.hash(state);
        self.cipher_nonce.hash(state);
    }
}

pub struct EZLogConfigBuilder {
    config: EZLogConfig,
}

impl EZLogConfigBuilder {
    pub fn new() -> Self {
        EZLogConfigBuilder {
            config: EZLogConfig {
                level: Level::Trace,
                version: Version::V1,
                dir_path: "".to_string(),
                name: DEFAULT_LOG_NAME.to_string(),
                file_suffix: DEFAULT_LOG_FILE_SUFFIX.to_string(),
                duration: Duration::days(7),
                max_size: DEFAULT_MAX_LOG_SIZE,
                compress: CompressKind::NONE,
                compress_level: CompressLevel::Default,
                cipher: CipherKind::NONE,
                cipher_key: None,
                cipher_nonce: None,
            },
        }
    }

    pub fn level(mut self, level: Level) -> Self {
        self.config.level = level;
        self
    }

    pub fn dir_path(mut self, dir_path: String) -> Self {
        self.config.dir_path = dir_path;
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.config.name = name;
        self
    }

    pub fn file_suffix(mut self, file_suffix: String) -> Self {
        self.config.file_suffix = file_suffix;
        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.config.duration = duration;
        self
    }

    pub fn max_size(mut self, max_size: u64) -> Self {
        self.config.max_size = max_size;
        self
    }

    pub fn compress(mut self, compress: CompressKind) -> Self {
        self.config.compress = compress;
        self
    }

    pub fn cipher(mut self, cipher: CipherKind) -> Self {
        self.config.cipher = cipher;
        self
    }

    pub fn cipher_key(mut self, cipher_key: Vec<u8>) -> Self {
        self.config.cipher_key = Some(cipher_key);
        self
    }

    pub fn cipher_nonce(mut self, cipher_nonce: Vec<u8>) -> Self {
        self.config.cipher_nonce = Some(cipher_nonce);
        self
    }

    pub fn build(self) -> EZLogConfig {
        self.config
    }
}
