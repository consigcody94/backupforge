#![no_main]
//! Arbitrary bytes presented as a stored chunk must never panic, and must never
//! be accepted unless they hash to the claimed ID.

use backupforge_common::types::ChunkId;
use backupforge_core::{BackupConfig, BackupEngine, EncryptionKey};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let plain = BackupEngine::new(BackupConfig::default());
    let encrypted = BackupEngine::new(BackupConfig {
        encryption_key: Some(EncryptionKey::from_bytes(&[7u8; 32]).unwrap()),
        ..BackupConfig::default()
    });
    let id = ChunkId("00".repeat(32));
    for engine in [&plain, &encrypted] {
        if let Ok(out) = engine.open_chunk(&id, data) {
            // Accepting random bytes is only legitimate if they really are a
            // chunk with this ID, which for an all-zero ID essentially never holds.
            panic!("accepted fuzz input as a valid chunk ({} bytes)", out.len());
        }
    }
});
