package wtf.s1.ezlog

class EZLogConfig(var logName: String, var dirPath: String) {
    var maxLevel: Int = EZLog.VERBOSE
    var keepDays: Int = 7
    var rotateHours: Int = 24
    var compress = 0
    var compressLevel = 0
    var cipher = 0
    var cipherKey: ByteArray = byteArrayOf()
    var cipherNonce: ByteArray = byteArrayOf()
    var enableTrace = false
    var extra: String? = null

    /**
     * EZLog Builder
     */
    class Builder(logName: String, dirPath: String) {
        val config = EZLogConfig(logName, dirPath)

        /**
         * set up the max level, any log's level < maxLevel will be filtered out.
         * @param level the max level
         */
        fun maxLevel(level: Int): Builder {
            this.config.maxLevel = level
            return this
        }

        /**
         * set log file expired days. The log files which out of date will be deleted
         * when trim() function call
         *
         * @param days keep log file days
         */
        fun keepDays(days: Int): Builder {
            this.config.keepDays = days
            return this
        }

        /**
         * set log file rotate hours. The log files will be rotated after create time + rotateHours
         *
         * @param hours after log file rotate
         */
        fun rotateHours(hours: Int): Builder {
            this.config.rotateHours = hours;
            return this;
        }

        /**
         * set compress kind
         * @param compress compress kind
         */
        fun compress(compress: Int): Builder {
            this.config.compress = compress
            return this
        }

        fun compress(compress: EZLog.Compress): Builder {
            this.config.compress = when (compress) {
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
            this.config.compressLevel = compressLevel
            return this
        }

        fun compressLevel(compressLevel: EZLog.CompressLevel): Builder {
            this.config.compressLevel = when (compressLevel) {
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
        @Deprecated(
            "this function is deprecated",
            replaceWith = ReplaceWith("cipher(cipher: EZLog.Cipher)"),
            level = DeprecationLevel.ERROR
        )
        fun cipher(cipher: Int): Builder {
            val c = when (cipher) {
                EZLog.Aes128Gcm -> EZLog.Cipher.AES128GCMSIV
                EZLog.Aes256Gcm -> EZLog.Cipher.AES256GCMSIV
                0 -> EZLog.Cipher.NONE
                else -> throw IllegalArgumentException("use cipher(cipher: EZLog.Cipher) to set cipher")
            }
            cipher(c)
            this.config.cipher = cipher
            return this
        }

        @SuppressWarnings
        fun cipher(cipher: EZLog.Cipher): Builder {
            this.config.cipher = when (cipher) {
                EZLog.Cipher.NONE -> 0
                EZLog.Cipher.AES256GCM, EZLog.Cipher.AES256GCMSIV -> EZLog.Aes256GcmSiv
                EZLog.Cipher.AES128GCM, EZLog.Cipher.AES128GCMSIV -> EZLog.Aes128GcmSiv
            }
            return this
        }

        /**
         * set cipher key
         * @param cipherKey cipher key
         */
        fun cipherKey(cipherKey: ByteArray): Builder {
            this.config.cipherKey = cipherKey
            return this
        }

        /**
         * set cipher nonce
         * @param cipherNonce nonce
         */
        fun cipherNonce(cipherNonce: ByteArray): Builder {
            this.config.cipherNonce = cipherNonce
            return this
        }

        /**
         * when enable trace, all event will print out to Logcat
         * @param isEnable is trace enable
         */
        fun enableTrace(isEnable: Boolean): Builder {
            this.config.enableTrace = isEnable
            return this
        }

        fun extra(extra: String): Builder {
            this.config.extra = extra
            return this
        }

        fun build(): EZLogConfig {
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
        if (rotateHours != other.rotateHours) return false
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
        result = 31 * result + rotateHours
        result = 31 * result + compress
        result = 31 * result + compressLevel
        result = 31 * result + cipher
        result = 31 * result + cipherKey.contentHashCode()
        result = 31 * result + cipherNonce.contentHashCode()
        result = 31 * result + enableTrace.hashCode()
        return result
    }

}