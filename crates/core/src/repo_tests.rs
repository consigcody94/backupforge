use super::*;

fn fast() -> KdfParams {
    KdfParams {
        memory_kib: 8,
        iterations: 1,
        parallelism: 1,
        salt: vec![3u8; 16],
    }
}

#[test]
fn unlock_round_trip_and_wrong_password() {
    let (cfg, key) = RepoConfig::new_encrypted_with("hunter2", fast()).unwrap();
    let cfg = RepoConfig::from_bytes(&cfg.to_bytes().unwrap()).unwrap();

    let unlocked = cfg.unlock(Some("hunter2")).unwrap().unwrap();
    assert_eq!(unlocked.as_bytes(), key.as_bytes());

    assert!(cfg.unlock(Some("wrong")).is_err());
    assert!(cfg.unlock(None).is_err());
}

#[test]
fn plaintext_repo_needs_no_password() {
    assert!(RepoConfig::plaintext().unlock(None).unwrap().is_none());
}

#[test]
fn newer_format_is_refused() {
    let mut cfg = RepoConfig::plaintext();
    cfg.format_version = FORMAT_VERSION + 1;
    assert!(RepoConfig::from_bytes(&cfg.to_bytes().unwrap()).is_err());
}
