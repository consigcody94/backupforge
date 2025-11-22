use async_trait::async_trait;
use backupforge_common::{types::ChunkId, Error, Result};
use rusoto_core::{Region, RusotoError};
use rusoto_s3::{
    DeleteObjectRequest, GetObjectRequest, HeadObjectRequest, ListObjectsV2Request,
    PutObjectRequest, S3Client, S3,
};
use std::str::FromStr;
use tokio::io::AsyncReadExt;

use crate::backend::{StorageBackend, StorageStats};

/// S3-compatible storage backend
pub struct S3Storage {
    client: S3Client,
    bucket: String,
    prefix: String,
}

impl S3Storage {
    pub fn new(
        bucket: String,
        region: String,
        endpoint: Option<String>,
        prefix: Option<String>,
    ) -> Result<Self> {
        let region = if let Some(endpoint_url) = endpoint {
            Region::Custom {
                name: region,
                endpoint: endpoint_url,
            }
        } else {
            Region::from_str(&region)
                .map_err(|e| Error::InvalidConfig(format!("Invalid region: {}", e)))?
        };

        let client = S3Client::new(region);

        Ok(Self {
            client,
            bucket,
            prefix: prefix.unwrap_or_else(|| "backupforge".to_string()),
        })
    }

    fn chunk_key(&self, chunk_id: &ChunkId) -> String {
        format!("{}/chunks/{}", self.prefix, chunk_id.0)
    }

    fn metadata_key(&self, key: &str) -> String {
        format!("{}/metadata/{}", self.prefix, key)
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn put_chunk(&self, chunk_id: &ChunkId, data: Vec<u8>) -> Result<()> {
        let key = self.chunk_key(chunk_id);

        let request = PutObjectRequest {
            bucket: self.bucket.clone(),
            key,
            body: Some(data.into()),
            ..Default::default()
        };

        self.client
            .put_object(request)
            .await
            .map_err(|e| Error::Storage(format!("S3 put failed: {}", e)))?;

        Ok(())
    }

    async fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Vec<u8>> {
        let key = self.chunk_key(chunk_id);

        let request = GetObjectRequest {
            bucket: self.bucket.clone(),
            key: key.clone(),
            ..Default::default()
        };

        let result = self
            .client
            .get_object(request)
            .await
            .map_err(|e| match e {
                RusotoError::Service(_) => Error::ChunkNotFound(chunk_id.0.clone()),
                _ => Error::Storage(format!("S3 get failed: {}", e)),
            })?;

        let mut data = Vec::new();
        if let Some(mut body) = result.body {
            body.read_to_end(&mut data)
                .await
                .map_err(|e| Error::Storage(format!("Failed to read S3 body: {}", e)))?;
        }

        Ok(data)
    }

    async fn chunk_exists(&self, chunk_id: &ChunkId) -> Result<bool> {
        let key = self.chunk_key(chunk_id);

        let request = HeadObjectRequest {
            bucket: self.bucket.clone(),
            key,
            ..Default::default()
        };

        match self.client.head_object(request).await {
            Ok(_) => Ok(true),
            Err(RusotoError::Service(_)) => Ok(false),
            Err(e) => Err(Error::Storage(format!("S3 head failed: {}", e))),
        }
    }

    async fn delete_chunk(&self, chunk_id: &ChunkId) -> Result<()> {
        let key = self.chunk_key(chunk_id);

        let request = DeleteObjectRequest {
            bucket: self.bucket.clone(),
            key,
            ..Default::default()
        };

        self.client
            .delete_object(request)
            .await
            .map_err(|e| Error::Storage(format!("S3 delete failed: {}", e)))?;

        Ok(())
    }

    async fn list_chunks(&self) -> Result<Vec<ChunkId>> {
        let prefix = format!("{}/chunks/", self.prefix);
        let mut chunks = Vec::new();
        let mut continuation_token = None;

        loop {
            let request = ListObjectsV2Request {
                bucket: self.bucket.clone(),
                prefix: Some(prefix.clone()),
                continuation_token,
                ..Default::default()
            };

            let result = self
                .client
                .list_objects_v2(request)
                .await
                .map_err(|e| Error::Storage(format!("S3 list failed: {}", e)))?;

            if let Some(contents) = result.contents {
                for object in contents {
                    if let Some(key) = object.key {
                        if let Some(chunk_id) = key.strip_prefix(&prefix) {
                            chunks.push(ChunkId(chunk_id.to_string()));
                        }
                    }
                }
            }

            if result.is_truncated == Some(true) {
                continuation_token = result.next_continuation_token;
            } else {
                break;
            }
        }

        Ok(chunks)
    }

    async fn put_metadata(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let s3_key = self.metadata_key(key);

        let request = PutObjectRequest {
            bucket: self.bucket.clone(),
            key: s3_key,
            body: Some(data.into()),
            ..Default::default()
        };

        self.client
            .put_object(request)
            .await
            .map_err(|e| Error::Storage(format!("S3 put metadata failed: {}", e)))?;

        Ok(())
    }

    async fn get_metadata(&self, key: &str) -> Result<Vec<u8>> {
        let s3_key = self.metadata_key(key);

        let request = GetObjectRequest {
            bucket: self.bucket.clone(),
            key: s3_key,
            ..Default::default()
        };

        let result = self
            .client
            .get_object(request)
            .await
            .map_err(|e| Error::Storage(format!("S3 get metadata failed: {}", e)))?;

        let mut data = Vec::new();
        if let Some(mut body) = result.body {
            body.read_to_end(&mut data)
                .await
                .map_err(|e| Error::Storage(format!("Failed to read S3 metadata: {}", e)))?;
        }

        Ok(data)
    }

    async fn stats(&self) -> Result<StorageStats> {
        let chunks = self.list_chunks().await?;

        Ok(StorageStats {
            total_chunks: chunks.len() as u64,
            total_bytes: 0, // Would need to sum object sizes
            available_bytes: None,
        })
    }
}
