use backupforge_common::{
    types::{ChunkId, FileMetadata, Snapshot, SnapshotManifest},
    Error, Result,
};
use backupforge_core::BackupEngine;
use backupforge_storage::StorageManager;
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions};
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::io::{BufWriter, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::io::AsyncReadExt;
use walkdir::WalkDir;

/// Files are read in segments of this size so memory use stays bounded.
const SEGMENT_SIZE: u64 = 64 * 1024 * 1024;

/// Metadata key prefix for snapshot manifests.
pub const SNAPSHOT_KEY_PREFIX: &str = "snapshot-";

/// Result of restoring a snapshot.
#[derive(Debug, Clone, Default)]
pub struct RestoreSummary {
    pub files: u64,
    pub directories: u64,
    pub symlinks: u64,
    /// Links not restored because their target would leave the restore directory.
    pub skipped_links: u64,
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

/// Result of a retention pass.
#[derive(Debug, Clone, Default)]
pub struct ForgetReport {
    pub dry_run: bool,
    /// Manifest keys that are kept.
    pub kept: Vec<String>,
    /// Manifest keys that are removed (or would be).
    pub removed: Vec<String>,
    pub chunks_referenced: u64,
    /// Chunks no kept snapshot uses, and so deleted (or to be deleted).
    pub chunks_unreferenced: u64,
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
    /// data. Entries that cannot be read, and special files such as sockets,
    /// are skipped and counted in the snapshot's `tags` as `partial:N`.
    /// Callers must surface that.
    pub async fn backup_directory(
        &self,
        source_path: &Path,
        excludes: &[String],
    ) -> Result<Snapshot> {
        let root_meta = tokio::fs::symlink_metadata(source_path).await?;
        if root_meta.file_type().is_symlink() {
            return Err(Error::InvalidConfig(
                "source must not be a symbolic link; pass the link's target instead".to_string(),
            ));
        }
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
            let file_type = entry.file_type();
            let meta = entry.metadata().ok();

            if file_type.is_symlink() {
                match std::fs::read_link(path) {
                    Ok(target) => files.push(FileMetadata {
                        path: rel_str,
                        size: 0,
                        modified: modified_time(&meta),
                        permissions: 0o777,
                        is_directory: false,
                        chunk_ids: Vec::new(),
                        symlink_target: Some(target.to_string_lossy().to_string()),
                    }),
                    Err(e) => {
                        tracing::warn!("Failed to read link {}: {}", path.display(), e);
                        skipped += 1;
                    }
                }
            } else if file_type.is_dir() {
                files.push(FileMetadata {
                    path: rel_str,
                    size: 0,
                    modified: modified_time(&meta),
                    permissions: mode_of(&meta, 0o755),
                    is_directory: true,
                    chunk_ids: Vec::new(),
                    symlink_target: None,
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
                tracing::warn!("Skipping special file (not backed up): {}", path.display());
                skipped += 1;
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
        // Directories are entries but not counted as files in the summary.
        snapshot.file_count = files.iter().filter(|f| !f.is_directory).count() as u64;
        if skipped > 0 {
            snapshot.tags.push(format!("partial:{}", skipped));
        }

        let manifest = SnapshotManifest {
            snapshot: snapshot.clone(),
            files,
        };
        let id = snapshot.id.0.to_string();
        let json =
            serde_json::to_vec(&manifest).map_err(|e| Error::Serialization(e.to_string()))?;
        let sealed = self.engine.seal_blob("snapshot", &id, &json)?;
        self.storage
            .put_metadata(&format!("{}{}", SNAPSHOT_KEY_PREFIX, id), sealed)
            .await?;

        Ok(snapshot)
    }

    /// Chunk one file and write any chunks storage does not already have.
    async fn store_file(&self, path: &Path, rel_path: String) -> Result<StoredFile> {
        let mut file = tokio::fs::File::open(path).await?;
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
                symlink_target: None,
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

    /// Every manifest with its key. Fails if any one cannot be verified, so that
    /// callers which delete data never work from an incomplete picture.
    async fn load_all_manifests(&self) -> Result<Vec<(String, SnapshotManifest)>> {
        let mut out = Vec::new();
        for key in self.storage.list_metadata(SNAPSHOT_KEY_PREFIX).await? {
            let manifest = self.read_manifest(&key).await.map_err(|e| {
                Error::Storage(format!(
                    "cannot verify snapshot {key} ({e}); refusing to change the repository"
                ))
            })?;
            out.push((key, manifest));
        }
        Ok(out)
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
            Some(id) => {
                // Refuse anything that is not a plain snapshot ID before building a key.
                if uuid::Uuid::parse_str(id).is_err() {
                    return Err(Error::SnapshotNotFound(id.to_string()));
                }
                self.read_manifest(&format!("{}{}", SNAPSHOT_KEY_PREFIX, id))
                    .await
                    .map_err(|_| Error::SnapshotNotFound(id.to_string()))
            }
            None => {
                let latest = self.list_snapshots().await?.pop().ok_or_else(|| {
                    Error::SnapshotNotFound("no snapshots in repository".to_string())
                })?;
                self.read_manifest(&format!("{}{}", SNAPSHOT_KEY_PREFIX, latest.id.0))
                    .await
            }
        }
    }

    /// Restore a snapshot into `target_path`.
    ///
    /// The manifest is untrusted. Every write goes through a capability handle
    /// rooted at the target, so a symlink that is created or swapped in during
    /// the restore cannot redirect a write outside it. Unsafe symbolic links are
    /// skipped and counted. Every chunk is decrypted and hash-verified before it
    /// is written.
    pub async fn restore_snapshot(
        &self,
        snapshot_id: Option<&str>,
        target_path: &Path,
    ) -> Result<RestoreSummary> {
        let manifest = self.load_manifest(snapshot_id).await?;
        let storage = self.storage.clone();
        let engine = self.engine.clone();
        let target = target_path.to_path_buf();
        let handle = tokio::runtime::Handle::current();

        tokio::task::spawn_blocking(move || {
            restore_blocking(&handle, &storage, &engine, &manifest, &target)
        })
        .await
        .map_err(|e| Error::Unknown(format!("restore task failed: {e}")))?
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

    /// Keep the newest `keep_last` snapshots and delete the rest, then delete
    /// every chunk that no kept snapshot references.
    ///
    /// Manifests are deleted before chunks. A crash in between leaves
    /// unreferenced chunks, which a later run removes, and never a snapshot that
    /// points at missing data. Callers must hold the repository lock.
    pub async fn forget(&self, keep_last: usize, dry_run: bool) -> Result<ForgetReport> {
        if keep_last == 0 {
            return Err(Error::InvalidConfig(
                "keep_last must be at least 1".to_string(),
            ));
        }

        let mut manifests = self.load_all_manifests().await?;
        manifests.sort_by_key(|(_, m)| m.snapshot.created_at);
        let cut = manifests.len().saturating_sub(keep_last);
        let (old, kept) = manifests.split_at(cut);

        let referenced: HashSet<String> = kept
            .iter()
            .flat_map(|(_, m)| m.files.iter())
            .flat_map(|f| f.chunk_ids.iter().map(|c| c.0.clone()))
            .collect();

        let mut report = ForgetReport {
            dry_run,
            kept: kept.iter().map(|(k, _)| k.clone()).collect(),
            removed: old.iter().map(|(k, _)| k.clone()).collect(),
            chunks_referenced: referenced.len() as u64,
            chunks_unreferenced: 0,
        };

        if !dry_run {
            for (key, _) in old {
                self.storage.delete_metadata(key).await?;
            }
        }

        let unreferenced: Vec<ChunkId> = self
            .storage
            .list_chunks()
            .await?
            .into_iter()
            .filter(|c| !referenced.contains(&c.0))
            .collect();
        report.chunks_unreferenced = unreferenced.len() as u64;

        if !dry_run {
            for chunk in &unreferenced {
                self.storage.delete_chunk(chunk).await?;
            }
        }

        Ok(report)
    }
}

/// Runs on a blocking thread, so it may wait on async storage calls.
fn restore_blocking(
    handle: &tokio::runtime::Handle,
    storage: &StorageManager,
    engine: &BackupEngine,
    manifest: &SnapshotManifest,
    target: &Path,
) -> Result<RestoreSummary> {
    std::fs::create_dir_all(target)?;
    let root = Dir::open_ambient_dir(target, ambient_authority())?;
    let mut summary = RestoreSummary::default();
    let mut dirs: Vec<&FileMetadata> = Vec::new();

    for entry in &manifest.files {
        let rel = sanitize_relative(&entry.path)?;

        if entry.is_directory {
            root.create_dir_all(&rel)?;
            dirs.push(entry);
            summary.directories += 1;
        } else if let Some(link) = &entry.symlink_target {
            let depth = rel.parent().map_or(0, |p| p.components().count());
            if !link_stays_inside(link, depth) {
                tracing::warn!(
                    "Not restoring link {} -> {}: target leaves the restore directory",
                    entry.path,
                    link
                );
                summary.skipped_links += 1;
                continue;
            }
            ensure_parent(&root, &rel)?;
            clear_existing(&root, &rel)?;
            if create_symlink(&root, link, &rel)? {
                summary.symlinks += 1;
            } else {
                tracing::warn!(
                    "Not restoring link {} -> {}: symbolic links are not restored on this platform",
                    entry.path,
                    link
                );
                summary.skipped_links += 1;
            }
        } else {
            ensure_parent(&root, &rel)?;
            summary.bytes += write_file(handle, storage, engine, &root, entry, &rel)?;
            summary.files += 1;
        }
    }

    // Directory modes and times go last: writing children changes them. Deepest
    // first, so a parent is never locked read-only before its children are set.
    dirs.sort_by_key(|d| std::cmp::Reverse(d.path.matches('/').count()));
    for d in dirs {
        let rel = sanitize_relative(&d.path)?;
        let dir = root.open_dir(&rel)?;
        apply_metadata(&dir.into_std_file(), d);
    }

    Ok(summary)
}

fn write_file(
    handle: &tokio::runtime::Handle,
    storage: &StorageManager,
    engine: &BackupEngine,
    root: &Dir,
    entry: &FileMetadata,
    rel: &Path,
) -> Result<u64> {
    clear_existing(root, rel)?;

    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let file = root.open_with(rel, &options)?;

    let mut writer = BufWriter::new(file.into_std());
    let mut written = 0u64;
    for chunk_id in &entry.chunk_ids {
        let stored = handle.block_on(storage.get_chunk(chunk_id))?;
        let plain = engine.open_chunk(chunk_id, &stored)?;
        writer.write_all(&plain)?;
        written += plain.len() as u64;
    }
    if written != entry.size {
        return Err(Error::Storage(format!(
            "Restored size mismatch for {}: expected {} bytes, got {}",
            entry.path, entry.size, written
        )));
    }

    let file = writer.into_inner().map_err(|e| Error::Io(e.into_error()))?;
    file.sync_all()?;
    apply_metadata(&file, entry);
    Ok(written)
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

/// A link target is safe when it is relative and, starting from the link's own
/// directory (`link_dir_depth` components below the root), never climbs above
/// the root. Absolute targets are never restored.
fn link_stays_inside(target: &str, link_dir_depth: usize) -> bool {
    let path = Path::new(target);
    if target.is_empty() || path.has_root() {
        return false;
    }
    let mut depth = link_dir_depth as i64;
    for component in path.components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            Component::CurDir => {}
            _ => return false,
        }
    }
    true
}

/// Create a symbolic link inside `root`. Returns `false` where this platform does
/// not support it (Windows needs a different call per target type and privileges).
#[cfg(unix)]
fn create_symlink(root: &Dir, target: &str, rel: &Path) -> Result<bool> {
    root.symlink(target, rel)?;
    Ok(true)
}

#[cfg(not(unix))]
fn create_symlink(_root: &Dir, _target: &str, _rel: &Path) -> Result<bool> {
    Ok(false)
}

fn ensure_parent(root: &Dir, rel: &Path) -> Result<()> {
    match rel.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => Ok(root.create_dir_all(parent)?),
        _ => Ok(()),
    }
}

/// Remove whatever is at `rel` so it can be recreated. Never follows a link: a
/// symlink is unlinked, not read through. A directory where a file should go is
/// an error rather than something to delete.
fn clear_existing(root: &Dir, rel: &Path) -> Result<()> {
    match root.symlink_metadata(rel) {
        Ok(meta) if meta.is_dir() => Err(Error::Storage(format!(
            "cannot replace directory with a file: {}",
            rel.display()
        ))),
        Ok(_) => {
            root.remove_file(rel)?;
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Apply mode and modification time. Best effort: a failure is logged, not fatal,
/// because the content is already verified chunk by chunk.
fn apply_metadata(file: &std::fs::File, entry: &FileMetadata) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Drop setuid, setgid and sticky bits on restore.
        let mode = entry.permissions & 0o777;
        if let Err(e) = file.set_permissions(std::fs::Permissions::from_mode(mode)) {
            tracing::warn!("Could not set mode on {}: {}", entry.path, e);
        }
    }
    if let Err(e) = file.set_modified(SystemTime::from(entry.modified)) {
        tracing::warn!("Could not set time on {}: {}", entry.path, e);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_accepts_relative_and_rejects_traversal() {
        assert_eq!(
            sanitize_relative("a/b.txt").unwrap(),
            PathBuf::from("a/b.txt")
        );
        assert_eq!(sanitize_relative("./a").unwrap(), PathBuf::from("a"));
        for bad in ["../x", "a/../../x", "/etc/passwd", "", ".", ".."] {
            assert!(sanitize_relative(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn link_targets_must_stay_inside_the_root() {
        // A link at the top level (depth 0)
        assert!(link_stays_inside("file.txt", 0));
        assert!(link_stays_inside("sub/file.txt", 0));
        assert!(!link_stays_inside("../escape", 0));
        assert!(!link_stays_inside("/etc/passwd", 0));
        // A link two levels down may climb back to the top, not above it
        assert!(link_stays_inside("../../top", 2));
        assert!(!link_stays_inside("../../../above", 2));
        assert!(!link_stays_inside("", 0));
    }
}
