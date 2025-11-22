use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a chunk of data
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub String);

impl ChunkId {
    pub fn from_hash(hash: &[u8]) -> Self {
        Self(hex::encode(hash))
    }
}

/// Unique identifier for a backup snapshot
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub Uuid);

impl SnapshotId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SnapshotId {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a chunk of data in the backup system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: ChunkId,
    pub size: u64,
    pub hash: Vec<u8>,
    pub data: Vec<u8>,
}

/// Metadata for a backup snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: SnapshotId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub source_path: String,
    pub total_size: u64,
    pub compressed_size: u64,
    pub file_count: u64,
    pub chunk_ids: Vec<ChunkId>,
    pub parent_snapshot: Option<SnapshotId>,
    pub tags: Vec<String>,
}

/// File metadata in a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub permissions: u32,
    pub is_directory: bool,
    pub chunk_ids: Vec<ChunkId>,
}

/// Backup job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupJob {
    pub id: Uuid,
    pub name: String,
    pub source: BackupSource,
    pub destination: String,
    pub schedule: Option<String>, // Cron expression
    pub retention_days: u32,
    pub enabled: bool,
    pub encryption_enabled: bool,
    pub compression_level: u8,
}

/// Source of backup data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BackupSource {
    LocalPath { path: String, excludes: Vec<String> },
    RemoteSSH { host: String, port: u16, user: String, path: String },
    ProxmoxVM { node: String, vmid: String },
    LXC { node: String, ctid: String },
}

/// Backup statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStats {
    pub total_files: u64,
    pub total_bytes: u64,
    pub new_chunks: u64,
    pub reused_chunks: u64,
    pub compressed_bytes: u64,
    pub duration_seconds: u64,
}

/// Tenant information for multi-tenancy support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub quota_bytes: Option<u64>,
    pub used_bytes: u64,
}
