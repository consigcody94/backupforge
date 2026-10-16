use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
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

        // Fresh random 96-bit nonce from the OS RNG for every message.
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: plaintext,
                    aad,
                },
            )
            .map_err(|_| Error::Encryption("Encryption failed".to_string()))?;

        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(nonce.as_slice());
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
#[path = "encryption_tests.rs"]
mod tests;
