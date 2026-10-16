use backupforge_common::{
    hash::hash_data,
    types::{ChunkId, FileMetadata, Snapshot, SnapshotId},
    Error, Result,
};
use chrono::Utc;
use zeroize::Zeroizing;

use crate::{
    chunker::{Chunker, ChunkingStrategy},
    compression::{CompressionAlgorithm, Compressor},
    dedup::DedupStore,
    encryption::{EncryptionKey, Encryptor},
};
use std::sync::Arc;

/// Sealed-object format tags (first byte of every stored chunk or blob).
const FORMAT_PLAIN: u8 = 1;
const FORMAT_ENCRYPTED: u8 = 2;

/// Configuration for the backup engine
#[derive(Clone)]
pub struct BackupConfig {
    pub chunking_strategy: ChunkingStrategy,
    pub compression: CompressionAlgorithm,
    pub encryption_key: Option<EncryptionKey>,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            chunking_strategy: ChunkingStrategy::default(),
            compression: CompressionAlgorithm::default(),
            encryption_key: None,
        }
    }
}

/// A chunk ready to be written to storage.
pub struct SealedChunk {
    pub id: ChunkId,
    /// Length of the chunk before compression and encryption
    pub plain_len: u64,
    /// Stored bytes: format tag, then (optionally encrypted) compressed data
    pub bytes: Vec<u8>,
}

/// Turns data into storable chunks and back: chunk -> compress -> encrypt.
///
/// Chunk IDs are BLAKE3 of the plaintext. When encryption is on they are a
/// keyed BLAKE3 MAC instead, so the storage side cannot confirm that a known
/// file is present. Ciphertexts are bound to their chunk ID as associated data,
/// so a valid chunk cannot be swapped for another, and every restored chunk is
/// re-hashed against its ID.
pub struct BackupEngine {
    chunker: Chunker,
    compressor: Compressor,
    encryptor: Option<Encryptor>,
    id_key: Option<Zeroizing<[u8; 32]>>,
    dedup_store: Arc<DedupStore>,
}

impl BackupEngine {
    pub fn new(config: BackupConfig) -> Self {
        let chunker = Chunker::new(config.chunking_strategy.clone());
        let compressor = Compressor::new(config.compression);
        let encryptor = config.encryption_key.as_ref().map(Encryptor::new);
        let id_key = config
            .encryption_key
            .as_ref()
            .map(|k| Zeroizing::new(k.subkey("chunk-id")));

        Self {
            chunker,
            compressor,
            encryptor,
            id_key,
            dedup_store: Arc::new(DedupStore::new()),
        }
    }

    pub fn is_encrypted(&self) -> bool {
        self.encryptor.is_some()
    }

    fn chunk_id_for(&self, data: &[u8]) -> ChunkId {
        match &self.id_key {
            Some(key) => ChunkId::from_hash(blake3::keyed_hash(key, data).as_bytes()),
            None => ChunkId::from_hash(&hash_data(data)),
        }
    }

    fn chunk_aad(id: &ChunkId) -> Vec<u8> {
        format!("bf1/chunk/{}", id.0).into_bytes()
    }

    fn blob_aad(kind: &str, id: &str) -> Vec<u8> {
        format!("bf1/blob/{}/{}", kind, id).into_bytes()
    }

    /// Chunk, compress and (if enabled) encrypt `data`.
    pub fn seal_data(&self, data: &[u8]) -> Result<Vec<SealedChunk>> {
        let mut sealed = Vec::new();

        for chunk in self.chunker.chunk_data(data)? {
            let id = self.chunk_id_for(&chunk.data);

            let mut payload = vec![self.compressor.algorithm().tag()];
            payload.extend_from_slice(&self.compressor.compress(&chunk.data)?);

            let bytes = match &self.encryptor {
                Some(enc) => {
                    let mut out = vec![FORMAT_ENCRYPTED];
                    out.extend_from_slice(&enc.encrypt(&payload, &Self::chunk_aad(&id))?);
                    out
                }
                None => {
                    let mut out = vec![FORMAT_PLAIN];
                    out.extend_from_slice(&payload);
                    out
                }
            };

            sealed.push(SealedChunk {
                id,
                plain_len: chunk.data.len() as u64,
                bytes,
            });
        }

        Ok(sealed)
    }

