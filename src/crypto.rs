use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Key, Nonce}; // Or `Aes128Gcm`

use crate::errors::{CrytoError, ParseError};
use crate::{Decryptor, Encryptor};

pub struct Aes256Gcm {
    // 256-bits; key
    key: Vec<u8>,
    // 96-bits; unique per message
    nonce: Vec<u8>,
}

impl Aes256Gcm {
    pub fn new(key: &[u8], nonce: &[u8]) -> Result<Self, CrytoError> {
        if key.len() != 32 {
            // todo s1rius correct error type
            return Err(CrytoError::new(ParseError::new(format!(
                "key must be 32 bytes long"
            ))));
        }

        if nonce.len() != 12 {
            return Err(CrytoError::new(ParseError::new(format!(
                "nonce must be 12 bytes long"
            ))));
        }

        Ok(Aes256Gcm {
            key: key.to_owned(),
            nonce: nonce.to_owned(),
        })
    }
}

impl Encryptor for Aes256Gcm {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CrytoError> {
        let key = Key::from_slice(&self.key);
        let cipher = aes_gcm::Aes256Gcm::new(key);

        let nonce = Nonce::from_slice(&self.nonce); // 96-bits; unique per message

        cipher.encrypt(nonce, data).map_err(|e| e.into())
    }
}

impl Decryptor for Aes256Gcm {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CrytoError> {
        let key = Key::from_slice(&self.key);
        let cipher = aes_gcm::Aes256Gcm::new(key);

        let nonce = Nonce::from_slice(&self.nonce);

        cipher.decrypt(nonce, data).map_err(|e| e.into())
    }
}

pub struct Aes128Gcm {
    // 256-bits; key
    key: Vec<u8>,
    // 96-bits; unique per message
    nonce: Vec<u8>,
}

impl Aes128Gcm {
    pub fn new(key: &[u8], nonce: &[u8]) -> Result<Self, CrytoError> {
        if key.len() != 16 {
            return Err(CrytoError::new(ParseError::new(format!(
                "key must be 32 bytes long"
            ))));
        }

        if nonce.len() != 12 {
            return Err(CrytoError::new(ParseError::new(format!(
                "nonce must be 12 bytes long"
            ))));
        }

        Ok(Aes128Gcm {
            key: key.to_owned(),
            nonce: nonce.to_owned(),
        })
    }
}

impl Encryptor for Aes128Gcm {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CrytoError> {
        let key = Key::from_slice(&self.key);
        let cipher = aes_gcm::Aes128Gcm::new(key);

        let nonce = Nonce::from_slice(&self.nonce); // 96-bits; unique per message

        cipher.encrypt(nonce, data).map_err(|e| e.into())
    }
}

impl Decryptor for Aes128Gcm {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CrytoError> {
        let key = Key::from_slice(&self.key);
        let cipher = aes_gcm::Aes128Gcm::new(key);

        let nonce = Nonce::from_slice(&self.nonce);

        cipher.decrypt(nonce, data).map_err(|e| e.into())
    }
}
