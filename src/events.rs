/// # [eventlisteners are good](https://publicobject.com/2022/05/01/eventlisteners-are-good/)
/// 
/// 
#[macro_export]
macro_rules! ezlog_init {
    () => ($crate::events::init());
    ($($arg:tt)*) => ({
        $crate::events::init();
        //$crate::io::_print($crate::format_args_nl!($($arg)*));
    });
}

pub fn init() {

}

macro_rules! logger_create {
    () => {
        
    };
    ($($arg:tt)*) => ({
        
    });
}

#[macro_export]
macro_rules! event {
    () => ({
        $(
            println!("sdfsfa");
        )*
    });
    (log_create $($args: expr),*) => {
        $(
            print!(", {}",$args);
        )*
        println!("131231"); // to get a new line at the end
    };
    ($($args: expr),*) => {
        print!("TRACE: file: {}, line: {}", file!(), line!());
        $(
            print!(", {}: {}", stringify!($args), $args);
        )*
        println!(""); // to get a new line at the end
    }
}

pub struct EventListener();

impl EventListener {
    
}