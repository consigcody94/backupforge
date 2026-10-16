use aes_gcm::{
    aead::{Aead, KeyInit, OsRng, Payload},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use backupforge_common::{Error, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

const NONCE_SIZE: usize = 12; // 96 bits for GCM
const KEY_SIZE: usize = 32;
const MIN_SALT_SIZE: usize = 16;

/// Argon2id parameters, stored with the repository so they can be raised later
/// without breaking existing backups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    /// Memory cost in KiB
    pub memory_kib: u32,
    /// Number of passes
    pub iterations: u32,
    /// Degree of parallelism
    pub parallelism: u32,
    /// Random salt (at least 16 bytes)
    pub salt: Vec<u8>,
}

impl KdfParams {
    /// Default cost for new repositories: 64 MiB, 3 passes, 1 lane.
    pub fn new_random() -> Self {
        let mut salt = vec![0u8; 16];
        OsRng.fill_bytes(&mut salt);
        Self {
            memory_kib: 64 * 1024,
            iterations: 3,
            parallelism: 1,
            salt,
        }
    }
}

/// Master key for a repository. Zeroized when dropped.
///
/// Purpose-specific subkeys (chunk encryption, chunk-ID MAC) are derived from
/// it with BLAKE3 `derive_key`, so one key is never used for two jobs.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey {
    key: [u8; KEY_SIZE],
}

impl EncryptionKey {
    /// Generate a new random encryption key
    pub fn generate() -> Self {
        let mut key = [0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key);
        Self { key }
    }

    /// Derive a key from a password using Argon2id with explicit parameters
    pub fn derive(password: &str, params: &KdfParams) -> Result<Self> {
        if params.salt.len() < MIN_SALT_SIZE {
            return Err(Error::Encryption(
                "Salt must be at least 16 bytes".to_string(),
            ));
        }
        let argon_params = Params::new(
            params.memory_kib,
            params.iterations,
            params.parallelism,
            Some(KEY_SIZE),
        )
        .map_err(|e| Error::Encryption(format!("Invalid KDF parameters: {}", e)))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);

        let mut key = [0u8; KEY_SIZE];
        argon2
            .hash_password_into(password.as_bytes(), &params.salt, &mut key)
            .map_err(|e| Error::Encryption(format!("Key derivation failed: {}", e)))?;

        Ok(Self { key })
    }

    /// Load key from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != KEY_SIZE {
            return Err(Error::Encryption(
                "Key must be exactly 32 bytes".to_string(),
            ));
        }

        let mut key = [0u8; KEY_SIZE];
        key.copy_from_slice(bytes);
        Ok(Self { key })
    }

    /// Export key as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }

    /// Derive a 32-byte subkey bound to `context`.
    pub fn subkey(&self, context: &str) -> [u8; KEY_SIZE] {
        blake3::derive_key(&format!("backupforge 2026 v1 {}", context), &self.key)
    }
}

/// AES-256-GCM encryptor with caller-supplied associated data.
///
/// Every message gets a fresh random 96-bit nonce. With random nonces the
/// safe limit is well below 2^32 messages per key; see docs/THREAT_MODEL.md.
pub struct Encryptor {
    key: [u8; KEY_SIZE],
}

impl Drop for Encryptor {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl Encryptor {
    pub fn new(key: &EncryptionKey) -> Self {
        Self {
            key: key.subkey("aead"),
        }
    }

