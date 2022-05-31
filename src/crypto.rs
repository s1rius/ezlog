use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Key, Nonce}; // Or `Aes128Gcm`

use crate::errors::{CryptoError, ParseError};
use crate::{Decryptor, Encryptor};

pub struct Aes256Gcm {
    // 96-bits; unique per message
    nonce: Vec<u8>,
    // aes256gcm
    cipher: aes_gcm::Aes256Gcm,
}

impl Aes256Gcm {
    pub fn new(key: &[u8], nonce: &[u8]) -> Result<Self, CryptoError> {
        if key.len() != 32 {
            // todo s1rius correct error type
            return Err(CryptoError::new(ParseError::new(format!(
                "key must be 32 bytes long {}",
                key.len()
            ))));
        }

        if nonce.len() != 12 {
            return Err(CryptoError::new(ParseError::new(format!(
                "nonce must be 12 bytes long {}",
                nonce.len()
            ))));
        }
        let _key = Key::from_slice(key);
        Ok(Aes256Gcm {
            nonce: nonce.to_owned(),
            cipher: aes_gcm::Aes256Gcm::new(_key),
        })
    }
}

impl Encryptor for Aes256Gcm {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let nonce = Nonce::from_slice(&self.nonce); // 96-bits; unique per message
        self.cipher.encrypt(nonce, data).map_err(|e| e.into())
    }
}

impl Decryptor for Aes256Gcm {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let nonce = Nonce::from_slice(&self.nonce);
        self.cipher.decrypt(nonce, data).map_err(|e| e.into())
    }
}

pub struct Aes128Gcm {
    // 96-bits; unique per message
    nonce: Vec<u8>,
    // aes128gcm
    cipher: aes_gcm::Aes128Gcm,
}

impl Aes128Gcm {
    pub fn new(key: &[u8], nonce: &[u8]) -> Result<Self, CryptoError> {
        if key.len() != 16 {
            return Err(CryptoError::new(ParseError::new(format!(
                "key must be 16 bytes long {}",
                key.len()
            ))));
        }

        if nonce.len() != 12 {
            return Err(CryptoError::new(ParseError::new(format!(
                "nonce must be 12 bytes long {}",
                key.len()
            ))));
        }

        let _key = Key::from_slice(&key);

        Ok(Aes128Gcm {
            nonce: nonce.to_owned(),
            cipher: aes_gcm::Aes128Gcm::new(_key),
        })
    }
}

impl Encryptor for Aes128Gcm {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let nonce = Nonce::from_slice(&self.nonce); // 96-bits; unique per message
        self.cipher.encrypt(nonce, data).map_err(|e| e.into())
    }
}

impl Decryptor for Aes128Gcm {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let nonce = Nonce::from_slice(&self.nonce);
        self.cipher.decrypt(nonce, data).map_err(|e| e.into())
    }
}
