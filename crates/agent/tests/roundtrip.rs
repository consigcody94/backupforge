//! End-to-end tests: real files in, real repository on disk, real restore.

use backupforge_agent::BackupAgent;
use backupforge_common::types::{
    BackupJob, BackupSource, FileMetadata, Snapshot, SnapshotId, SnapshotManifest,
};
use backupforge_core::BackupConfig;
use backupforge_storage::StorageConfig;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use uuid::Uuid;

const PASSWORD: &str = "correct horse battery staple";
const SECRET_MARKER: &str = "TOP-SECRET-MARKER-9f3c1a";

fn storage_cfg(path: &Path) -> StorageConfig {
    StorageConfig::Local {
        path: path.to_string_lossy().to_string(),
    }
}

fn job(source: &Path) -> BackupJob {
    BackupJob {
        id: Uuid::new_v4(),
        name: "test".to_string(),
        source: BackupSource::LocalPath {
            path: source.to_string_lossy().to_string(),
            excludes: vec![],
        },
        destination: String::new(),
        schedule: None,
        retention_days: 30,
        enabled: true,
        encryption_enabled: false,
        compression_level: 3,
    }
}

/// Deterministic pseudo-random bytes (incompressible enough to span chunks).
fn noise(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

fn make_source(root: &Path) {
    fs::create_dir_all(root.join("docs/nested/deeper")).unwrap();
    fs::create_dir_all(root.join("empty-dir")).unwrap();
    fs::write(root.join("small.txt"), b"hello world").unwrap();
    fs::write(root.join("empty.bin"), b"").unwrap();
    fs::write(root.join("docs/secret.txt"), SECRET_MARKER.repeat(500)).unwrap();
    fs::write(
        root.join("docs/nested/deeper/big.bin"),
        noise(3 * 1024 * 1024 + 123, 1),
    )
    .unwrap();
    fs::write(root.join("unicode-é-名前.txt"), "unicode name").unwrap();
}

/// Map of relative path -> contents (None for directories).
fn tree(root: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    for entry in walkdir_sorted(root) {
        let rel = entry
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .to_string();
        if rel.is_empty() {
            continue;
        }
        if entry.is_dir() {
            out.push((rel, None));
        } else {
            out.push((rel, Some(fs::read(&entry).unwrap())));
        }
    }
    out
}

fn walkdir_sorted(root: &Path) -> Vec<PathBuf> {
    let mut all = vec![root.to_path_buf()];
    let mut i = 0;
    while i < all.len() {
        if all[i].is_dir() {
            let mut kids: Vec<PathBuf> = fs::read_dir(&all[i])
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            kids.sort();
            all.extend(kids);
        }
        i += 1;
    }
    all.sort();
    all
}

fn all_files(root: &Path) -> Vec<PathBuf> {
    walkdir_sorted(root)
        .into_iter()
        .filter(|p| p.is_file())
        .collect()
}

async fn backup(source: &Path, repo: &Path, password: Option<&str>, encrypt: bool) -> Snapshot {
    let agent = BackupAgent::open_repository(
        BackupConfig::default(),
        storage_cfg(repo),
        password,
        encrypt,
    )
    .await
    .unwrap();
    agent.run_job(&job(source)).await.unwrap()
}

async fn open(repo: &Path, password: Option<&str>) -> backupforge_common::Result<BackupAgent> {
    BackupAgent::open_repository(BackupConfig::default(), storage_cfg(repo), password, false).await
}

fn chunk_files(repo: &Path) -> Vec<PathBuf> {
    all_files(&repo.join("chunks"))
}

#[tokio::test]
async fn plaintext_round_trip_restores_identical_tree() {
    let tmp = TempDir::new().unwrap();
    let (src, repo, out) = (
        tmp.path().join("src"),
        tmp.path().join("repo"),
        tmp.path().join("out"),
    );
    make_source(&src);

    let snap = backup(&src, &repo, None, false).await;
    assert_eq!(snap.file_count, 5);
    assert!(snap.tags.is_empty());

    let agent = open(&repo, None).await.unwrap();
    let summary = agent.restore_snapshot(None, &out).await.unwrap();
    assert_eq!(summary.files, 5);
    assert_eq!(tree(&src), tree(&out));
    assert!(agent.verify_snapshot(None).await.unwrap().is_ok());
}

#[tokio::test]
async fn encrypted_round_trip_and_nothing_leaks_to_disk() {
    let tmp = TempDir::new().unwrap();
    let (src, repo, out) = (
        tmp.path().join("src"),
        tmp.path().join("repo"),
        tmp.path().join("out"),
    );
    make_source(&src);

    backup(&src, &repo, Some(PASSWORD), true).await;

    // Neither contents nor file names may appear anywhere in the repository,
    // except the public header which holds only KDF settings.
    for file in all_files(&repo) {
        let bytes = fs::read(&file).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains(SECRET_MARKER),
            "content leaked in {}",
            file.display()
        );
        assert!(
            !text.contains("secret.txt"),
            "file name leaked in {}",
            file.display()
        );
        assert!(
            !text.contains("big.bin"),
            "file name leaked in {}",
            file.display()
        );
    }

    let agent = open(&repo, Some(PASSWORD)).await.unwrap();
    agent.restore_snapshot(None, &out).await.unwrap();
    assert_eq!(tree(&src), tree(&out));
}

