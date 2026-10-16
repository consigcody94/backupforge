use backupforge_common::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::encryption::{EncryptionKey, Encryptor, KdfParams};

/// Metadata key under which the repository config is stored.
pub const REPO_CONFIG_KEY: &str = "repo.json";

/// Highest on-disk format this build can read.
pub const FORMAT_VERSION: u32 = 1;

const KEY_CHECK_PLAINTEXT: &[u8] = b"backupforge key check v1";
const KEY_CHECK_AAD: &[u8] = b"bf1/key-check";

/// Repository header. Stored unencrypted: it holds only public KDF settings
/// and an encrypted check value used to tell a wrong password from corruption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub format_version: u32,
    pub encrypted: bool,
    pub kdf: Option<KdfParams>,
    /// Hex of `encrypt("backupforge key check v1")` under the derived key.
    pub key_check: Option<String>,
}

impl RepoConfig {
    pub fn plaintext() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            encrypted: false,
            kdf: None,
            key_check: None,
        }
    }

    /// Create an encrypted repository header and the matching key.
    pub fn new_encrypted(password: &str) -> Result<(Self, EncryptionKey)> {
        Self::new_encrypted_with(password, KdfParams::new_random())
    }

    /// Like [`Self::new_encrypted`] with caller-chosen KDF parameters.
    pub fn new_encrypted_with(password: &str, kdf: KdfParams) -> Result<(Self, EncryptionKey)> {
        let key = EncryptionKey::derive(password, &kdf)?;
        let check = Encryptor::new(&key).encrypt(KEY_CHECK_PLAINTEXT, KEY_CHECK_AAD)?;
        Ok((
            Self {
                format_version: FORMAT_VERSION,
                encrypted: true,
                kdf: Some(kdf),
                key_check: Some(hex::encode(check)),
            },
            key,
        ))
    }

    /// Derive and verify the key. Returns `None` for unencrypted repositories.
    pub fn unlock(&self, password: Option<&str>) -> Result<Option<EncryptionKey>> {
        if !self.encrypted {
            return Ok(None);
        }
        let password = password.ok_or_else(|| {
            Error::AuthenticationFailed(
                "This repository is encrypted; a password is required".to_string(),
            )
        })?;
        let kdf = self.kdf.as_ref().ok_or_else(|| {
            Error::InvalidConfig("Encrypted repository has no KDF settings".to_string())
        })?;
        let check = self.key_check.as_ref().ok_or_else(|| {
            Error::InvalidConfig("Encrypted repository has no key check".to_string())
        })?;
        let check = hex::decode(check)
            .map_err(|_| Error::InvalidConfig("Corrupt key check".to_string()))?;

        let key = EncryptionKey::derive(password, kdf)?;
        match Encryptor::new(&key).decrypt(&check, KEY_CHECK_AAD) {
            Ok(plain) if plain == KEY_CHECK_PLAINTEXT => Ok(Some(key)),
            _ => Err(Error::AuthenticationFailed("Wrong password".to_string())),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec_pretty(self).map_err(|e| Error::Serialization(e.to_string()))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let cfg: Self =
            serde_json::from_slice(bytes).map_err(|e| Error::Serialization(e.to_string()))?;
        if cfg.format_version > FORMAT_VERSION {
            return Err(Error::InvalidConfig(format!(
                "Repository format {} is newer than this build supports ({})",
                cfg.format_version, FORMAT_VERSION
            )));
        }
        Ok(cfg)
    }
}

#[cfg(test)]
#[path = "repo_tests.rs"]
mod tests;
