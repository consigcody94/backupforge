use backupforge_common::{
    types::{ChunkId, FileMetadata, Snapshot, SnapshotManifest},
    Error, Result,
};
use backupforge_core::BackupEngine;
use backupforge_storage::StorageManager;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use walkdir::WalkDir;

/// Files are read and chunked in segments of this size so memory use stays
/// bounded; a chunk boundary is forced at each segment boundary.
const SEGMENT_SIZE: u64 = 64 * 1024 * 1024;

/// Metadata key prefix for snapshot manifests.
pub const SNAPSHOT_KEY_PREFIX: &str = "snapshot-";

/// Result of restoring a snapshot.
#[derive(Debug, Clone, Default)]
pub struct RestoreSummary {
    pub files: u64,
    pub directories: u64,
    pub bytes: u64,
}

/// Result of verifying every chunk a snapshot references.
#[derive(Debug, Clone, Default)]
pub struct VerifyReport {
    pub chunks_checked: u64,
    pub missing: Vec<String>,
    pub corrupt: Vec<String>,
}

impl VerifyReport {
    pub fn is_ok(&self) -> bool {
        self.missing.is_empty() && self.corrupt.is_empty()
    }
}

struct StoredFile {
    meta: FileMetadata,
    stored_bytes: u64,
}

/// Filesystem backup handler
pub struct FilesystemBackup {
    engine: Arc<BackupEngine>,
    storage: Arc<StorageManager>,
}

impl FilesystemBackup {
    pub fn new(engine: Arc<BackupEngine>, storage: Arc<StorageManager>) -> Self {
        Self { engine, storage }
    }

