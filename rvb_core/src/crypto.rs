use rsa::Pkcs1v15Encrypt;
use rsa::pkcs1v15::{Signature, SigningKey, VerifyingKey};
use rsa::signature::{Signer, Verifier, Keypair, SignatureEncoding};
use rsa::{
    Error, RsaPrivateKey, RsaPublicKey,
    pkcs1::{self, DecodeRsaPrivateKey, EncodeRsaPrivateKey, EncodeRsaPublicKey},
};

pub const RSA_KEY_SIZE: usize = 4096;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(pkcs1::Error),
    #[error("Key generation error: {0}")]
    KeyGenerationError(Error),
    #[error("Invalid key")]
    InvalidKey,
}

#[derive(Clone, Debug)]
pub struct PublicKey {
    public: RsaPublicKey,
    verifying_key: VerifyingKey<sha2::Sha256>,
}

impl PublicKey {
    pub fn new(key: RsaPublicKey) -> Self {
        let verifying_key = VerifyingKey::<sha2::Sha256>::from(key.clone());
        Self { 
            public: key,
            verifying_key,
        }
    }

    pub fn from_signing_key(signing_key: &SigningKey<sha2::Sha256>) -> Self {
        let verifying_key = signing_key.verifying_key();
        let public = signing_key.as_ref().clone(); // Get the RsaPrivateKey and convert to public
        let public = RsaPublicKey::from(public);
        Self {
            public,
            verifying_key,
        }
    }

    fn get_verifying_key(&self) -> VerifyingKey<sha2::Sha256> {
        self.verifying_key.clone()
    }

    pub fn armor(&self) -> String {
        self.public
            .to_pkcs1_pem(rsa::pkcs8::LineEnding::CRLF)
            .unwrap()
            .to_string()
    }

    pub fn export(&self) -> Vec<u8> {
        self.public.to_pkcs1_der().unwrap().as_bytes().to_vec()
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut rng = rand::thread_rng();
        self.public
            .encrypt(&mut rng, Pkcs1v15Encrypt, data)
            .map_err(|_| CryptoError::InvalidKey)
    }

    pub fn verify(&self, data: &[u8], signature: &[u8]) -> bool {
        match Signature::try_from(signature) {
            Ok(sig) => self.verifying_key.verify(data, &sig).is_ok(),
            Err(_) => false
        }
    }
}

#[derive(Clone, Debug)]
pub struct KeyPair {
    private: RsaPrivateKey,
    public: PublicKey,
}

impl KeyPair {
    pub fn new(key: RsaPrivateKey) -> Self {
        let signing_key = SigningKey::<sha2::Sha256>::new(key.clone());
        Self {
            public: PublicKey::from_signing_key(&signing_key),
            private: key,
        }
    }

    pub fn generate() -> Result<Self, CryptoError> {
        let mut thread_rng = rand::thread_rng();
        let private = RsaPrivateKey::new(&mut thread_rng, RSA_KEY_SIZE)
            .map_err(CryptoError::KeyGenerationError)?;
        Ok(KeyPair::new(private))
    }

    pub fn from_der(der: &[u8]) -> Result<Self, CryptoError> {
        let private = RsaPrivateKey::from_pkcs1_der(der).map_err(CryptoError::InvalidKeyFormat)?;
        Ok(Self::new(private))
    }

    pub fn from_pem(pem: &str) -> Result<Self, CryptoError> {
        let private = RsaPrivateKey::from_pkcs1_pem(pem).map_err(CryptoError::InvalidKeyFormat)?;
        Ok(Self::new(private))
    }

    pub fn armor_private(&self) -> String {
        self.private
            .to_pkcs1_pem(rsa::pkcs8::LineEnding::CRLF)
            .unwrap()
            .to_string()
    }

    pub fn armor_public(&self) -> String {
        self.public.armor()
    }

    pub fn export_private(&self) -> Vec<u8> {
        self.private.to_pkcs1_der().unwrap().to_bytes().to_vec()
    }

    pub fn export_public(&self) -> Vec<u8> {
        self.public.export()
    }

    fn get_signing_key(&self) -> SigningKey<sha2::Sha256> {
        SigningKey::<sha2::Sha256>::new(self.private.clone())
    }

    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let signing_key = SigningKey::<sha2::Sha256>::new(self.private.clone());
        signing_key.sign(data).to_vec()
    }

    pub fn verify(&self, data: &[u8], signature: &[u8]) -> bool {
        let signing_key = SigningKey::<sha2::Sha256>::new(self.private.clone());
        let verifying_key = signing_key.verifying_key();
        match Signature::try_from(signature) {
            Ok(sig) => verifying_key.verify(data, &sig).is_ok(),
            Err(_) => false
        }
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.public.encrypt(data)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.private
            .decrypt(Pkcs1v15Encrypt, data)
            .map_err(|_| CryptoError::InvalidKey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generate_and_export_import() {
        let keypair = KeyPair::generate().unwrap();
        let exported = keypair.export_private();
        let imported = KeyPair::from_der(&exported).unwrap();
        assert_eq!(keypair.export_public(), imported.export_public());
    }

    #[test]
    fn test_keypair_pem_roundtrip() {
        let keypair = KeyPair::generate().unwrap();
        let pem = keypair.armor_private();
        let imported = KeyPair::from_pem(&pem).unwrap();
        assert_eq!(keypair.export_public(), imported.export_public());
    }

    #[test]
    fn test_public_key_armor_and_export() {
        let keypair = KeyPair::generate().unwrap();
        let public = keypair.public.clone();
        let armored = public.armor();
        assert!(armored.contains("BEGIN RSA PUBLIC KEY"));
        let exported = public.export();
        assert!(!exported.is_empty());
    }

    #[test]
    fn test_encrypt_decrypt() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"hello world";
        let encrypted = keypair.encrypt(message).unwrap();
        let decrypted = keypair.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, message);
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate().unwrap();
        let message = b"test message";
        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature));
        assert!(!keypair.verify(b"wrong message", &signature));
    }

    #[test]
    fn test_public_key_verify() {
        let keypair = KeyPair::generate().unwrap();
        let public = keypair.public.clone();
        let message = b"verify me";
        let signature = keypair.sign(message);
        assert!(public.verify(message, &signature));
        assert!(!public.verify(b"wrong", &signature));
    }

    #[test]
    fn test_invalid_signature() {
        let keypair = KeyPair::generate().unwrap();
        let public = keypair.public.clone();
        let message = b"msg";
        let invalid_signature = vec![0u8; 32];
        assert!(!public.verify(message, &invalid_signature));
    }

    #[test]
    fn test_invalid_decrypt() {
        let keypair = KeyPair::generate().unwrap();
        let invalid_data = vec![1, 2, 3, 4, 5];
        assert!(keypair.decrypt(&invalid_data).is_err());
    }

    #[test]
    fn test_invalid_key_import() {
        // Invalid DER
        assert!(KeyPair::from_der(&[1, 2, 3]).is_err());
        // Invalid PEM
        assert!(KeyPair::from_pem("not a pem").is_err());
    }
}
