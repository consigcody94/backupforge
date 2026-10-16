use async_trait::async_trait;
use backupforge_common::{types::ChunkId, Error, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::backend::{StorageBackend, StorageStats};

/// Local filesystem storage backend
pub struct LocalStorage {
    chunks_path: PathBuf,
    metadata_path: PathBuf,
}

impl LocalStorage {
    pub async fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let chunks_path = base_path.join("chunks");
        let metadata_path = base_path.join("metadata");

        // Create directories
        fs::create_dir_all(&chunks_path).await?;
        fs::create_dir_all(&metadata_path).await?;

        Ok(Self {
            chunks_path,
            metadata_path,
        })
    }

    /// Get the path for a chunk file
    fn chunk_path(&self, chunk_id: &ChunkId) -> Result<PathBuf> {
        validate_name(&chunk_id.0, false)?;
        // Use first 2 chars for subdirectory to avoid too many files in one dir
        let prefix = &chunk_id.0[..2.min(chunk_id.0.len())];
        let subdir = self.chunks_path.join(prefix);
        Ok(subdir.join(&chunk_id.0))
    }

    /// Get the path for a metadata file
    fn metadata_path(&self, key: &str) -> Result<PathBuf> {
        validate_name(key, true)?;
        Ok(self.metadata_path.join(key))
    }
}

/// Reject names that could escape the repository directory.
///
/// Chunk IDs and metadata keys can come from untrusted manifests, so they are
/// restricted to a conservative character set and never used as raw paths.
fn validate_name(name: &str, allow_dot: bool) -> Result<()> {
    let ok = !name.is_empty()
        && name.len() <= 255
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || (allow_dot && c == '.'));
    if ok {
        Ok(())
    } else {
        Err(Error::Storage(format!("Invalid storage name: {:?}", name)))
    }
}

/// Write `data` to `path` atomically: temp file, fsync, rename.
async fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    let mut file = fs::File::create(&tmp).await?;
    file.write_all(data).await?;
    file.sync_all().await?;
    drop(file);
    if let Err(e) = fs::rename(&tmp, path).await {
        let _ = fs::remove_file(&tmp).await;
        return Err(e.into());
    }
    Ok(())
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn put_chunk(&self, chunk_id: &ChunkId, data: Vec<u8>) -> Result<()> {
        let path = self.chunk_path(chunk_id)?;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        write_atomic(&path, &data).await
    }

    async fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Vec<u8>> {
        let path = self.chunk_path(chunk_id)?;

        if !path.exists() {
            return Err(Error::ChunkNotFound(chunk_id.0.clone()));
        }

        let mut file = fs::File::open(&path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;

        Ok(data)
    }

    async fn chunk_exists(&self, chunk_id: &ChunkId) -> Result<bool> {
        let path = self.chunk_path(chunk_id)?;
        Ok(path.exists())
    }

    async fn delete_chunk(&self, chunk_id: &ChunkId) -> Result<()> {
        let path = self.chunk_path(chunk_id)?;

        if path.exists() {
            fs::remove_file(&path).await?;
        }

        Ok(())
    }

    async fn list_chunks(&self) -> Result<Vec<ChunkId>> {
        let mut chunks = Vec::new();

        let mut entries = fs::read_dir(&self.chunks_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                // Read subdirectory
                let mut subdir_entries = fs::read_dir(&path).await?;

                while let Some(subentry) = subdir_entries.next_entry().await? {
                    if let Some(filename) = subentry.file_name().to_str() {
                        if filename.contains(".tmp-") {
                            continue;
                        }
                        chunks.push(ChunkId(filename.to_string()));
                    }
                }
            } else if let Some(filename) = entry.file_name().to_str() {
                chunks.push(ChunkId(filename.to_string()));
            }
        }

        Ok(chunks)
    }

    async fn put_metadata(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let path = self.metadata_path(key)?;
        write_atomic(&path, &data).await
    }

    async fn get_metadata(&self, key: &str) -> Result<Vec<u8>> {
        let path = self.metadata_path(key)?;

        if !path.exists() {
            return Err(Error::Unknown(format!("Metadata not found: {}", key)));
        }

        let mut file = fs::File::open(&path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;

        Ok(data)
    }

    async fn list_metadata(&self, prefix: &str) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut entries = fs::read_dir(&self.metadata_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(prefix) && !name.contains(".tmp-") {
                    keys.push(name.to_string());
                }
            }
        }

        keys.sort();
        Ok(keys)
    }

    async fn stats(&self) -> Result<StorageStats> {
        let chunks = self.list_chunks().await?;
        let mut total_bytes = 0u64;

        for chunk_id in &chunks {
            let path = self.chunk_path(chunk_id)?;
            if let Ok(metadata) = fs::metadata(&path).await {
                total_bytes += metadata.len();
            }
        }

        Ok(StorageStats {
            total_chunks: chunks.len() as u64,
            total_bytes,
            available_bytes: None, // Could implement by checking filesystem
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path()).await.unwrap();

        let chunk_id = ChunkId("test123".to_string());
        let data = b"test data".to_vec();

        // Put chunk
        storage.put_chunk(&chunk_id, data.clone()).await.unwrap();

        // Check exists
        assert!(storage.chunk_exists(&chunk_id).await.unwrap());

        // Get chunk
        let retrieved = storage.get_chunk(&chunk_id).await.unwrap();
        assert_eq!(data, retrieved);

        // List chunks
        let chunks = storage.list_chunks().await.unwrap();
        assert_eq!(chunks.len(), 1);

        // Delete chunk
        storage.delete_chunk(&chunk_id).await.unwrap();
        assert!(!storage.chunk_exists(&chunk_id).await.unwrap());
    }

    #[tokio::test]
    async fn test_rejects_path_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path()).await.unwrap();

        for bad in ["../evil", "a/b", ".hidden", "", "a\\b"] {
            assert!(storage
                .put_chunk(&ChunkId(bad.to_string()), vec![1])
                .await
                .is_err());
            assert!(storage.put_metadata(bad, vec![1]).await.is_err());
        }
        assert!(!temp_dir.path().join("evil").exists());
    }

    #[tokio::test]
    async fn test_metadata_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = LocalStorage::new(temp_dir.path()).await.unwrap();

        let key = "test_metadata";
        let data = b"metadata content".to_vec();

        storage.put_metadata(key, data.clone()).await.unwrap();
        let retrieved = storage.get_metadata(key).await.unwrap();

        assert_eq!(data, retrieved);
    }
}
