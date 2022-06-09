#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * init
 */
void c_init(void);

/**
 * # Safety
 *
 */
void c_flush(const char *c_log_name);

/**
 * # Safety
 *
 */
void c_flush_all(void);

/**
 * # Safety
 *
 */
void c_create_log(const char *c_log_name,
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
void c_log(const char *c_log_name,
           unsigned char c_level,
           const char *c_target,
           const char *c_content);
