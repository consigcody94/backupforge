use async_trait::async_trait;
use backupforge_common::{types::ChunkId, Error, Result};

use crate::backend::{StorageBackend, StorageStats};

/// S3-compatible storage backend (unfinished; rejects initialization until a maintained S3 SDK is wired up).
pub struct S3Storage {
    _bucket: String,
    _region: String,
    _prefix: String,
}

impl S3Storage {
    pub fn new(
        bucket: String,
        region: String,
        _endpoint: Option<String>,
        prefix: Option<String>,
    ) -> Result<Self> {
        let _ = (&bucket, &region, &prefix);
        Err(Error::InvalidConfig(
            "S3 and B2 storage are unfinished and disabled (no maintained S3 client is wired up yet)"
                .to_string(),
        ))
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn put_chunk(&self, _chunk_id: &ChunkId, _data: Vec<u8>) -> Result<()> {
        Err(Error::Storage("S3 storage is not implemented".to_string()))
    }

    async fn get_chunk(&self, _chunk_id: &ChunkId) -> Result<Vec<u8>> {
        Err(Error::Storage("S3 storage is not implemented".to_string()))
    }

    async fn chunk_exists(&self, _chunk_id: &ChunkId) -> Result<bool> {
        Err(Error::Storage("S3 storage is not implemented".to_string()))
    }

    async fn delete_chunk(&self, _chunk_id: &ChunkId) -> Result<()> {
        Err(Error::Storage("S3 storage is not implemented".to_string()))
    }

    async fn list_chunks(&self) -> Result<Vec<ChunkId>> {
        Err(Error::Storage("S3 storage is not implemented".to_string()))
    }

    async fn put_metadata(&self, _key: &str, _data: Vec<u8>) -> Result<()> {
        Err(Error::Storage("S3 storage is not implemented".to_string()))
    }

    async fn get_metadata(&self, _key: &str) -> Result<Vec<u8>> {
        Err(Error::Storage("S3 storage is not implemented".to_string()))
    }

    async fn list_metadata(&self, _prefix: &str) -> Result<Vec<String>> {
        Err(Error::Storage("S3 storage is not implemented".to_string()))
    }

    async fn stats(&self) -> Result<StorageStats> {
        Err(Error::Storage("S3 storage is not implemented".to_string()))
    }
}
