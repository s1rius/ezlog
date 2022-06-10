#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * init
 */
void ezlog_init(void);

/**
 * # Safety
 *
 */
void ezlog_flush(const char *c_log_name);

/**
 * # Safety
 *
 */
void ezlog_flush_all(void);

/**
 * # Safety
 *
 */
void ezlog_create_log(const char *c_log_name,
                      unsigned char c_level,
                      const char *c_dir_path,
                      unsigned int c_keep_days,
                      unsigned char c_compress,
                      unsigned char c_compress_level,
                      unsigned char c_cipher,
                      const unsigned char *c_cipher_key,
                      uintptr_t c_key_len,
                      const unsigned char *c_cipher_nonce,
                      uintptr_t c_nonce_len);

/**
 * # Safety
 *
 */
void ezlog_log(const char *c_log_name,
               unsigned char c_level,
               const char *c_target,
               const char *c_content);
