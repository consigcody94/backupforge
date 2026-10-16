use backupforge_common::{
    types::{BackupJob, BackupSource, BackupStats, Snapshot},
    Error, Result,
};
use backupforge_core::{BackupConfig, BackupEngine, RepoConfig, REPO_CONFIG_KEY};
use backupforge_storage::{StorageConfig, StorageManager};
use std::path::Path;
use std::sync::Arc;

use crate::filesystem::{RestoreSummary, VerifyReport};
use crate::FilesystemBackup;

/// Main backup agent that coordinates backup, restore and verification.
pub struct BackupAgent {
    engine: Arc<BackupEngine>,
    storage: Arc<StorageManager>,
    fs_backup: FilesystemBackup,
}

impl BackupAgent {
    /// Create an agent from an explicit config. The caller supplies any
    /// encryption key; no repository header is read or written. Prefer
    /// [`Self::open_repository`] for real use.
    pub async fn new(backup_config: BackupConfig, storage_config: StorageConfig) -> Result<Self> {
        let storage = Arc::new(StorageManager::from_config(storage_config).await?);
        Ok(Self::from_parts(backup_config, storage))
    }

    fn from_parts(backup_config: BackupConfig, storage: Arc<StorageManager>) -> Self {
        let engine = Arc::new(BackupEngine::new(backup_config));
        let fs_backup = FilesystemBackup::new(engine.clone(), storage.clone());
        Self {
            engine,
            storage,
            fs_backup,
        }
    }

    /// Open a repository, creating it on first use.
    ///
    /// An existing repository dictates its own encryption mode: asking for
    /// encryption on a plaintext repository is an error rather than a silent
    /// downgrade, and an encrypted repository needs the right `password`.
    pub async fn open_repository(
        mut backup_config: BackupConfig,
        storage_config: StorageConfig,
        password: Option<&str>,
        encrypt_if_new: bool,
    ) -> Result<Self> {
        let storage = Arc::new(StorageManager::from_config(storage_config).await?);

        let exists = storage
            .list_metadata(REPO_CONFIG_KEY)
            .await?
            .iter()
            .any(|k| k == REPO_CONFIG_KEY);

        let key = if exists {
            let repo = RepoConfig::from_bytes(&storage.get_metadata(REPO_CONFIG_KEY).await?)?;
            if encrypt_if_new && !repo.encrypted {
                return Err(Error::InvalidConfig(
                    "Repository already exists and is not encrypted".to_string(),
                ));
            }
            repo.unlock(password)?
        } else if encrypt_if_new {
            let password = password.ok_or_else(|| {
                Error::AuthenticationFailed(
                    "A password is required to create an encrypted repository".to_string(),
                )
            })?;
            let (repo, key) = RepoConfig::new_encrypted(password)?;
            storage
                .put_metadata(REPO_CONFIG_KEY, repo.to_bytes()?)
                .await?;
            Some(key)
        } else {
            storage
                .put_metadata(REPO_CONFIG_KEY, RepoConfig::plaintext().to_bytes()?)
                .await?;
            None
        };

        backup_config.encryption_key = key;
        Ok(Self::from_parts(backup_config, storage))
    }

    /// Execute a backup job. Only local paths are supported so far.
    pub async fn run_job(&self, job: &BackupJob) -> Result<Snapshot> {
        tracing::info!("Running backup job: {}", job.name);

        match &job.source {
            BackupSource::LocalPath { path, excludes } => {
                self.fs_backup
                    .backup_directory(Path::new(path), excludes)
                    .await
            }
            other => Err(Error::InvalidConfig(format!(
                "Backup source not supported yet: {}",
                source_name(other)
            ))),
        }
    }

    pub async fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        self.fs_backup.list_snapshots().await
    }

    /// Restore a snapshot (the newest when `snapshot_id` is `None`).
    pub async fn restore_snapshot(
        &self,
        snapshot_id: Option<&str>,
        target_path: &Path,
    ) -> Result<RestoreSummary> {
        self.fs_backup
            .restore_snapshot(snapshot_id, target_path)
            .await
    }

    pub async fn verify_snapshot(&self, snapshot_id: Option<&str>) -> Result<VerifyReport> {
        self.fs_backup.verify_snapshot(snapshot_id).await
    }

    /// Storage totals for the repository.
    pub async fn get_stats(&self) -> Result<BackupStats> {
        let storage_stats = self.storage.stats().await?;
        let snapshots = self.fs_backup.list_snapshots().await?;

        Ok(BackupStats {
            total_files: snapshots.iter().map(|s| s.file_count).sum(),
            total_bytes: storage_stats.total_bytes,
            new_chunks: 0,
            reused_chunks: 0,
            compressed_bytes: storage_stats.total_bytes,
            duration_seconds: 0,
        })
    }

    pub fn engine(&self) -> Arc<BackupEngine> {
        self.engine.clone()
    }

    pub fn storage(&self) -> Arc<StorageManager> {
        self.storage.clone()
    }
}

fn source_name(source: &BackupSource) -> &'static str {
    match source {
        BackupSource::LocalPath { .. } => "local path",
        BackupSource::RemoteSSH { .. } => "remote SSH",
        BackupSource::ProxmoxVM { .. } => "Proxmox VM",
        BackupSource::LXC { .. } => "Proxmox LXC",
        BackupSource::DockerContainer { .. } => "Docker container",
        BackupSource::DockerVolume { .. } => "Docker volume",
        BackupSource::PostgreSQL { .. } => "PostgreSQL",
        BackupSource::MySQL { .. } => "MySQL",
        BackupSource::MongoDB { .. } => "MongoDB",
        BackupSource::Redis { .. } => "Redis",
        BackupSource::AWSEC2 { .. } => "AWS EC2",
        BackupSource::AzureVM { .. } => "Azure VM",
        BackupSource::GCPVM { .. } => "GCP VM",
    }
}
