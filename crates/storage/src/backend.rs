use async_trait::async_trait;
use backupforge_common::{types::ChunkId, Error, Result};
use serde::{Deserialize, Serialize};

/// Storage backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StorageConfig {
    Local {
        path: String,
    },
    S3 {
        bucket: String,
        region: String,
        endpoint: Option<String>,
        access_key: String,
        secret_key: String,
    },
    B2 {
        bucket: String,
        key_id: String,
        application_key: String,
    },
}

/// Trait for storage backends
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a chunk
    async fn put_chunk(&self, chunk_id: &ChunkId, data: Vec<u8>) -> Result<()>;

    /// Retrieve a chunk
    async fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Vec<u8>>;

    /// Check if a chunk exists
    async fn chunk_exists(&self, chunk_id: &ChunkId) -> Result<bool>;

    /// Delete a chunk
    async fn delete_chunk(&self, chunk_id: &ChunkId) -> Result<()>;

    /// List all chunks
    async fn list_chunks(&self) -> Result<Vec<ChunkId>>;

    /// Store metadata
    async fn put_metadata(&self, key: &str, data: Vec<u8>) -> Result<()>;

    /// Retrieve metadata
    async fn get_metadata(&self, key: &str) -> Result<Vec<u8>>;

    /// Get storage statistics
    async fn stats(&self) -> Result<StorageStats>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_chunks: u64,
    pub total_bytes: u64,
    pub available_bytes: Option<u64>,
}