#[tokio::test]
async fn wrong_or_missing_password_is_refused() {
    let tmp = TempDir::new().unwrap();
    let (src, repo) = (tmp.path().join("src"), tmp.path().join("repo"));
    make_source(&src);
    backup(&src, &repo, Some(PASSWORD), true).await;

    assert!(open(&repo, Some("wrong password")).await.is_err());
    assert!(open(&repo, None).await.is_err());
}

#[tokio::test]
async fn existing_plaintext_repo_is_not_silently_reused_as_encrypted() {
    let tmp = TempDir::new().unwrap();
    let (src, repo) = (tmp.path().join("src"), tmp.path().join("repo"));
    make_source(&src);
    backup(&src, &repo, None, false).await;

    let result = BackupAgent::open_repository(
        BackupConfig::default(),
        storage_cfg(&repo),
        Some(PASSWORD),
        true,
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn identical_backup_adds_no_new_chunks() {
    let tmp = TempDir::new().unwrap();
    let (src, repo) = (tmp.path().join("src"), tmp.path().join("repo"));
    make_source(&src);

    backup(&src, &repo, None, false).await;
    let before = chunk_files(&repo).len();
    assert!(before > 3);
    backup(&src, &repo, None, false).await;
    assert_eq!(chunk_files(&repo).len(), before);

    let agent = open(&repo, None).await.unwrap();
    assert_eq!(agent.list_snapshots().await.unwrap().len(), 2);
}

async fn corruption_case(encrypted: bool) {
    let tmp = TempDir::new().unwrap();
    let (src, repo, out) = (
        tmp.path().join("src"),
        tmp.path().join("repo"),
        tmp.path().join("out"),
    );
    make_source(&src);
    let pw = if encrypted { Some(PASSWORD) } else { None };
    backup(&src, &repo, pw, encrypted).await;

    let chunks = chunk_files(&repo);
    let agent = open(&repo, pw).await.unwrap();
    assert!(agent.verify_snapshot(None).await.unwrap().is_ok());

    // Flip one bit in one chunk
    let victim = &chunks[0];
    let mut bytes = fs::read(victim).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0x01;
    fs::write(victim, &bytes).unwrap();

    let report = agent.verify_snapshot(None).await.unwrap();
    assert_eq!(report.corrupt.len(), 1, "{report:?}");
    assert!(agent.restore_snapshot(None, &out).await.is_err());

    // Truncate it
    fs::write(victim, &bytes[..bytes.len() / 2]).unwrap();
    assert!(!agent.verify_snapshot(None).await.unwrap().is_ok());

    // Delete it
    fs::remove_file(victim).unwrap();
    let report = agent.verify_snapshot(None).await.unwrap();
    assert_eq!(report.missing.len(), 1, "{report:?}");
}

#[tokio::test]
async fn corruption_truncation_and_loss_are_detected_plaintext() {
    corruption_case(false).await;
}

#[tokio::test]
async fn corruption_truncation_and_loss_are_detected_encrypted() {
    corruption_case(true).await;
}

#[tokio::test]
async fn swapping_two_valid_chunks_is_detected() {
    for encrypted in [false, true] {
        let tmp = TempDir::new().unwrap();
        let (src, repo) = (tmp.path().join("src"), tmp.path().join("repo"));
        make_source(&src);
        let pw = if encrypted { Some(PASSWORD) } else { None };
        backup(&src, &repo, pw, encrypted).await;

        let chunks = chunk_files(&repo);
        let (a, b) = (&chunks[0], &chunks[1]);
        let (da, db) = (fs::read(a).unwrap(), fs::read(b).unwrap());
        fs::write(a, db).unwrap();
        fs::write(b, da).unwrap();

        let agent = open(&repo, pw).await.unwrap();
        let report = agent.verify_snapshot(None).await.unwrap();
        assert_eq!(report.corrupt.len(), 2, "encrypted={encrypted} {report:?}");
    }
}

#[tokio::test]
async fn tampered_manifest_is_rejected() {
    for encrypted in [false, true] {
        let tmp = TempDir::new().unwrap();
        let (src, repo) = (tmp.path().join("src"), tmp.path().join("repo"));
        make_source(&src);
        let pw = if encrypted { Some(PASSWORD) } else { None };
        backup(&src, &repo, pw, encrypted).await;

        let manifest = all_files(&repo.join("metadata"))
            .into_iter()
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("snapshot-")
            })
            .unwrap();
        let mut bytes = fs::read(&manifest).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        fs::write(&manifest, bytes).unwrap();

        let agent = open(&repo, pw).await.unwrap();
        assert!(
            agent.list_snapshots().await.unwrap().is_empty(),
            "encrypted={encrypted}"
        );
        assert!(agent
            .restore_snapshot(None, &tmp.path().join("out"))
            .await
            .is_err());
    }
}

