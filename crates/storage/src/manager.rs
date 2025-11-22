use async_trait::async_trait;
use backupforge_common::{types::ChunkId, Result};
use std::sync::Arc;

use crate::backend::{StorageBackend, StorageConfig, StorageStats};
use crate::{LocalStorage, S3Storage};

/// Storage manager that handles different backend types
pub struct StorageManager {
    backend: Arc<dyn StorageBackend>,
}

impl StorageManager {
    pub async fn from_config(config: StorageConfig) -> Result<Self> {
        let backend: Arc<dyn StorageBackend> = match config {
            StorageConfig::Local { path } => {
                Arc::new(LocalStorage::new(path).await?)
            }

            StorageConfig::S3 {
                bucket,
                region,
                endpoint,
                ..
            } => {
                Arc::new(S3Storage::new(bucket, region, endpoint, None)?)
            }

            StorageConfig::B2 {
                bucket,
                key_id,
                application_key,
            } => {
                // B2 is S3-compatible
                Arc::new(S3Storage::new(
                    bucket,
                    "us-west-000".to_string(),
                    Some(format!("https://s3.us-west-000.backblazeb2.com")),
                    None,
                )?)
            }
        };

        Ok(Self { backend })
    }

    pub async fn put_chunk(&self, chunk_id: &ChunkId, data: Vec<u8>) -> Result<()> {
        self.backend.put_chunk(chunk_id, data).await
    }

    pub async fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Vec<u8>> {
        self.backend.get_chunk(chunk_id).await
    }

    pub async fn chunk_exists(&self, chunk_id: &ChunkId) -> Result<bool> {
        self.backend.chunk_exists(chunk_id).await
    }

    pub async fn delete_chunk(&self, chunk_id: &ChunkId) -> Result<()> {
        self.backend.delete_chunk(chunk_id).await
    }

    pub async fn list_chunks(&self) -> Result<Vec<ChunkId>> {
        self.backend.list_chunks().await
    }

    pub async fn put_metadata(&self, key: &str, data: Vec<u8>) -> Result<()> {
        self.backend.put_metadata(key, data).await
    }

    pub async fn get_metadata(&self, key: &str) -> Result<Vec<u8>> {
        self.backend.get_metadata(key).await
    }

    pub async fn stats(&self) -> Result<StorageStats> {
        self.backend.stats().await
    }
}
