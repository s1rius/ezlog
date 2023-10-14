use std::thread::sleep;
use std::time::Duration;

use ezlog::EZLogConfigBuilder;
use log::{
    error,
    info,
    warn,
};
use log::{
    LevelFilter,
    Log,
};

extern crate log;

pub fn main() {
    let logger = env_logger::builder()
        .filter_level(LevelFilter::Trace)
        .build();
    let ezlog = ezlog::InitBuilder::new().debug(true).init();

    struct MyLog {
        list: Vec<Box<dyn log::Log>>,
    }

    impl Log for MyLog {
        fn enabled(&self, meta: &log::Metadata) -> bool {
            self.list.iter().for_each(|l| {
                l.enabled(meta);
            });
            true
        }

        fn log(&self, record: &log::Record) {
            self.list.iter().for_each(|l| l.log(record));
        }

        fn flush(&self) {
            self.list.iter().for_each(|l| l.flush());
        }
    }

    let mylog = MyLog {
        list: vec![Box::new(logger), Box::new(ezlog)],
    };

    ezlog::create_log(
        EZLogConfigBuilder::new()
            .dir_path(dirs::download_dir().unwrap().to_string_lossy().to_string())
            .build(),
    );

    log::set_boxed_logger(Box::new(mylog)).unwrap();
    log::set_max_level(LevelFilter::Trace);

    info!("starting up");
    warn!("warning");
    error!("an error!");

    sleep(Duration::from_secs(1));
}
