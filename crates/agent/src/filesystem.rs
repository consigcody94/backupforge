use backupforge_common::{
    types::{BackupStats, FileMetadata, Snapshot},
    Error, Result,
};
use backupforge_core::BackupEngine;
use backupforge_storage::StorageManager;
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncReadExt;
use walkdir::WalkDir;

/// Filesystem backup handler
pub struct FilesystemBackup {
    engine: Arc<BackupEngine>,
    storage: Arc<StorageManager>,
}

impl FilesystemBackup {
    pub fn new(engine: Arc<BackupEngine>, storage: Arc<StorageManager>) -> Self {
        Self { engine, storage }
    }

    /// Backup a directory recursively
    pub async fn backup_directory(
        &self,
        source_path: &Path,
        excludes: &[String],
    ) -> Result<Snapshot> {
        let mut file_metadatas = Vec::new();
        let mut total_files = 0u64;
        let mut total_bytes = 0u64;

        // Walk directory
        for entry in WalkDir::new(source_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !self.should_exclude(e.path(), excludes))
        {
            let entry = entry.map_err(|e| Error::Io(std::io::Error::other(e)))?;
            let path = entry.path();

            // Skip if not a file
            if !path.is_file() {
                continue;
            }

            match self.backup_file(path).await {
                Ok(metadata) => {
                    total_bytes += metadata.size;
                    total_files += 1;
                    file_metadatas.push(metadata);
                }
                Err(e) => {
                    tracing::warn!("Failed to backup {}: {}", path.display(), e);
                }
            }
        }

        // Create snapshot
        let snapshot = self
            .engine
            .create_snapshot(
                format!("backup-{}", Utc::now().format("%Y%m%d-%H%M%S")),
                source_path.to_string_lossy().to_string(),
                file_metadatas,
            )
            .await?;

        Ok(snapshot)
    }

    /// Backup a single file
    pub async fn backup_file(&self, path: &Path) -> Result<FileMetadata> {
        self.engine.backup_file(path).await
    }

    /// Check if path should be excluded
    fn should_exclude(&self, path: &Path, excludes: &[String]) -> bool {
        let path_str = path.to_string_lossy();

        for exclude_pattern in excludes {
            if path_str.contains(exclude_pattern) {
                return true;
            }
        }

        false
    }

    /// Restore a snapshot to a directory
    pub async fn restore_snapshot(
        &self,
        snapshot: &Snapshot,
        target_path: &Path,
    ) -> Result<()> {
        // Create target directory
        fs::create_dir_all(target_path).await?;

        // Restore each file (simplified - would need actual implementation)
        for chunk_id in &snapshot.chunk_ids {
            // Fetch chunk from storage
            let _chunk_data = self.storage.get_chunk(chunk_id).await?;

            // Decompress and decrypt would happen here via engine
            // Then write to target path
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use backupforge_core::BackupConfig;
    use backupforge_storage::StorageConfig;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_backup_directory() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source");
        let storage_path = temp_dir.path().join("storage");

        fs::create_dir_all(&source).await.unwrap();
        fs::write(source.join("file1.txt"), b"content1")
            .await
            .unwrap();
        fs::write(source.join("file2.txt"), b"content2")
            .await
            .unwrap();

        let config = BackupConfig::default();
        let engine = Arc::new(BackupEngine::new(config));

        let storage_config = StorageConfig::Local {
            path: storage_path.to_string_lossy().to_string(),
        };
        let storage = Arc::new(StorageManager::from_config(storage_config).await.unwrap());

        let fs_backup = FilesystemBackup::new(engine, storage);

        let snapshot = fs_backup.backup_directory(&source, &[]).await.unwrap();

        assert_eq!(snapshot.file_count, 2);
    }
}