    /// Back up a directory (or a single file) and save a snapshot manifest.
    ///
    /// Chunks are written first and the manifest last, so an interrupted run
    /// leaves unreferenced chunks but never a snapshot that points at missing
    /// data. Files that cannot be read are skipped and recorded in the
    /// snapshot's `tags` as `partial:N`; callers must surface that.
    pub async fn backup_directory(
        &self,
        source_path: &Path,
        excludes: &[String],
    ) -> Result<Snapshot> {
        let root_meta = fs::metadata(source_path).await?;
        let single_file = root_meta.is_file();

        let mut files: Vec<FileMetadata> = Vec::new();
        let mut stored_total = 0u64;
        let mut skipped = 0u64;

        for entry in WalkDir::new(source_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !self.should_exclude(e.path(), excludes))
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Skipping unreadable entry: {}", e);
                    skipped += 1;
                    continue;
                }
            };
            let path = entry.path();
            let file_type = entry.file_type();

            let rel = if single_file {
                PathBuf::from(entry.file_name())
            } else {
                match path.strip_prefix(source_path) {
                    Ok(r) if r.as_os_str().is_empty() => continue, // the root itself
                    Ok(r) => r.to_path_buf(),
                    Err(_) => continue,
                }
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");

            if file_type.is_dir() {
                files.push(FileMetadata {
                    path: rel_str,
                    size: 0,
                    modified: modified_time(&entry.metadata().ok()),
                    permissions: mode_of(&entry.metadata().ok(), 0o755),
                    is_directory: true,
                    chunk_ids: Vec::new(),
                });
            } else if file_type.is_file() {
                match self.store_file(path, rel_str).await {
                    Ok(stored) => {
                        stored_total += stored.stored_bytes;
                        files.push(stored.meta);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to back up {}: {}", path.display(), e);
                        skipped += 1;
                    }
                }
            } else {
                tracing::warn!("Skipping symlink or special file: {}", path.display());
            }
        }

        let mut snapshot = self
            .engine
            .create_snapshot(
                format!("backup-{}", Utc::now().format("%Y%m%d-%H%M%S")),
                source_path.to_string_lossy().to_string(),
                &files,
                stored_total,
            )
            .await?;
        // Directories are entries but not "files" in the summary count.
        snapshot.file_count = files.iter().filter(|f| !f.is_directory).count() as u64;
        if skipped > 0 {
            snapshot.tags.push(format!("partial:{}", skipped));
        }

        let manifest = SnapshotManifest {
            snapshot: snapshot.clone(),
            files,
        };
        let id = snapshot.id.0.to_string();
        let json = serde_json::to_vec(&manifest).map_err(|e| Error::Serialization(e.to_string()))?;
        let sealed = self.engine.seal_blob("snapshot", &id, &json)?;
        self.storage
            .put_metadata(&format!("{}{}", SNAPSHOT_KEY_PREFIX, id), sealed)
            .await?;

        Ok(snapshot)
    }

    /// Chunk one file and write any chunks storage does not already have.
    async fn store_file(&self, path: &Path, rel_path: String) -> Result<StoredFile> {
        let mut file = fs::File::open(path).await?;
        let meta = file.metadata().await?;

        let mut chunk_ids: Vec<ChunkId> = Vec::new();
        let mut total_read = 0u64;
        let mut stored_bytes = 0u64;
        let mut buf: Vec<u8> = Vec::with_capacity(meta.len().min(SEGMENT_SIZE) as usize);

        loop {
            buf.clear();
            let read = (&mut file).take(SEGMENT_SIZE).read_to_end(&mut buf).await? as u64;
            if read == 0 {
                break;
            }
            total_read += read;

            for sealed in self.engine.seal_data(&buf)? {
                stored_bytes += sealed.bytes.len() as u64;
                if !self.storage.chunk_exists(&sealed.id).await? {
                    self.storage.put_chunk(&sealed.id, sealed.bytes).await?;
                }
                chunk_ids.push(sealed.id);
            }
        }

        Ok(StoredFile {
            meta: FileMetadata {
                path: rel_path,
                size: total_read,
                modified: modified_time(&Some(meta.clone())),
                permissions: mode_of(&Some(meta), 0o644),
                is_directory: false,
                chunk_ids,
            },
            stored_bytes,
        })
    }

    /// Check if path should be excluded
    fn should_exclude(&self, path: &Path, excludes: &[String]) -> bool {
        let path_str = path.to_string_lossy();
        excludes.iter().any(|p| path_str.contains(p.as_str()))
    }

    /// List snapshots, oldest first. Manifests that fail verification are
    /// skipped with a warning rather than hiding the rest.
    pub async fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        let mut snapshots = Vec::new();
        for key in self.storage.list_metadata(SNAPSHOT_KEY_PREFIX).await? {
            match self.read_manifest(&key).await {
                Ok(m) => snapshots.push(m.snapshot),
                Err(e) => tracing::warn!("Skipping snapshot {}: {}", key, e),
            }
        }
        snapshots.sort_by_key(|s| s.created_at);
        Ok(snapshots)
    }

    async fn read_manifest(&self, key: &str) -> Result<SnapshotManifest> {
        let id = key
            .strip_prefix(SNAPSHOT_KEY_PREFIX)
            .ok_or_else(|| Error::SnapshotNotFound(key.to_string()))?;
        let stored = self.storage.get_metadata(key).await?;
        let json = self.engine.open_blob("snapshot", id, &stored)?;
        let manifest: SnapshotManifest =
            serde_json::from_slice(&json).map_err(|e| Error::Serialization(e.to_string()))?;
        if manifest.snapshot.id.0.to_string() != id {
            return Err(Error::Storage(format!(
                "Snapshot manifest {} does not match its name",
                key
            )));
        }
        Ok(manifest)
    }

    /// Load a manifest by snapshot ID, or the newest one when `id` is `None`.
    pub async fn load_manifest(&self, id: Option<&str>) -> Result<SnapshotManifest> {
        match id {
            Some(id) => self
                .read_manifest(&format!("{}{}", SNAPSHOT_KEY_PREFIX, id))
                .await
                .map_err(|e| match e {
                    Error::Io(_) | Error::Unknown(_) => Error::SnapshotNotFound(id.to_string()),
                    other => other,
                }),
            None => {
                let latest = self
                    .list_snapshots()
                    .await?
                    .pop()
                    .ok_or_else(|| Error::SnapshotNotFound("no snapshots in repository".to_string()))?;
                self.read_manifest(&format!("{}{}", SNAPSHOT_KEY_PREFIX, latest.id.0))
                    .await
            }
        }
    }

    /// Restore a snapshot into `target_path`.
    ///
    /// Manifest paths are untrusted: absolute paths, `..` components and
    /// existing symlinks inside the target are refused. Every chunk is
    /// decrypted and hash-verified before it is written.
    pub async fn restore_snapshot(
        &self,
        snapshot_id: Option<&str>,
        target_path: &Path,
    ) -> Result<RestoreSummary> {
        let manifest = self.load_manifest(snapshot_id).await?;

        fs::create_dir_all(target_path).await?;
        let target = fs::canonicalize(target_path).await?;
        let mut summary = RestoreSummary::default();

        for entry in &manifest.files {
            let rel = sanitize_relative(&entry.path)?;
            let dest = target.join(&rel);
            refuse_symlinks(&target, &rel).await?;

            if entry.is_directory {
                fs::create_dir_all(&dest).await?;
                set_mode(&dest, entry.permissions).await;
                summary.directories += 1;
                continue;
            }

            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).await?;
            }
            let mut out = fs::File::create(&dest).await?;
            let mut written = 0u64;
            for chunk_id in &entry.chunk_ids {
                let stored = self.storage.get_chunk(chunk_id).await?;
                let plain = self.engine.open_chunk(chunk_id, &stored)?;
                out.write_all(&plain).await?;
                written += plain.len() as u64;
            }
            out.sync_all().await?;
            drop(out);

            if written != entry.size {
                return Err(Error::Storage(format!(
                    "Restored size mismatch for {}: expected {} bytes, got {}",
                    entry.path, entry.size, written
                )));
            }
            set_mode(&dest, entry.permissions).await;
            summary.files += 1;
            summary.bytes += written;
        }

        Ok(summary)
    }

    /// Fetch and verify every chunk a snapshot references.
    pub async fn verify_snapshot(&self, snapshot_id: Option<&str>) -> Result<VerifyReport> {
        let manifest = self.load_manifest(snapshot_id).await?;
        let mut report = VerifyReport::default();
        let mut seen: HashSet<&str> = HashSet::new();

        for entry in &manifest.files {
            for chunk_id in &entry.chunk_ids {
                if !seen.insert(chunk_id.0.as_str()) {
                    continue;
                }
                report.chunks_checked += 1;
                match self.storage.get_chunk(chunk_id).await {
                    Err(_) => report.missing.push(chunk_id.0.clone()),
                    Ok(stored) => {
                        if self.engine.open_chunk(chunk_id, &stored).is_err() {
                            report.corrupt.push(chunk_id.0.clone());
                        }
                    }
                }
            }
        }

        Ok(report)
    }
}

