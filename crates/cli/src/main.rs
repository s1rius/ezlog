use std::{
    fs::OpenOptions,
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
};

use clap::Parser;
pub use ezlog::*;
use serde::{Deserialize, Serialize};

macro_rules! unwrap_or_return {
    ( $e:expr, $e1:expr ) => {
        match $e {
            Ok(x) => x,
            Err(e) => {
                $e1;
                println!("{}", e);
                return;
            }
        }
    };
}

macro_rules! some_or_return {
    ( $e:expr, $e1:expr ) => {
        match $e {
            Some(x) => x,
            None => {
                $e1;
                return;
            }
        }
    };
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {

    /// Origin ezlog file path
    #[clap(short, long, value_parser, value_name = "FILE")]
    input: Option<PathBuf>,

    /// Decode log file path
    #[clap(short, long, value_parser, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Sets a JSON config file contains the configuration of the logger.
    /// 
    /// {
	///     "crypto_key": "an example very very secret key.",
	///     "crypto_nonce": "unique nonce"
    /// }
    #[clap(short, long, value_parser, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn debugging information on
    #[clap(short, long, action)]
    debug: bool,
}

#[derive(Serialize, Deserialize)]
struct Config {
    crypto_key: String,
    crypto_nonce: String,
}

pub fn main() {
    let cli = Cli::parse();

    if cli.debug {
        println!("debug enable");
    }

    let input = some_or_return!(cli.input.as_deref(), println!("-i input file must set"));

    let input_file = unwrap_or_return!(
        OpenOptions::new().read(true).open(input),
        println!("-i input file open error")
    );

    let output = some_or_return!(cli.output.as_deref(), println!("-o output file must set"));

    let output_file = unwrap_or_return!(
        OpenOptions::new().write(true).create(true).open(output),
        println!("-o output file create error")
    );

    let mut buf_reader = BufReader::new(input_file);
    let header = unwrap_or_return!(
        ezlog::Header::decode(&mut buf_reader),
        println!("log file can not be recognized")
    );

    if cli.debug {
        println!("header parse {:?}", &header);
    }

    let mut crypto_key: Vec<u8> = Vec::new();
    let mut crypto_nonce: Vec<u8> = Vec::new();

    if let Some(config_path) = cli.config.as_deref() {
        if let Ok(config_file) = OpenOptions::new().read(true).open(config_path) {
            let mut json = String::new();
            if BufReader::new(config_file)
                .read_to_string(&mut json)
                .is_ok()
            {
                let config: Config = unwrap_or_return!(
                    serde_json::from_str(&json),
                    println!("parse config json error")
                );
                crypto_key = config.crypto_key.as_bytes().to_vec();
                crypto_nonce = config.crypto_nonce.as_bytes().to_vec();

                if cli.debug {
                    println!(
                        "config read \n    key={} \n    nonce={}",
                        &config.crypto_key, &config.crypto_nonce
                    );
                }
            }
        } else {
            println!("config file read error")
        }
    }

    let config = EZLogConfigBuilder::new()
        .from_header(&header)
        .cipher_key(crypto_key)
        .cipher_nonce(crypto_nonce)
        .build();

    let compression = EZLogger::create_compress(&config);
    let decryptor = unwrap_or_return!(
        EZLogger::create_cryptor(&config),
        println!("create cryptor error")
    );

    let mut plain_text_write = BufWriter::new(output_file);
    let mut end = false;
    loop {
        if end {
            break;
        }

        match EZLogger::decode_from_read(&mut buf_reader, &compression, &decryptor) {
            Ok(buf) => {
                plain_text_write.write_all(&buf).unwrap();
            }
            Err(_e) => {
                end = true;
            }
        }
    }
    plain_text_write.flush().unwrap();
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_decode() {}
}