    /// Decrypt, decompress and verify one stored chunk.
    ///
    /// Fails if the chunk was altered, swapped for another chunk, encrypted
    /// under a different key, or does not hash to `id`.
    pub fn open_chunk(&self, id: &ChunkId, stored: &[u8]) -> Result<Vec<u8>> {
        let payload = self.open_payload(stored, &Self::chunk_aad(id))?;

        let (&tag, compressed) = payload
            .split_first()
            .ok_or_else(|| Error::Serialization("Empty chunk payload".to_string()))?;
        let algorithm = CompressionAlgorithm::from_tag(tag)?;
        let plain = Compressor::new(algorithm).decompress(compressed)?;

        if &self.chunk_id_for(&plain) != id {
            return Err(Error::Storage(format!(
                "Integrity check failed for chunk {}",
                id.0
            )));
        }

        Ok(plain)
    }

    fn open_payload(&self, stored: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        let (&format, body) = stored
            .split_first()
            .ok_or_else(|| Error::Serialization("Empty stored object".to_string()))?;

        match (format, &self.encryptor) {
            (FORMAT_ENCRYPTED, Some(enc)) => enc.decrypt(body, aad),
            (FORMAT_ENCRYPTED, None) => Err(Error::Encryption(
                "Object is encrypted but no key was provided".to_string(),
            )),
            // An encrypted repository must never accept plaintext objects.
            (FORMAT_PLAIN, Some(_)) => Err(Error::Encryption(
                "Unencrypted object found in an encrypted repository".to_string(),
            )),
            (FORMAT_PLAIN, None) => Ok(body.to_vec()),
            (other, _) => Err(Error::Serialization(format!(
                "Unknown object format {}",
                other
            ))),
        }
    }

    /// Seal a small named object such as a snapshot manifest.
    pub fn seal_blob(&self, kind: &str, id: &str, plain: &[u8]) -> Result<Vec<u8>> {
        match &self.encryptor {
            Some(enc) => {
                let mut out = vec![FORMAT_ENCRYPTED];
                out.extend_from_slice(&enc.encrypt(plain, &Self::blob_aad(kind, id))?);
                Ok(out)
            }
            None => {
                // No encryption: add a checksum so corruption is still detected.
                let mut out = vec![FORMAT_PLAIN];
                out.extend_from_slice(&hash_data(plain));
                out.extend_from_slice(plain);
                Ok(out)
            }
        }
    }

    pub fn open_blob(&self, kind: &str, id: &str, stored: &[u8]) -> Result<Vec<u8>> {
        let body = self.open_payload(stored, &Self::blob_aad(kind, id))?;
        if self.encryptor.is_some() {
            return Ok(body);
        }
        if body.len() < 32 {
            return Err(Error::Serialization("Truncated object".to_string()));
        }
        let (checksum, plain) = body.split_at(32);
        if hash_data(plain) != checksum {
            return Err(Error::Storage(format!("Integrity check failed for {}", id)));
        }
        Ok(plain.to_vec())
    }

    /// Chunk, compress and encrypt `data`, tracking IDs in the dedup index.
    pub async fn process_data(&self, data: Vec<u8>) -> Result<Vec<ChunkId>> {
        let mut chunk_ids = Vec::new();
        for sealed in self.seal_data(&data)? {
            self.dedup_store.register_chunk(sealed.id.clone());
            chunk_ids.push(sealed.id);
        }
        Ok(chunk_ids)
    }

    /// Create a snapshot summary from backed-up files.
    pub async fn create_snapshot(
        &self,
        name: String,
        source_path: String,
        file_metadatas: &[FileMetadata],
        compressed_size: u64,
    ) -> Result<Snapshot> {
        let total_size: u64 = file_metadatas.iter().map(|f| f.size).sum();
        let chunk_ids: Vec<ChunkId> = file_metadatas
            .iter()
            .flat_map(|f| f.chunk_ids.clone())
            .collect();

        Ok(Snapshot {
            id: SnapshotId::new(),
            name,
            created_at: Utc::now(),
            source_path,
            total_size,
            compressed_size,
            file_count: file_metadatas.len() as u64,
            chunk_ids,
            parent_snapshot: None,
            tags: Vec::new(),
        })
    }

    /// Get deduplication statistics
    pub fn dedup_stats(&self) -> backupforge_common::Result<crate::dedup::DedupStats> {
        Ok(self.dedup_store.stats())
    }

