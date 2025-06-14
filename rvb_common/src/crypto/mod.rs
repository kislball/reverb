use base64::Engine;
#[cfg(feature = "encrypt")]
use ecies::{decrypt, encrypt};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey, ed25519::signature::SignerMut};
#[cfg(feature = "crypto_random")]
use rand::rngs::OsRng;

#[must_use] pub fn b64_encode(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

pub fn b64_decode(data: &str) -> Result<Vec<u8>, CryptoError> {
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|_| CryptoError::InvalidKey)
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),
    #[error("Key generation error: {0}")]
    KeyGenerationError(String),
    #[error("Invalid key")]
    InvalidKey,
}

#[derive(Clone, Debug)]
pub struct PublicKey {
    encrypting_key: VerifyingKey,
    verifying_key: VerifyingKey,
}

impl PublicKey {
    pub fn import(data: &[u8]) -> Result<PublicKey, CryptoError> {
        if data.len() != 64 {
            return Err(CryptoError::InvalidKey);
        }
        Ok(Self {
            verifying_key: VerifyingKey::try_from(&data[..32])
                .map_err(|_| CryptoError::InvalidKey)?,
            encrypting_key: VerifyingKey::try_from(&data[32..])
                .map_err(|_| CryptoError::InvalidKey)?,
        })
    }

    pub fn import_armored(data: &str) -> Result<Self, CryptoError> {
        let data = b64_decode(data)?;
        Self::import(&data)
    }

    #[must_use]
    pub fn armor(&self) -> String {
        b64_encode(&self.export())
    }

    #[must_use]
    pub fn export(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(64);

        v.extend_from_slice(self.verifying_key.as_bytes());
        v.extend_from_slice(self.encrypting_key.as_bytes());

        v
    }

    #[cfg(feature = "encrypt")]
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let public_key_bytes = self.encrypting_key.as_bytes();
        encrypt(public_key_bytes, data).map_err(|_| CryptoError::InvalidKey)
    }

    #[must_use]
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> bool {
        match Signature::from_slice(signature) {
            Ok(sign) => self.verifying_key.verify_strict(data, &sign).is_ok(),
            Err(_) => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct KeyPair {
    signing_pair: SigningKey,
    encrypting_pair: SigningKey,
}

impl KeyPair {
    #[must_use]
    #[cfg(feature = "crypto_random")]
    pub fn generate() -> Self {
        let mut rng = OsRng;
        Self {
            signing_pair: SigningKey::generate(&mut rng),
            encrypting_pair: SigningKey::generate(&mut rng),
        }
    }

    pub fn import_armored(data: &str) -> Result<Self, CryptoError> {
        let data = b64_decode(data)?;
        Self::import(&data)
    }

    pub fn import(data: &[u8]) -> Result<Self, CryptoError> {
        if data.len() != 64 {
            return Err(CryptoError::InvalidKey);
        }
        Ok(Self {
            signing_pair: SigningKey::try_from(&data[..32]).map_err(|_| CryptoError::InvalidKey)?,
            encrypting_pair: SigningKey::try_from(&data[32..])
                .map_err(|_| CryptoError::InvalidKey)?,
        })
    }

    #[must_use]
    pub fn public(&self) -> PublicKey {
        PublicKey {
            encrypting_key: self.encrypting_pair.verifying_key(),
            verifying_key: self.signing_pair.verifying_key(),
        }
    }

    #[must_use]
    pub fn armor_private(&self) -> String {
        b64_encode(&self.export_private())
    }

    #[must_use]
    pub fn armor_public(&self) -> String {
        b64_encode(&self.export_public())
    }

    #[must_use]
    pub fn export_private(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(64);

        v.extend_from_slice(self.signing_pair.as_bytes());
        v.extend_from_slice(self.encrypting_pair.as_bytes());

        v
    }

    #[must_use]
    pub fn export_public(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(64);

        v.extend_from_slice(self.signing_pair.verifying_key().as_bytes());
        v.extend_from_slice(self.encrypting_pair.verifying_key().as_bytes());

        v
    }

    pub fn sign(&mut self, data: &[u8]) -> Vec<u8> {
        self.signing_pair.sign(data).to_vec()
    }

    #[must_use]
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> bool {
        match Signature::from_slice(signature) {
            Ok(sign) => self.signing_pair.verify_strict(data, &sign).is_ok(),
            Err(_) => false,
        }
    }

    #[cfg(feature = "encrypt")]
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let verkey = self.encrypting_pair.verifying_key();
        let public_key_bytes = verkey.as_bytes();
        encrypt(public_key_bytes, data).map_err(|_| CryptoError::InvalidKey)
    }

    #[cfg(feature = "encrypt")]
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let private_key_bytes = self.encrypting_pair.as_bytes();
        decrypt(private_key_bytes, data).map_err(|_| CryptoError::InvalidKey)
    }
}

#[cfg(all(test, feature = "encrypt", feature = "crypto_random"))]
mod tests;
