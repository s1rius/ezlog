package wtf.s1.ezlog

class EZLogConfig(var logName: String, var dirPath: String) {
    var maxLevel: Int = EZLog.VERBOSE
    var keepDays: Int = 7
    var compress = 0
    var compressLevel = 0
    var cipher = 0
    var cipherKey: ByteArray
    var cipherNonce: ByteArray
    var enableTrace = false

    init {
        cipherKey = byteArrayOf()
        cipherNonce = byteArrayOf()
    }

    /**
     * EZLog Builder
     */
    class Builder(var logName: String, var dirPath: String) {
        var maxLevel = EZLog.VERBOSE
        var keepDays = 7
        var compress = 0
        var compressLevel = 0
        var cipher = 0
        private var cipherKey: ByteArray = ByteArray(0)
        private var cipherNonce: ByteArray = ByteArray(0)
        var enableTrace = false

        /**
         * set up the max level, any log's level < maxLevel will be filtered out.
         * @param level the max level
         */
        fun maxLevel(level: Int): Builder {
            maxLevel = level
            return this
        }

        /**
         * set log file expired days. The log files which out of date will be deleted
         * when trim() function call
         *
         * @param days keep log file days
         */
        fun keepDays(days: Int): Builder {
            keepDays = days
            return this
        }

        /**
         * set compress kind
         * @param compress compress kind
         */
        fun compress(compress: Int): Builder {
            this.compress = compress
            return this
        }

        fun compress(compress: EZLog.Compress): Builder {
            this.compress = when (compress) {
                EZLog.Compress.NONE -> 0
                EZLog.Compress.ZLIB -> EZLog.CompressZlib
            }
            return this
        }

        /**
         * set compress level. eg: default, fast, best
         * @param compressLevel compress level
         */
        fun compressLevel(compressLevel: Int): Builder {
            this.compressLevel = compressLevel
            return this
        }

        fun compressLevel(compressLevel: EZLog.CompressLevel): Builder {
            this.compressLevel = when (compressLevel) {
                EZLog.CompressLevel.DEFAULT -> EZLog.CompressDefault
                EZLog.CompressLevel.BEST -> EZLog.CompressBest
                EZLog.CompressLevel.FAST -> EZLog.CompressFast
            }
            return this
        }

        /**
         * set cipher kind
         * @param cipher cipher kind
         */
        fun cipher(cipher: Int): Builder {
            this.cipher = cipher
            return this
        }

        fun cipher(cipher: EZLog.Cipher): Builder {
            this.cipher = when(cipher) {
                EZLog.Cipher.NONE -> 0
                EZLog.Cipher.AES256GCM -> EZLog.Aes256Gcm
                EZLog.Cipher.AES128GCM -> EZLog.Aes128Gcm
            }
            return this
        }

        /**
         * set cipher key
         * @param cipherKey cipher key
         */
        fun cipherKey(cipherKey: ByteArray): Builder {
            this.cipherKey = cipherKey
            return this
        }

        /**
         * set cipher nonce
         * @param cipherNonce nonce
         */
        fun cipherNonce(cipherNonce: ByteArray): Builder {
            this.cipherNonce = cipherNonce
            return this
        }

        /**
         * when enable trace, all event will print out to Logcat
         * @param isEnable is trace enable
         */
        fun enableTrace(isEnable: Boolean): Builder {
            enableTrace = isEnable
            return this
        }

        fun build(): EZLogConfig {
            val config = EZLogConfig(logName, dirPath)
            config.maxLevel = maxLevel
            config.keepDays = keepDays
            config.compress = compress
            config.compressLevel = compressLevel
            config.cipher = cipher
            config.cipherKey = cipherKey
            config.cipherNonce = cipherNonce
            config.enableTrace = enableTrace
            return config
        }
    }

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as EZLogConfig

        if (logName != other.logName) return false
        if (dirPath != other.dirPath) return false
        if (maxLevel != other.maxLevel) return false
        if (keepDays != other.keepDays) return false
        if (compress != other.compress) return false
        if (compressLevel != other.compressLevel) return false
        if (cipher != other.cipher) return false
        if (!cipherKey.contentEquals(other.cipherKey)) return false
        if (!cipherNonce.contentEquals(other.cipherNonce)) return false
        if (enableTrace != other.enableTrace) return false

        return true
    }

    override fun hashCode(): Int {
        var result = logName.hashCode()
        result = 31 * result + dirPath.hashCode()
        result = 31 * result + maxLevel
        result = 31 * result + keepDays
        result = 31 * result + compress
        result = 31 * result + compressLevel
        result = 31 * result + cipher
        result = 31 * result + cipherKey.contentHashCode()
        result = 31 * result + cipherNonce.contentHashCode()
        result = 31 * result + enableTrace.hashCode()
        return result
    }
}