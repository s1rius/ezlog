/// # [eventlisteners are good](https://publicobject.com/2022/05/01/eventlisteners-are-good/)
///
#[macro_export]
macro_rules! event {
    (init) => {
        println!("init")
    };
    (map_create) => {
        println!("log map create")
    };
    (log_create $log_name:expr) => {
        println!("log create name={}", $log_name)
    };
    (log_create_fail $log_name:expr, $err_msg:expr) => {
        println!("log create fail name={}, err={}", $log_name, $err_msg)
    };
    (compress_start $record_id:tt) => {
        println!("record compress start {}", $record_id);
    };
    (compress_end $record_id:tt) => {
        println!("record compress end {}", $record_id);
    };
    (compress_fail $record_id:expr, $e:expr) => {
        println!("record compress fail {}, reason {}", $record_id, $e)
    };
    (encrypt_start $record_id:tt) => {
        println!("encrypt start {}", $record_id)
    };
    (encrypt_end $record_id:tt $start_time_ns:tt) => {
        println!("encrypt end {}", $record_id)
    };
    (encrypt_fail $record_id:expr, $e:expr) => {
        println!("fail reason: {}", $e)
    };
    (record_complete $record_id:expr) => {
        println!("record complete {}", $record_id)
    };
    (record_filter_out $record_id:expr, $record_level:expr, $target_level:expr) => {
        println!(
            "record filter out {} level = {}, target level = {}",
            $record_id, $record_level, $target_level
        )
    };
    (io_error $record_id:expr, $e:expr) => {
        println!("log {} append io err {}", $record_id, $e)
    };
    (unexpect_fail $record_id:expr, $e:expr) => {
        println!("log {} append err {}", $record_id, $e)
    };
    (logger_not_match $log_name:expr) => {
        println!("log get from map err {}", $log_name)
    };
    (channel_send_log_create $log_name:expr) => {
        println!("send create msg {}", $log_name)
    };
    (logger_force_flush $log_name:expr) => {
        println!("log flush {}", $log_name)
    };
    (channel_send_record $record_id:expr) => {
        println!("send record msg {}", $record_id)
    };
    (channel_send_flush $log_name:expr) => {
        println!("send flush msg {}", $log_name)
    };
    (channel_send_flush_all) => {
        println!("send flush all msg")
    };
    (channel_send_err $e:tt) => {
        println!("channel err {}", $e)
    };
    (channel_recv_err $e:tt) => {
        println!("channel err {}", $e)
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_event() {
        event!(log_create_fail "default", "unknown err");
    }
}
