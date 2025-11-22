use backupforge_common::{
    types::{BackupJob, BackupSource, BackupStats, Snapshot},
    Error, Result,
};
use backupforge_core::{BackupConfig, BackupEngine};
use backupforge_storage::{StorageConfig, StorageManager};
use std::path::Path;
use std::sync::Arc;

use crate::{FilesystemBackup, ProxmoxBackup, ProxmoxConfig, SshBackup};

/// Main backup agent that coordinates all backup operations
pub struct BackupAgent {
    engine: Arc<BackupEngine>,
    storage: Arc<StorageManager>,
    fs_backup: FilesystemBackup,
    ssh_backup: SshBackup,
}

impl BackupAgent {
    pub async fn new(backup_config: BackupConfig, storage_config: StorageConfig) -> Result<Self> {
        let engine = Arc::new(BackupEngine::new(backup_config));
        let storage = Arc::new(StorageManager::from_config(storage_config).await?);

        let fs_backup = FilesystemBackup::new(engine.clone(), storage.clone());
        let ssh_backup = SshBackup::new(engine.clone(), storage.clone());

        Ok(Self {
            engine,
            storage,
            fs_backup,
            ssh_backup,
        })
    }

    /// Execute a backup job
    pub async fn run_job(&self, job: &BackupJob) -> Result<Snapshot> {
        tracing::info!("Running backup job: {}", job.name);

        match &job.source {
            BackupSource::LocalPath { path, excludes } => {
                self.fs_backup
                    .backup_directory(Path::new(path), excludes)
                    .await
            }

            BackupSource::RemoteSSH {
                host,
                port,
                user,
                path,
            } => {
                // Connect to SSH and backup
                let session = self
                    .ssh_backup
                    .connect(host, *port, user, None, None)?;

                // List remote files
                let files = self.ssh_backup.list_remote_files(&session, path)?;

                tracing::info!("Found {} files to backup", files.len());

                // For now, return an error as full implementation requires more work
                Err(Error::Unknown(
                    "SSH backup not fully implemented".to_string(),
                ))
            }

            BackupSource::ProxmoxVM { node, vmid } => {
                // Would create ProxmoxBackup and execute
                Err(Error::Unknown("Proxmox VM backup not fully implemented".to_string()))
            }

            BackupSource::LXC { node, ctid } => {
                // Would create ProxmoxBackup and execute
                Err(Error::Unknown(
                    "Proxmox LXC backup not fully implemented".to_string(),
                ))
            }
        }
    }

    /// Get backup statistics
    pub async fn get_stats(&self) -> Result<BackupStats> {
        let dedup_stats = self.engine.dedup_stats()?;
        let storage_stats = self.storage.stats().await?;

        Ok(BackupStats {
            total_files: 0, // Would track this
            total_bytes: storage_stats.total_bytes,
            new_chunks: 0,
            reused_chunks: 0,
            compressed_bytes: 0,
            duration_seconds: 0,
        })
    }

    /// Restore a snapshot
    pub async fn restore_snapshot(&self, snapshot: &Snapshot, target_path: &Path) -> Result<()> {
        self.fs_backup.restore_snapshot(snapshot, target_path).await
    }

    /// Get the backup engine
    pub fn engine(&self) -> Arc<BackupEngine> {
        self.engine.clone()
    }

    /// Get the storage manager
    pub fn storage(&self) -> Arc<StorageManager> {
        self.storage.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_agent_creation() {
        let temp_dir = TempDir::new().unwrap();

        let backup_config = BackupConfig::default();
        let storage_config = StorageConfig::Local {
            path: temp_dir.path().to_string_lossy().to_string(),
        };

        let agent = BackupAgent::new(backup_config, storage_config)
            .await
            .unwrap();

        let stats = agent.get_stats().await.unwrap();
        assert_eq!(stats.total_bytes, 0);
    }
}
