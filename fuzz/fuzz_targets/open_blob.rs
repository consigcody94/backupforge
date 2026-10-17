#![no_main]
//! Arbitrary bytes presented as a sealed manifest must never panic.

use backupforge_core::{BackupConfig, BackupEngine, EncryptionKey};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let plain = BackupEngine::new(BackupConfig::default());
    let encrypted = BackupEngine::new(BackupConfig {
        encryption_key: Some(EncryptionKey::from_bytes(&[9u8; 32]).unwrap()),
        ..BackupConfig::default()
    });
    let _ = plain.open_blob("snapshot", "id", data);
    let _ = encrypted.open_blob("snapshot", "id", data);
});