/// Turn a manifest path into a safe relative path: normal components only.
fn sanitize_relative(path: &str) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(c) => out.push(c),
            Component::CurDir => {}
            _ => {
                return Err(Error::PermissionDenied(format!(
                    "Refusing unsafe path in snapshot: {:?}",
                    path
                )))
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err(Error::PermissionDenied(format!(
            "Refusing empty path in snapshot: {:?}",
            path
        )));
    }
    Ok(out)
}

/// Refuse to write through a symlink already present under the target.
async fn refuse_symlinks(target: &Path, rel: &Path) -> Result<()> {
    let mut current = target.to_path_buf();
    for component in rel.components() {
        current.push(component);
        match fs::symlink_metadata(&current).await {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(Error::PermissionDenied(format!(
                    "Refusing to restore through symlink: {}",
                    current.display()
                )))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn modified_time(meta: &Option<std::fs::Metadata>) -> DateTime<Utc> {
    meta.as_ref()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now)
}

#[cfg(unix)]
fn mode_of(meta: &Option<std::fs::Metadata>, default: u32) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    meta.as_ref()
        .map(|m| m.permissions().mode() & 0o7777)
        .unwrap_or(default)
}

#[cfg(not(unix))]
fn mode_of(_meta: &Option<std::fs::Metadata>, default: u32) -> u32 {
    default
}

/// Apply permission bits, dropping setuid/setgid/sticky on restore.
#[cfg(unix)]
async fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, std::fs::Permissions::from_mode(mode & 0o777)).await;
}

#[cfg(not(unix))]
async fn set_mode(_path: &Path, _mode: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_accepts_relative_and_rejects_traversal() {
        assert_eq!(sanitize_relative("a/b.txt").unwrap(), PathBuf::from("a/b.txt"));
        assert_eq!(sanitize_relative("./a").unwrap(), PathBuf::from("a"));
        for bad in ["../x", "a/../../x", "/etc/passwd", "", ".", ".."] {
            assert!(sanitize_relative(bad).is_err(), "{bad:?}");
        }
    }
}
