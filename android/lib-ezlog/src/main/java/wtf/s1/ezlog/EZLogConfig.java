package wtf.s1.ezlog;

import org.jetbrains.annotations.NotNull;

import java.util.Arrays;

public class EZLogConfig {
    @NotNull String logName;
    int maxLevel;
    @NotNull String dirPath;
    int keepDays;
    int compress;
    int compressLevel;
    int cipher;
    byte[] cipherKey;
    byte[] cipherNonce;
    boolean enableTrace;

    public EZLogConfig(@NotNull String logName, @NotNull String dirPath) {
        this.logName = logName;
        this.dirPath = dirPath;
        this.maxLevel = EZLog.ERROR;
        this.keepDays = 7;
        this.cipherKey = new byte[]{};
        this.cipherNonce = new byte[]{};
    }

    public static class Builder {
        @NotNull String logName;
        int maxLevel;
        @NotNull String dirPath;
        int keepDays;
        int compress;
        int compressLevel;
        int cipher;
        byte[] cipherKey;
        byte[] cipherNonce;
        boolean enableTrace;

        public Builder(@NotNull String logName, @NotNull String dirPath) {
            this.logName = logName;
            this.dirPath = dirPath;
        }

        public Builder level(int level) {
            this.maxLevel = level;
            return this;
        }

        public Builder keepDays(int days) {
            this.keepDays = days;
            return this;
        }

        public Builder compress(int compress) {
            this.compress = compress;
            return this;
        }

        public Builder compressLevel(int compressLevel) {
            this.compressLevel = compressLevel;
            return this;
        }

        public Builder cipher(int cipher) {
            this.cipher = cipher;
            return this;
        }

        public Builder cipherKey(byte[] cipherKey) {
            this.cipherKey = cipherKey;
            return this;
        }

        public Builder cipherNonce(byte[] cipherNonce) {
            this.cipherNonce = cipherNonce;
            return this;
        }

        public Builder enableTrace(boolean isEnable) {
            this.enableTrace = isEnable;
            return this;
        }

        public EZLogConfig build() {
            EZLogConfig config = new EZLogConfig(this.logName, this.dirPath);
            config.maxLevel = this.maxLevel;
            config.keepDays = this.keepDays;
            config.compress = this.compress;
            config.compressLevel = this.compressLevel;
            config.cipher = this.cipher;
            config.cipherKey = this.cipherKey;
            config.cipherNonce = this.cipherNonce;
            config.enableTrace = this.enableTrace;
            return config;
        }
    }

    @Override
    public boolean equals(Object o) {
        if (this == o) return true;
        if (!(o instanceof EZLogConfig)) return false;

        EZLogConfig that = (EZLogConfig) o;

        if (maxLevel != that.maxLevel) return false;
        if (keepDays != that.keepDays) return false;
        if (compress != that.compress) return false;
        if (compressLevel != that.compressLevel) return false;
        if (cipher != that.cipher) return false;
        if (!logName.equals(that.logName)) return false;
        if (!dirPath.equals(that.dirPath)) return false;
        if (!Arrays.equals(cipherKey, that.cipherKey)) return false;
        return Arrays.equals(cipherNonce, that.cipherNonce);
    }

    @Override
    public int hashCode() {
        int result = logName.hashCode();
        result = 31 * result + maxLevel;
        result = 31 * result + dirPath.hashCode();
        result = 31 * result + keepDays;
        result = 31 * result + compress;
        result = 31 * result + compressLevel;
        result = 31 * result + cipher;
        result = 31 * result + Arrays.hashCode(cipherKey);
        result = 31 * result + Arrays.hashCode(cipherNonce);
        return result;
    }
}
