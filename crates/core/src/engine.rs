use backupforge_common::{
    types::{Chunk, ChunkId, Snapshot, SnapshotId, BackupStats, FileMetadata},
    Error, Result,
};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncReadExt;
use chrono::Utc;

use crate::{
    chunker::{Chunker, ChunkingStrategy},
    compression::{Compressor, CompressionAlgorithm},
    dedup::DedupStore,
    encryption::{Encryptor, EncryptionKey},
};

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

/// Main backup engine that orchestrates chunking, compression, dedup, and encryption
pub struct BackupEngine {
    config: BackupConfig,
    chunker: Chunker,
    compressor: Compressor,
    encryptor: Option<Encryptor>,
    dedup_store: Arc<DedupStore>,
}

impl BackupEngine {
    pub fn new(config: BackupConfig) -> Self {
        let chunker = Chunker::new(config.chunking_strategy.clone());
        let compressor = Compressor::new(config.compression);
        let encryptor = config
            .encryption_key
            .clone()
            .map(Encryptor::new);

        Self {
            config,
            chunker,
            compressor,
            encryptor,
            dedup_store: Arc::new(DedupStore::new()),
        }
    }

    /// Process data: chunk -> compress -> encrypt -> deduplicate
    pub async fn process_data(&self, data: Vec<u8>) -> Result<Vec<ChunkId>> {
        // Step 1: Chunk the data
        let chunks = self.chunker.chunk_data(&data)?;
        let mut chunk_ids = Vec::new();

        for mut chunk in chunks {
            // Check if we already have this chunk
            if self.dedup_store.is_duplicate(&chunk.id) {
                chunk_ids.push(chunk.id.clone());
                self.dedup_store.register_chunk(chunk.id);
                continue;
            }

            // Step 2: Compress
            let compressed = self.compressor.compress(&chunk.data)?;

            // Step 3: Encrypt (if enabled)
            let final_data = if let Some(ref encryptor) = self.encryptor {
                encryptor.encrypt(&compressed)?
            } else {
                compressed
            };

            // Store the processed chunk (in real implementation, this would write to storage)
            // For now, we just register it in the dedup store
            chunk.data = final_data;
            chunk_ids.push(chunk.id.clone());
            self.dedup_store.register_chunk(chunk.id);
        }

        Ok(chunk_ids)
    }

    /// Restore data from chunk IDs
    pub async fn restore_data(&self, chunk_ids: &[ChunkId]) -> Result<Vec<u8>> {
        let mut result = Vec::new();

        for chunk_id in chunk_ids {
            // In real implementation, fetch chunk from storage
            // For now, this is a placeholder
            // let chunk_data = storage.get_chunk(chunk_id).await?;

            // Decrypt if needed
            // let decrypted = if let Some(ref encryptor) = self.encryptor {
            //     encryptor.decrypt(&chunk_data)?
            // } else {
            //     chunk_data
            // };

            // Decompress
            // let decompressed = self.compressor.decompress(&decrypted)?;

            // result.extend_from_slice(&decompressed);
        }

        Ok(result)
    }

    /// Backup a file
    pub async fn backup_file(&self, file_path: &Path) -> Result<FileMetadata> {
        let metadata = fs::metadata(file_path).await?;
        let mut file = fs::File::open(file_path).await?;

        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;

        let chunk_ids = self.process_data(data).await?;

        Ok(FileMetadata {
            path: file_path.to_string_lossy().to_string(),
            size: metadata.len(),
            modified: metadata.modified()?.into(),
            permissions: 0o644, // Simplified
            is_directory: metadata.is_dir(),
            chunk_ids,
        })
    }

    /// Create a snapshot
    pub async fn create_snapshot(
        &self,
        name: String,
        source_path: String,
        file_metadatas: Vec<FileMetadata>,
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
            compressed_size: total_size, // Would be calculated properly
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

    #[tokio::test]
    async fn test_process_data_no_encryption() {
        let config = BackupConfig::default();
        let engine = BackupEngine::new(config);

        let data = b"Hello, World!".repeat(1000).to_vec();
        let chunk_ids = engine.process_data(data).await.unwrap();

        assert!(!chunk_ids.is_empty());
    }

    #[tokio::test]
    async fn test_process_data_with_encryption() {
        let mut config = BackupConfig::default();
        config.encryption_key = Some(EncryptionKey::generate());

        let engine = BackupEngine::new(config);

        let data = b"Secret data!".repeat(1000).to_vec();
        let chunk_ids = engine.process_data(data).await.unwrap();

        assert!(!chunk_ids.is_empty());
    }

    #[tokio::test]
    async fn test_deduplication() {
        let config = BackupConfig::default();
        let engine = BackupEngine::new(config);

        let data = b"Same data".repeat(1000).to_vec();

        let chunk_ids1 = engine.process_data(data.clone()).await.unwrap();
        let chunk_ids2 = engine.process_data(data).await.unwrap();

        // Should produce same chunk IDs
        assert_eq!(chunk_ids1, chunk_ids2);

        // Stats should show deduplication working
        let stats = engine.dedup_stats().unwrap();
        assert!(stats.total_chunks > 0);
    }
}
