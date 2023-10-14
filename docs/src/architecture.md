## Architecture

### Code structure

```
├── android
│   ├── app # android demo app
│   └── lib-ezlog # ezlog android library
├── examples # Rust examples
├── ezlog_flutter # Flutter plugin
├── ezlogcli # Rust command line tool
├── ezlog-core # Rust core library
├── ios
│   ├── EZLog # ezlog iOS library
│   ├── demo # iOS demo app
│   └── framework # ezlog XCFramework
```

### Log file format

#### Header 

| Bytes Offset | Meaning                            |
|--------|------------------------------------------|
| 0-1    | 'ez'                                     |
| 2      | Version number                           |
| 3      | Flag bits                                |
| 4-7    | Offset of recorder position in bytes     |
| 8-15   | Unix timestamp (big-endian)              |
| 16     | Compression type                         |
| 17     | Encryption type                          |
| 18-21  | Encryption key hash                      |

#### Per log record

| Byte Offset | Field Name| Description  |
|----------|-----------|-----------------|
| 0| Start Byte| Always 0x3b indicating the start|
| 1-varint|Record Length| A variable-length integer that specifies the length|
| varint+1-varint+n | Record Content | The actual log record content |
| varint+n+1| End Byte| Always 0x21 indicating the start |

### Compression

We use zlib as the compression algorithm.

### Encryption

#### We use AES-GCM-SIV as the encryption algorithm.

AES-GCM-SIV, as a symmetric encryption algorithm, is more efficient compared to asymmetric encryption. As an AEAD, When compared to AES-CFB, it is more secure, and when compared to AES-GCM, AES-GCM-SIV is nonce-misuse-resistant.

### Make nonce not repeat

First of all, we need an init nonce, which is generated randomly when the logger is created. Then, we get the timestamp of the log file creation. When we write a log record, we know the current index of the log file, and we can calculate the nonce of the current log record by the following formula:

```
nonce = init_nonce ^ timestamp.extend(index)
```