#[tokio::test]
async fn manifest_renamed_to_another_snapshot_id_is_rejected() {
    let tmp = TempDir::new().unwrap();
    let (src, repo) = (tmp.path().join("src"), tmp.path().join("repo"));
    make_source(&src);
    let snap = backup(&src, &repo, None, false).await;

    let meta = repo.join("metadata");
    let original = meta.join(format!("snapshot-{}", snap.id.0));
    let forged = meta.join(format!("snapshot-{}", Uuid::new_v4()));
    fs::rename(&original, &forged).unwrap();

    let agent = open(&repo, None).await.unwrap();
    assert!(agent.list_snapshots().await.unwrap().is_empty());
}

/// Seal and store a hostile manifest the way a compromised storage side could.
async fn plant_manifest(agent: &BackupAgent, files: Vec<FileMetadata>) -> String {
    let snapshot = Snapshot {
        id: SnapshotId::new(),
        name: "planted".to_string(),
        created_at: chrono::Utc::now(),
        source_path: "/".to_string(),
        total_size: 0,
        compressed_size: 0,
        file_count: files.len() as u64,
        chunk_ids: vec![],
        parent_snapshot: None,
        tags: vec![],
    };
    let id = snapshot.id.0.to_string();
    let manifest = SnapshotManifest { snapshot, files };
    let sealed = agent
        .engine()
        .seal_blob("snapshot", &id, &serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    agent
        .storage()
        .put_metadata(&format!("snapshot-{}", id), sealed)
        .await
        .unwrap();
    id
}

fn entry(path: &str, is_directory: bool) -> FileMetadata {
    FileMetadata {
        path: path.to_string(),
        size: 0,
        modified: chrono::Utc::now(),
        permissions: 0o644,
        is_directory,
        chunk_ids: vec![],
    }
}

#[tokio::test]
async fn hostile_manifest_paths_cannot_escape_the_target() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    let agent = open_new(&repo).await;

    for (i, bad) in [
        "../escape.txt",
        "a/../../escape.txt",
        "/tmp/backupforge-abs-escape.txt",
    ]
    .iter()
    .enumerate()
    {
        let id = plant_manifest(&agent, vec![entry(bad, false)]).await;
        let target = tmp.path().join(format!("target{i}"));
        let result = agent.restore_snapshot(Some(&id), &target).await;
        assert!(result.is_err(), "path {bad:?} was accepted");
    }
    assert!(!tmp.path().join("escape.txt").exists());
    assert!(!Path::new("/tmp/backupforge-abs-escape.txt").exists());
}