    /// Encrypt data, returns (nonce || ciphertext || tag).
    ///
    /// `aad` is authenticated but not encrypted. Callers bind the object's
    /// identity here so ciphertexts cannot be swapped between objects.
    pub fn encrypt(&self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| Error::Encryption(format!("Cipher creation failed: {}", e)))?;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(
                nonce,
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| Error::Encryption("Encryption failed".to_string()))?;

        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt data from (nonce || ciphertext || tag) format
    pub fn decrypt(&self, data: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        if data.len() < NONCE_SIZE {
            return Err(Error::Encryption("Data too short".to_string()));
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| Error::Encryption(format!("Cipher creation failed: {}", e)))?;

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        cipher
            .decrypt(
                nonce,
                Payload {
                    msg: ciphertext,
                    aad,
                },
            )
            .map_err(|_| {
                Error::Encryption("Decryption failed: wrong key or corrupted data".to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_params() -> KdfParams {
        KdfParams {
            memory_kib: 8,
            iterations: 1,
            parallelism: 1,
            salt: vec![7u8; 16],
        }
    }

    #[test]
    fn test_key_generation() {
        let key1 = EncryptionKey::generate();
        let key2 = EncryptionKey::generate();

        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_key_derivation_is_deterministic_and_salted() {
        let params = fast_params();
        let key = EncryptionKey::derive("my_password", &params).unwrap();
        let key2 = EncryptionKey::derive("my_password", &params).unwrap();
        assert_eq!(key.as_bytes(), key2.as_bytes());

        let other_pw = EncryptionKey::derive("other", &params).unwrap();
        assert_ne!(key.as_bytes(), other_pw.as_bytes());

        let mut other_salt = fast_params();
        other_salt.salt = vec![8u8; 16];
        let key3 = EncryptionKey::derive("my_password", &other_salt).unwrap();
        assert_ne!(key.as_bytes(), key3.as_bytes());
    }

    #[test]
    fn test_short_salt_rejected() {
        let mut params = fast_params();
        params.salt = vec![1u8; 8];
        assert!(EncryptionKey::derive("pw", &params).is_err());
    }

    #[test]
    fn test_subkeys_differ_by_context() {
        let key = EncryptionKey::generate();
        assert_ne!(key.subkey("aead"), key.subkey("chunk-id"));
        assert_ne!(key.subkey("aead"), key.as_bytes());
    }

    #[test]
    fn test_encryption_decryption() {
        let encryptor = Encryptor::new(&EncryptionKey::generate());

        let plaintext = b"Hello, World! This is secret data.";
        let encrypted = encryptor.encrypt(plaintext, b"ctx").unwrap();

        assert_ne!(plaintext.to_vec(), encrypted);
        assert_eq!(encrypted.len(), NONCE_SIZE + plaintext.len() + 16);

        let decrypted = encryptor.decrypt(&encrypted, b"ctx").unwrap();
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encryption_different_each_time() {
        let encryptor = Encryptor::new(&EncryptionKey::generate());

        let encrypted1 = encryptor.encrypt(b"test data", b"").unwrap();
        let encrypted2 = encryptor.encrypt(b"test data", b"").unwrap();
        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_wrong_aad_rejected() {
        let encryptor = Encryptor::new(&EncryptionKey::generate());
        let encrypted = encryptor.encrypt(b"payload", b"chunk-a").unwrap();
        assert!(encryptor.decrypt(&encrypted, b"chunk-b").is_err());
    }

    #[test]
    fn test_wrong_key_rejected() {
        let a = Encryptor::new(&EncryptionKey::generate());
        let b = Encryptor::new(&EncryptionKey::generate());
        let encrypted = a.encrypt(b"payload", b"x").unwrap();
        assert!(b.decrypt(&encrypted, b"x").is_err());
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let encryptor = Encryptor::new(&EncryptionKey::generate());

        assert!(encryptor.decrypt(b"short", b"").is_err());

        // Flip a bit in the nonce, the ciphertext, and the tag in turn
        let encrypted = encryptor.encrypt(b"test", b"").unwrap();
        for i in [0, NONCE_SIZE, encrypted.len() - 1] {
            let mut tampered = encrypted.clone();
            tampered[i] ^= 0x01;
            assert!(encryptor.decrypt(&tampered, b"").is_err(), "byte {i}");
        }

        // Truncation
        assert!(encryptor
            .decrypt(&encrypted[..encrypted.len() - 1], b"")
            .is_err());
    }
}