    /// Get the dedup store
    pub fn dedup_store(&self) -> Arc<DedupStore> {
        self.dedup_store.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encrypted_engine() -> BackupEngine {
        let mut config = BackupConfig::default();
        config.encryption_key = Some(EncryptionKey::generate());
        BackupEngine::new(config)
    }

    #[tokio::test]
    async fn test_process_data_no_encryption() {
        let engine = BackupEngine::new(BackupConfig::default());

        let data = b"Hello, World!".repeat(1000).to_vec();
        let chunk_ids = engine.process_data(data).await.unwrap();

        assert!(!chunk_ids.is_empty());
    }

    #[tokio::test]
    async fn test_process_data_with_encryption() {
        let engine = encrypted_engine();

        let data = b"Secret data!".repeat(1000).to_vec();
        let chunk_ids = engine.process_data(data).await.unwrap();

        assert!(!chunk_ids.is_empty());
    }

    #[tokio::test]
    async fn test_deduplication() {
        let engine = BackupEngine::new(BackupConfig::default());

        let data = b"Same data".repeat(1000).to_vec();

        let chunk_ids1 = engine.process_data(data.clone()).await.unwrap();
        let chunk_ids2 = engine.process_data(data).await.unwrap();

        assert_eq!(chunk_ids1, chunk_ids2);

        let stats = engine.dedup_stats().unwrap();
        assert!(stats.total_chunks > 0);
    }

    #[test]
    fn seal_open_round_trip_plain_and_encrypted() {
        let data: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        for engine in [BackupEngine::new(BackupConfig::default()), encrypted_engine()] {
            let mut restored = Vec::new();
            for sealed in engine.seal_data(&data).unwrap() {
                restored.extend(engine.open_chunk(&sealed.id, &sealed.bytes).unwrap());
            }
            assert_eq!(restored, data);
        }
    }

    #[test]
    fn empty_input_produces_no_chunks() {
        assert!(BackupEngine::new(BackupConfig::default())
            .seal_data(&[])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn encrypted_ids_depend_on_key() {
        let data = b"identical plaintext".repeat(50);
        let a = encrypted_engine().seal_data(&data).unwrap();
        let b = encrypted_engine().seal_data(&data).unwrap();
        let plain = BackupEngine::new(BackupConfig::default())
            .seal_data(&data)
            .unwrap();
        assert_ne!(a[0].id, b[0].id);
        assert_ne!(a[0].id, plain[0].id);
    }

    #[test]
    fn corruption_is_detected_in_every_byte_region() {
        for engine in [BackupEngine::new(BackupConfig::default()), encrypted_engine()] {
            let sealed = engine.seal_data(&b"important data ".repeat(200)).unwrap();
            let chunk = &sealed[0];
            for i in [0, 1, chunk.bytes.len() / 2, chunk.bytes.len() - 1] {
                let mut bad = chunk.bytes.clone();
                bad[i] ^= 0x40;
                assert!(engine.open_chunk(&chunk.id, &bad).is_err(), "byte {i}");
            }
            assert!(engine
                .open_chunk(&chunk.id, &chunk.bytes[..chunk.bytes.len() - 1])
                .is_err());
        }
    }

    #[test]
    fn chunks_cannot_be_swapped() {
        let engine = encrypted_engine();
        let a = engine.seal_data(&b"first".repeat(100)).unwrap().remove(0);
        let b = engine.seal_data(&b"second".repeat(100)).unwrap().remove(0);
        // Valid ciphertext for b presented under a's ID
        assert!(engine.open_chunk(&a.id, &b.bytes).is_err());
    }

    #[test]
    fn wrong_key_and_downgrade_are_rejected() {
        let engine = encrypted_engine();
        let sealed = engine.seal_data(&b"data".repeat(100)).unwrap().remove(0);

        assert!(encrypted_engine().open_chunk(&sealed.id, &sealed.bytes).is_err());
        assert!(BackupEngine::new(BackupConfig::default())
            .open_chunk(&sealed.id, &sealed.bytes)
            .is_err());

        let plain_engine = BackupEngine::new(BackupConfig::default());
        let plain = plain_engine.seal_data(&b"data".repeat(100)).unwrap().remove(0);
        assert!(engine.open_chunk(&plain.id, &plain.bytes).is_err());
    }

    #[test]
    fn blobs_round_trip_and_are_bound_to_name() {
        for engine in [BackupEngine::new(BackupConfig::default()), encrypted_engine()] {
            let sealed = engine.seal_blob("snapshot", "abc", b"manifest").unwrap();
            assert_eq!(engine.open_blob("snapshot", "abc", &sealed).unwrap(), b"manifest");

            if engine.is_encrypted() {
                assert!(engine.open_blob("snapshot", "other", &sealed).is_err());
            }
            let mut bad = sealed.clone();
            let last = bad.len() - 1;
            bad[last] ^= 1;
            assert!(engine.open_blob("snapshot", "abc", &bad).is_err());
        }
    }
}
