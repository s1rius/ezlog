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

/// Cipher kind current support
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
#[cfg_attr(feature = "json", derive(serde::Serialize, serde::Deserialize))]
pub enum CipherKind {
    #[deprecated(since = "0.2.0", note = "Use AES128GCMSIV instead")]
    AES128GCM,
    #[deprecated(since = "0.2.0", note = "Use AES256GCMSIV instead")]
    AES256GCM,
    AES128GCMSIV,
    AES256GCMSIV,
    NONE,
    UNKNOWN,
}

#[allow(deprecated)]
impl From<u8> for CipherKind {
    fn from(orig: u8) -> Self {
        match orig {
            0x00 => CipherKind::NONE,
            0x01 => CipherKind::AES128GCM,
            0x02 => CipherKind::AES256GCM,
            0x03 => CipherKind::AES128GCMSIV,
            0x04 => CipherKind::AES256GCMSIV,
            _ => CipherKind::UNKNOWN,
        }
    }
}

#[allow(deprecated)]
impl From<CipherKind> for u8 {
    fn from(orig: CipherKind) -> Self {
        match orig {
            CipherKind::NONE => 0x00,
            CipherKind::AES128GCM => 0x01,
            CipherKind::AES256GCM => 0x02,
            CipherKind::AES128GCMSIV => 0x03,
            CipherKind::AES256GCMSIV => 0x04,
            CipherKind::UNKNOWN => 0xff,
        }
    }
}

#[allow(deprecated)]
impl core::fmt::Display for CipherKind {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            CipherKind::AES128GCM => write!(f, "AEAD_AES_128_GCM"),
            CipherKind::AES256GCM => write!(f, "AEAD_AES_256_GCM"),
            CipherKind::AES128GCMSIV => write!(f, "AEAD_AES_128_GCM_SIV"),
            CipherKind::AES256GCMSIV => write!(f, "AEAD_AES_128_GCM_SIV"),
            CipherKind::NONE => write!(f, "NONE"),
            _ => write!(f, "UNKNOWN"),
        }
    }
}

#[allow(deprecated)]
impl std::str::FromStr for CipherKind {
    type Err = LogError;

    fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "AEAD_AES_128_GCM" => Ok(CipherKind::AES128GCM),
            "AEAD_AES_256_GCM" => Ok(CipherKind::AES256GCM),
            "AEAD_AES_128_GCM_SIV" => Ok(CipherKind::AES128GCMSIV),
            "AEAD_AES_256_GCM_SIV" => Ok(CipherKind::AES256GCMSIV),
            "NONE" => Ok(CipherKind::NONE),
            _ => Err(crate::errors::LogError::Parse(
                "unknown cipher kind".to_string(),
            )),
        }
    }
}

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
