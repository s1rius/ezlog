use aead::{
    Aead,
    KeyInit,
};
use aes_gcm_siv::Nonce;

use crate::errors::LogError;
use crate::{
    Decryptor,
    Encryptor,
};

pub struct Aes256GcmSiv {
    // 96-bits; unique per message
    nonce: Vec<u8>,
    // aes256gcmSiv
    cipher: aes_gcm_siv::Aes256GcmSiv,
}

impl Aes256GcmSiv {
    pub fn new(key: &[u8], nonce: &[u8]) -> crate::Result<Self> {
        if nonce.len() != 12 {
            return Err(LogError::Illegal(format!(
                "nonce must be 12 bytes, but current is {}",
                nonce.len()
            )));
        }
        match aes_gcm_siv::Aes256GcmSiv::new_from_slice(key) {
            Ok(cipher) => Ok(Aes256GcmSiv {
                nonce: nonce.to_owned(),
                cipher,
            }),
            Err(e) => Err(LogError::Illegal(format!("key length invalid {}", e))),
        }
    }
}

impl Encryptor for Aes256GcmSiv {
    fn encrypt(
        &self,
        data: &[u8],
        op: Box<dyn Fn(&[u8]) -> Vec<u8>>,
    ) -> std::result::Result<Vec<u8>, LogError> {
        let new_nonce = op(&self.nonce);
        let nonce = Nonce::from_slice(new_nonce.as_slice()); // 96-bits; unique per message
        self.cipher
            .encrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}

impl Decryptor for Aes256GcmSiv {
    fn decrypt(&self, data: &[u8], op: Box<dyn Fn(&[u8]) -> Vec<u8>>) -> Result<Vec<u8>, LogError> {
        let new_nonce = op(&self.nonce);
        let nonce = Nonce::from_slice(&new_nonce);
        self.cipher
            .decrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}

pub struct Aes128GcmSiv {
    // 96-bits; unique per message
    nonce: Vec<u8>,
    // aes256gcmSiv
    cipher: aes_gcm_siv::Aes128GcmSiv,
}

impl Aes128GcmSiv {
    pub fn new(key: &[u8], nonce: &[u8]) -> crate::Result<Self> {
        if nonce.len() != 12 {
            return Err(LogError::Illegal(format!(
                "nonce must be 12 bytes, but current is {}",
                nonce.len()
            )));
        }
        match aes_gcm_siv::Aes128GcmSiv::new_from_slice(key) {
            Ok(cipher) => Ok(Aes128GcmSiv {
                nonce: nonce.to_owned(),
                cipher,
            }),
            Err(e) => Err(LogError::Illegal(format!("key length invalid {}", e))),
        }
    }
}

impl Encryptor for Aes128GcmSiv {
    fn encrypt(
        &self,
        data: &[u8],
        op: Box<dyn Fn(&[u8]) -> Vec<u8>>,
    ) -> std::result::Result<Vec<u8>, LogError> {
        let new_nonce = op(&self.nonce);
        let nonce = Nonce::from_slice(new_nonce.as_slice()); // 96-bits; unique per message
        self.cipher
            .encrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}

impl Decryptor for Aes128GcmSiv {
    fn decrypt(&self, data: &[u8], op: Box<dyn Fn(&[u8]) -> Vec<u8>>) -> Result<Vec<u8>, LogError> {
        let new_nonce = op(&self.nonce);
        let nonce = Nonce::from_slice(&new_nonce);
        self.cipher
            .decrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}

#[cfg(feature = "decode")]
pub struct Aes256Gcm {
    // 96-bits; unique per message
    nonce: Vec<u8>,
    // aes256gcm
    cipher: aes_gcm::Aes256Gcm,
}

#[cfg(feature = "decode")]
impl Aes256Gcm {
    pub fn new(key: &[u8], nonce: &[u8]) -> crate::Result<Self> {
        if nonce.len() != 12 {
            return Err(LogError::Illegal(format!(
                "nonce must be 12 bytes, but current is {}",
                nonce.len()
            )));
        }
        match aes_gcm::Aes256Gcm::new_from_slice(key) {
            Ok(cipher) => Ok(Aes256Gcm {
                nonce: nonce.to_owned(),
                cipher,
            }),
            Err(e) => Err(LogError::Illegal(format!("key length invalid {}", e))),
        }
    }
}

#[cfg(feature = "decode")]
impl Encryptor for Aes256Gcm {
    fn encrypt(
        &self,
        data: &[u8],
        op: Box<dyn Fn(&[u8]) -> Vec<u8>>,
    ) -> std::result::Result<Vec<u8>, LogError> {
        let new_nonce = op(&self.nonce);
        let nonce = Nonce::from_slice(new_nonce.as_slice()); // 96-bits; unique per message
        self.cipher
            .encrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}

#[cfg(feature = "decode")]
impl Decryptor for Aes256Gcm {
    fn decrypt(&self, data: &[u8], op: Box<dyn Fn(&[u8]) -> Vec<u8>>) -> Result<Vec<u8>, LogError> {
        let new_nonce = op(&self.nonce);
        let nonce = Nonce::from_slice(&new_nonce);
        self.cipher
            .decrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}

#[cfg(feature = "decode")]
pub struct Aes128Gcm {
    // 96-bits; unique per message
    nonce: Vec<u8>,
    // aes128gcm
    cipher: aes_gcm::Aes128Gcm,
}

#[cfg(feature = "decode")]
impl Aes128Gcm {
    pub fn new(key: &[u8], nonce: &[u8]) -> crate::Result<Self> {
        if nonce.len() != 12 {
            return Err(LogError::Illegal(format!(
                "nonce must be 12 bytes, but current is {}",
                nonce.len()
            )));
        }

        match aes_gcm::Aes128Gcm::new_from_slice(key) {
            Ok(cipher) => Ok(Aes128Gcm {
                nonce: nonce.to_owned(),
                cipher,
            }),
            Err(e) => Err(LogError::Illegal(format!("key length invalid {}", e))),
        }
    }
}

#[cfg(feature = "decode")]
impl Encryptor for Aes128Gcm {
    fn encrypt(&self, data: &[u8], op: Box<dyn Fn(&[u8]) -> Vec<u8>>) -> Result<Vec<u8>, LogError> {
        let new_nonce = op(&self.nonce);
        let nonce = Nonce::from_slice(new_nonce.as_slice()); // 96-bits; unique per message
        self.cipher
            .encrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}

#[cfg(feature = "decode")]
impl Decryptor for Aes128Gcm {
    fn decrypt(&self, data: &[u8], op: Box<dyn Fn(&[u8]) -> Vec<u8>>) -> Result<Vec<u8>, LogError> {
        let new_nonce = op(&self.nonce);
        let nonce = Nonce::from_slice(&new_nonce);
        self.cipher
            .decrypt(nonce, data)
            .map_err(|e| LogError::Crypto(format!("{e:?}")))
    }
}
