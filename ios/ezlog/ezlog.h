#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct Callback {
    const void *successPoint;
    const void (*onLogsFetchSuccess)(void* _Nonnull,
                                        const char* _Nonnull,
                                        const char* _Nonnull,
                                        const int8_t* _Nonnull const* _Nonnull,
                                        int32_t);
    const void *failPoint;
    const void (*onLogsFetchFail)(void* _Nonnull,
                                     const char* _Nonnull,
                                     const char* _Nonnull,
                                     const char* _Nonnull);
} Callback;

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

void ezlog_register_callback(struct Callback callback);

void ezlog_request_log_files_for_date(const char *c_log_name, const char *c_date);
