# EZLog command line tool

```
Usage: ezlog-cli [OPTIONS]

Options:
  -i, --input <FILE>
          Origin ezlog file path

  -o, --output <FILE>
          Decode log file path

  -c, --config <FILE>
          Sets a JSON config file contains the configuration of the logger.
          
          { "key": "an example very very secret key.", "nonce": "unique nonce" }

  -k, --key <KEY>
          Decrypt key

  -n, --nonce <NONCE>
          Decrypt nonce

  -d, --debug
          Turn debugging information on

  -h, --help
          Print help information (use `-h` for a summary)

  -V, --version
          Print version information

```          