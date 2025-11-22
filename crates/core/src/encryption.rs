use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use backupforge_common::{Error, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};

const NONCE_SIZE: usize = 12; // 96 bits for GCM

/// Encryption key for AES-256-GCM
#[derive(Clone)]
pub struct EncryptionKey {
    key: [u8; 32], // 256 bits
}

impl EncryptionKey {
    /// Generate a new random encryption key
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        Self { key }
    }

    /// Derive a key from a password using Argon2
    pub fn from_password(password: &str, salt: &[u8]) -> Result<Self> {
        let argon2 = Argon2::default();
        let salt_string = SaltString::encode_b64(salt)
            .map_err(|e| Error::Encryption(format!("Invalid salt: {}", e)))?;

        let hash = argon2
            .hash_password(password.as_bytes(), &salt_string)
            .map_err(|e| Error::Encryption(format!("Password hashing failed: {}", e)))?;

        let hash_bytes = hash
            .hash
            .ok_or_else(|| Error::Encryption("No hash generated".to_string()))?;

        let mut key = [0u8; 32];
        key.copy_from_slice(&hash_bytes.as_bytes()[..32]);

        Ok(Self { key })
    }

    /// Load key from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(Error::Encryption(
                "Key must be exactly 32 bytes".to_string(),
            ));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(bytes);
        Ok(Self { key })
    }

    /// Export key as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }
}

/// Encryptor for backup data using AES-256-GCM
pub struct Encryptor {
    key: EncryptionKey,
}

impl Encryptor {
    pub fn new(key: EncryptionKey) -> Self {
        Self { key }
    }

    /// Encrypt data, returns (nonce || ciphertext)
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.key.key)
            .map_err(|e| Error::Encryption(format!("Cipher creation failed: {}", e)))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| Error::Encryption(format!("Encryption failed: {}", e)))?;

        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt data from (nonce || ciphertext) format
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < NONCE_SIZE {
            return Err(Error::Encryption("Data too short".to_string()));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key.key)
            .map_err(|e| Error::Encryption(format!("Cipher creation failed: {}", e)))?;

        // Extract nonce and ciphertext
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| Error::Encryption(format!("Decryption failed: {}", e)))?;

        Ok(plaintext)
    }
}

/// Encrypted chunk metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedMetadata {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();

        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_key_from_password() {
        let salt = b"test_salt_123456";
        let key = EncryptionKey::from_password("my_password", salt).unwrap();
        assert_eq!(key.as_bytes().len(), 32);

        // Same password and salt should produce same key
        let key2 = EncryptionKey::from_password("my_password", salt).unwrap();
        assert_eq!(key.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_encryption_decryption() {
        let key = EncryptionKey::generate();
        let encryptor = Encryptor::new(key);

        let plaintext = b"Hello, World! This is secret data.";
        let encrypted = encryptor.encrypt(plaintext).unwrap();

        assert_ne!(plaintext.to_vec(), encrypted);
        assert!(encrypted.len() > plaintext.len()); // Includes nonce and auth tag

        let decrypted = encryptor.decrypt(&encrypted).unwrap();
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encryption_different_each_time() {
        let key = EncryptionKey::generate();
        let encryptor = Encryptor::new(key);

        let plaintext = b"test data";
        let encrypted1 = encryptor.encrypt(plaintext).unwrap();
        let encrypted2 = encryptor.encrypt(plaintext).unwrap();

        // Different nonces should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to same plaintext
        assert_eq!(
            encryptor.decrypt(&encrypted1).unwrap(),
            encryptor.decrypt(&encrypted2).unwrap()
        );
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let key = EncryptionKey::generate();
        let encryptor = Encryptor::new(key);

        // Too short
        let result = encryptor.decrypt(b"short");
        assert!(result.is_err());

        // Corrupted data
        let plaintext = b"test";
        let mut encrypted = encryptor.encrypt(plaintext).unwrap();
        encrypted[20] ^= 0xFF; // Flip some bits

        let result = encryptor.decrypt(&encrypted);
        assert!(result.is_err());
    }
}