async fn open_new(repo: &Path) -> BackupAgent {
    BackupAgent::open_repository(BackupConfig::default(), storage_cfg(repo), None, false)
        .await
        .unwrap()
}

#[cfg(unix)]
#[tokio::test]
async fn restore_refuses_to_write_through_a_symlink() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    let outside = tmp.path().join("outside");
    let target = tmp.path().join("target");
    fs::create_dir_all(&outside).unwrap();
    fs::create_dir_all(&target).unwrap();
    std::os::unix::fs::symlink(&outside, target.join("link")).unwrap();

    let agent = open_new(&repo).await;
    let id = plant_manifest(&agent, vec![entry("link/pwned.txt", false)]).await;

    assert!(agent.restore_snapshot(Some(&id), &target).await.is_err());
    assert!(!outside.join("pwned.txt").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn unreadable_files_mark_the_snapshot_partial() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = TempDir::new().unwrap();
    let (src, repo) = (tmp.path().join("src"), tmp.path().join("repo"));
    make_source(&src);
    let locked = src.join("locked.txt");
    fs::write(&locked, b"cannot read me").unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    // Root can read anything; skip rather than assert something false.
    if fs::read(&locked).is_ok() {
        return;
    }

    let snap = backup(&src, &repo, None, false).await;
    assert!(
        snap.tags.iter().any(|t| t == "partial:1"),
        "{:?}",
        snap.tags
    );
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn symlinks_in_the_source_are_not_followed() {
    let tmp = TempDir::new().unwrap();
    let (src, repo, out) = (
        tmp.path().join("src"),
        tmp.path().join("repo"),
        tmp.path().join("out"),
    );
    let outside = tmp.path().join("outside.txt");
    fs::create_dir_all(&src).unwrap();
    fs::write(&outside, SECRET_MARKER).unwrap();
    fs::write(src.join("real.txt"), b"real").unwrap();
    std::os::unix::fs::symlink(&outside, src.join("link.txt")).unwrap();

    backup(&src, &repo, None, false).await;
    let agent = open(&repo, None).await.unwrap();
    agent.restore_snapshot(None, &out).await.unwrap();

    assert!(out.join("real.txt").exists());
    assert!(!out.join("link.txt").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn file_permissions_are_restored_without_setuid() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = TempDir::new().unwrap();
    let (src, repo, out) = (
        tmp.path().join("src"),
        tmp.path().join("repo"),
        tmp.path().join("out"),
    );
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("run.sh"), b"#!/bin/sh\n").unwrap();
    fs::set_permissions(src.join("run.sh"), fs::Permissions::from_mode(0o4755)).unwrap();

    backup(&src, &repo, None, false).await;
    open(&repo, None)
        .await
        .unwrap()
        .restore_snapshot(None, &out)
        .await
        .unwrap();

    let mode = fs::metadata(out.join("run.sh"))
        .unwrap()
        .permissions()
        .mode()
        & 0o7777;
    assert_eq!(mode, 0o755);
}

#[tokio::test]
async fn restoring_a_missing_snapshot_fails_cleanly() {
    let tmp = TempDir::new().unwrap();
    let agent = open_new(&tmp.path().join("repo")).await;
    assert!(agent
        .restore_snapshot(None, &tmp.path().join("out"))
        .await
        .is_err());
    assert!(agent
        .restore_snapshot(Some(&Uuid::new_v4().to_string()), &tmp.path().join("out"))
        .await
        .is_err());
    assert!(agent
        .restore_snapshot(Some("../../etc/passwd"), &tmp.path().join("out"))
        .await
        .is_err());
}

#[tokio::test]
async fn unsupported_sources_error_instead_of_pretending() {
    let tmp = TempDir::new().unwrap();
    let agent = open_new(&tmp.path().join("repo")).await;
    let mut j = job(tmp.path());
    j.source = BackupSource::Redis {
        host: "localhost".into(),
        port: 6379,
    };
    assert!(agent.run_job(&j).await.is_err());
}
