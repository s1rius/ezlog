use std::path::Path;
use std::{fs::OpenOptions};
use memmap2::MmapOptions;
use log::{info, warn, LevelFilter};
use log::{Record, Level, Metadata};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

static LOGGER: SimpleLogger = SimpleLogger;

pub fn main() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace)).unwrap();
    
    let download_dir = dirs::download_dir().unwrap();

    let path = download_dir.join("1/1.mmp");
    let file = ezlog::init_mmap_temp_file(&path).unwrap();
    
    let mut mmap = unsafe { MmapOptions::new().map_mut(&file).expect("failed to map the file") };
    // println!("Hello, wtf!");
    // (&mut mmap[..]).write(b"Hello, world!").unwrap();
    // mmap.flush().unwrap();


    println!("write byte");
    let ustr = "asdf";
    let len = ustr.as_bytes().len();
    for x in 100usize..120usize {
        let start = x * len;
        let data = (mmap[start..start + len].as_mut_ptr()).cast::<u8>();
        let src = ustr.as_ptr();
        unsafe {
            std::ptr::copy_nonoverlapping(src, data, len);
        }
        // let mut temp = unsafe {MmapOptions::new().offset((x * 8) as u64).len(8).map_mut(&file).unwrap()};
        // (&mut temp[..]).write(b"sb").unwrap();
        // temp.flush().unwrap();
        println!("write byte end");
    }

    (&mut mmap[0 .. 8]).write_i64::<BigEndian>(i64::MAX).unwrap();
    (&mut mmap[8 .. 16]).write_u64::<BigEndian>(u64::MAX).unwrap();

    assert_eq!(i64::MAX, (&mmap[0 .. 8]).read_i64::<BigEndian>().unwrap());
    assert_eq!(u64::MAX, (&mmap[8 .. 16]).read_u64::<BigEndian>().unwrap());


    mmap.flush_async().unwrap();
}

struct SimpleLogger;

impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}