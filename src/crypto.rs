use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Key, Nonce}; // Or `Aes128Gcm`

use crate::errors::{CryptoError, ParseError};
use crate::{Cryptor, Decryptor, Encryptor};

pub struct Aes256Gcm {
    // 256-bits; key
    key: Vec<u8>,
    // 96-bits; unique per message
    nonce: Vec<u8>,
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

        Ok(Aes256Gcm {
            key: key.to_owned(),
            nonce: nonce.to_owned(),
        })
    }
}

impl Encryptor for Aes256Gcm {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let key = Key::from_slice(&self.key);
        let cipher = aes_gcm::Aes256Gcm::new(key);

        let nonce = Nonce::from_slice(&self.nonce); // 96-bits; unique per message

        cipher.encrypt(nonce, data).map_err(|e| e.into())
    }
}

impl Decryptor for Aes256Gcm {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let key = Key::from_slice(&self.key);
        let cipher = aes_gcm::Aes256Gcm::new(key);

        let nonce = Nonce::from_slice(&self.nonce);

        cipher.decrypt(nonce, data).map_err(|e| e.into())
    }
}

pub struct Aes128Gcm {
    // 128-bits; key
    key: Vec<u8>,
    // 96-bits; unique per message
    nonce: Vec<u8>,
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

        Ok(Aes128Gcm {
            key: key.to_owned(),
            nonce: nonce.to_owned(),
        })
    }
}

impl Encryptor for Aes128Gcm {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let key = Key::from_slice(&self.key);
        let cipher = aes_gcm::Aes128Gcm::new(key);

        let nonce = Nonce::from_slice(&self.nonce); // 96-bits; unique per message

        cipher.encrypt(nonce, data).map_err(|e| e.into())
    }
}

impl Decryptor for Aes128Gcm {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let key = Key::from_slice(&self.key);
        let cipher = aes_gcm::Aes128Gcm::new(key);

        let nonce = Nonce::from_slice(&self.nonce);

        cipher.decrypt(nonce, data).map_err(|e| e.into())
    }
}
