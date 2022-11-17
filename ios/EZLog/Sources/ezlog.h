#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * map to c Callback stuct
 */
typedef struct Callback {
    const void* _Nonnull successPoint;
    const void(* _Nonnull onLogsFetchSuccess)(void* _Nonnull,
                                              const char* _Nonnull,
                                              const char* _Nonnull,
                                              const int8_t* _Nonnull const* _Nonnull,
                                              int32_t);
    const void* _Nonnull failPoint;
    const void (* _Nonnull onLogsFetchFail)(void* _Nonnull,
                                            const char* _Nonnull,
                                            const char* _Nonnull,
                                            const char* _Nonnull);
} Callback;

/**
 * Init ezlog, must call before any other function
 */
void ezlog_init(bool enable_trace);

/**
 * Flush target log which name is `c_log_name`
 */
void ezlog_flush(const char * _Nonnull c_log_name);

/**
 * Flush all logger
 */
void ezlog_flush_all(void);

/**
 * Create a new log wtih config options
 */
void ezlog_create_log(const char * _Nonnull c_log_name,
                      unsigned char c_level,
                      const char * _Nonnull c_dir_path,
                      unsigned int c_keep_days,
                      unsigned char c_compress,
                      unsigned char c_compress_level,
                      unsigned char c_cipher,
                      const unsigned char * _Nonnull c_cipher_key,
                      uintptr_t c_key_len,
                      const unsigned char * _Nonnull c_cipher_nonce,
                      uintptr_t c_nonce_len);

/**
 * Write log to file
 */
void ezlog_log(const char * _Nonnull c_log_name,
               unsigned char c_level,
               const char * _Nonnull c_target,
               const char * _Nonnull c_content);

/**
 * Trim out of date log files
 */
void ezlog_trim(void);

/**
 * Register callback function for get logger's file path asynchronously
 */
void ezlog_register_callback(struct Callback callback);

/**
 * Request logger's files path array by specified date
 * before call this function, you should register a callback
 * call
 * ```
 * ezlog_register_callback(callback);
 * ```
 */
void ezlog_request_log_files_for_date(const char * _Nonnull c_log_name, const char * _Nonnull c_date);
