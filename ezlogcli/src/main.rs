use std::{
    fs::OpenOptions,
    io::{
        BufReader,
        BufWriter,
        Cursor,
        Read,
    },
    path::PathBuf,
};

use anyhow::{
    anyhow,
    Context,
};
use clap::Parser;
pub use ezlog::*;
use serde::{
    Deserialize,
    Serialize,
};

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
    ///     "key": "an example very very secret key.",
    ///     "nonce": "unique nonce"
    /// }
    #[clap(short, long, value_parser, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Decrypt key
    #[clap(short, long, value_parser)]
    key: Option<String>,

    /// Decrypt nonce
    #[clap(short, long, value_parser)]
    nonce: Option<String>,

    /// Turn debugging information on
    #[clap(short, long, action)]
    debug: bool,
}

#[derive(Serialize, Deserialize)]
struct Config {
    key: String,
    nonce: String,
}

pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.debug {
        println!("debug enable");
        println!();
        println!("{:?}", cli);
    }

    let input = cli
        .input
        .as_deref()
        .with_context(|| "-i input file must be set".to_string())?;

    let input_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(input)
        .with_context(|| "input file must valid".to_string())?;

    let output = cli
        .output
        .as_deref()
        .with_context(|| "-o output file must be set".to_string())?;

    let output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(output)
        .with_context(|| "output file create error".to_string())?;

    let mut buf = Vec::<u8>::new();
    let mut reader = BufReader::new(input_file);
    reader.read_to_end(&mut buf).unwrap();
    let mut cursor = Cursor::new(buf);
    let header =
        ezlog::Header::decode(&mut cursor).with_context(|| "header decode error".to_string())?;

    if cli.debug {
        println!();
        println!("header parse {:?}", &header);
    }

    let mut key: Vec<u8> = Vec::new();
    let mut nonce: Vec<u8> = Vec::new();

    if let Some(config_path) = cli.config.as_deref() {
        if let Ok(config_file) = OpenOptions::new().read(true).open(config_path) {
            let mut json = String::new();
            if BufReader::new(config_file)
                .read_to_string(&mut json)
                .is_ok()
            {
                let config: Config = serde_json::from_str(&json)
                    .with_context(|| "config file is not valid".to_string())?;
                key = config.key.as_bytes().to_vec();
                nonce = config.nonce.as_bytes().to_vec();

                if cli.debug {
                    println!(
                        "config read \n    key={} \n    nonce={}",
                        &config.key, &config.nonce
                    );
                }
            }
        } else {
            println!("config file read error")
        }
    }

    if key.is_empty() {
        key = cli.key.map_or(vec![], |k| k.as_bytes().to_vec())
    }

    if nonce.is_empty() {
        nonce = cli.nonce.map_or(vec![], |n| n.as_bytes().to_vec())
    }

    let config = EZLogConfigBuilder::new()
        .from_header(&header)
        .cipher_key(key)
        .cipher_nonce(nonce)
        .build();

    let compression = ezlog::create_compress(&config);
    let decryptor =
        ezlog::create_cryptor(&config).with_context(|| "create cryptor error".to_string())?;

    let mut plain_text_write = BufWriter::new(output_file);

    ezlog::decode::decode_with_writer(
        &mut cursor,
        &mut plain_text_write,
        compression,
        decryptor,
        &header,
    )
    .map_err(|e| anyhow!(format!("{}", e)))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use assert_cmd::prelude::{
        OutputAssertExt,
        OutputOkExt,
    };

    #[test]
    fn test_help() {
        escargot::CargoBuild::new()
            .bin("ezlogcli")
            .current_release()
            .current_target()
            .run()
            .unwrap()
            .command()
            .arg("--help")
            .unwrap()
            .assert()
            .success();
    }

    #[test]
    fn test_decode() {
        let bin_under_test = escargot::CargoBuild::new()
            .bin("ezlogcli")
            .current_release()
            .current_target()
            .run()
            .unwrap();

        let mut input_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        input_file.push("resources/test/test.mmap");

        let mut output_file = dirs::cache_dir().unwrap();
        output_file.push("1.log");

        let cmd = bin_under_test
            .command()
            .arg("-i")
            .arg(input_file.into_os_string())
            .arg("-o")
            .arg(output_file.into_os_string())
            .arg("-k")
            .arg("an example very very secret key.")
            .arg("-n")
            .arg("unique nonce")
            .unwrap();
        cmd.assert().success();
    }
}
