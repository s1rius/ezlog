use aead::KeyInit;
use aes_gcm::aead::Aead;
use aes_gcm::Nonce;

use crate::errors::LogError;
use crate::{Decryptor, Encryptor};

pub struct Aes256Gcm {
    // 96-bits; unique per message
    nonce: Vec<u8>,
    // aes256gcm
    cipher: aes_gcm::Aes256Gcm,
}

impl Aes256Gcm {
    pub fn new(key: &[u8], nonce: &[u8]) -> crate::Result<Self> {
        if nonce.len() != 12 {
            return Err(LogError::IllegalArgument(format!(
                "nonce must be 12 bytes, but current is {}",
                nonce.len()
            )));
        }
        match aes_gcm::Aes256Gcm::new_from_slice(key) {
            Ok(cipher) => Ok(Aes256Gcm {
                nonce: nonce.to_owned(),
                cipher,
            }),
            Err(e) => Err(LogError::IllegalArgument(format!(
                "key length invalid {}",
                e
            ))),
        }
    }
}

impl Encryptor for Aes256Gcm {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, LogError> {
        let nonce = Nonce::from_slice(&self.nonce); // 96-bits; unique per message
        self.cipher
            .encrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}

impl Decryptor for Aes256Gcm {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, LogError> {
        let nonce = Nonce::from_slice(&self.nonce);
        self.cipher
            .decrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}

pub struct Aes128Gcm {
    // 96-bits; unique per message
    nonce: Vec<u8>,
    // aes128gcm
    cipher: aes_gcm::Aes128Gcm,
}

impl Aes128Gcm {
    pub fn new(key: &[u8], nonce: &[u8]) -> crate::Result<Self> {
        if nonce.len() != 12 {
            return Err(LogError::IllegalArgument(format!(
                "nonce must be 12 bytes, but current is {}",
                nonce.len()
            )));
        }

        match aes_gcm::Aes128Gcm::new_from_slice(key) {
            Ok(cipher) => Ok(Aes128Gcm {
                nonce: nonce.to_owned(),
                cipher,
            }),
            Err(e) => Err(LogError::IllegalArgument(format!(
                "key length invalid {}",
                e
            ))),
        }
    }
}

impl Encryptor for Aes128Gcm {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, LogError> {
        let nonce = Nonce::from_slice(&self.nonce); // 96-bits; unique per message
        self.cipher
            .encrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}

impl Decryptor for Aes128Gcm {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, LogError> {
        let nonce = Nonce::from_slice(&self.nonce);
        self.cipher
            .decrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}
