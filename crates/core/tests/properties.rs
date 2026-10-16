//! Property tests for the sealed-object format. Random inputs, same invariants.

use backupforge_core::{BackupConfig, BackupEngine, EncryptionKey};
use proptest::prelude::*;

fn plain_engine() -> BackupEngine {
    BackupEngine::new(BackupConfig::default())
}

fn encrypted_engine() -> BackupEngine {
    BackupEngine::new(BackupConfig {
        encryption_key: Some(EncryptionKey::generate()),
        ..BackupConfig::default()
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn plain_chunks_round_trip(data in proptest::collection::vec(any::<u8>(), 0..200_000)) {
        let engine = plain_engine();
        let mut out = Vec::new();
        for sealed in engine.seal_data(&data).unwrap() {
            out.extend(engine.open_chunk(&sealed.id, &sealed.bytes).unwrap());
        }
        prop_assert_eq!(out, data);
    }

    #[test]
    fn encrypted_chunks_round_trip(data in proptest::collection::vec(any::<u8>(), 0..200_000)) {
        let engine = encrypted_engine();
        let mut out = Vec::new();
        for sealed in engine.seal_data(&data).unwrap() {
            out.extend(engine.open_chunk(&sealed.id, &sealed.bytes).unwrap());
        }
        prop_assert_eq!(out, data);
    }

    /// Any single flipped bit in an encrypted chunk must be rejected.
    #[test]
    fn encrypted_chunk_rejects_any_single_bit_flip(
        data in proptest::collection::vec(any::<u8>(), 1..20_000),
        position in any::<proptest::sample::Index>(),
        bit in 0u8..8,
    ) {
        let engine = encrypted_engine();
        let sealed = engine.seal_data(&data).unwrap().remove(0);
        let mut tampered = sealed.bytes.clone();
        let index = position.index(tampered.len());
        tampered[index] ^= 1 << bit;
        prop_assert!(engine.open_chunk(&sealed.id, &tampered).is_err());
    }

    /// A valid chunk presented under any other chunk ID must be rejected.
    #[test]
    fn encrypted_chunk_is_bound_to_its_id(
        data in proptest::collection::vec(any::<u8>(), 1..5_000),
        other in proptest::collection::vec(any::<u8>(), 1..5_000),
    ) {
        prop_assume!(data != other);
        let engine = encrypted_engine();
        let a = engine.seal_data(&data).unwrap().remove(0);
        let b = engine.seal_data(&other).unwrap().remove(0);
        prop_assume!(a.id != b.id);
        prop_assert!(engine.open_chunk(&b.id, &a.bytes).is_err());
    }

    #[test]
    fn blobs_round_trip_and_bind_their_name(
        plain in proptest::collection::vec(any::<u8>(), 0..10_000),
        name in "[a-z0-9-]{1,40}",
    ) {
        let engine = encrypted_engine();
        let sealed = engine.seal_blob("snapshot", &name, &plain).unwrap();
        prop_assert_eq!(engine.open_blob("snapshot", &name, &sealed).unwrap(), plain);
        let renamed = format!("{name}x");
        prop_assert!(engine.open_blob("snapshot", &renamed, &sealed).is_err());
    }

    /// The fuzz-style guarantee in property form: arbitrary bytes never panic and
    /// are never accepted as a chunk with the ID they claim.
    #[test]
    fn arbitrary_bytes_are_never_accepted_as_a_chunk(
        bytes in proptest::collection::vec(any::<u8>(), 0..4_000),
    ) {
        let id = backupforge_common::types::ChunkId("ab".repeat(32));
        let _ = plain_engine().open_chunk(&id, &bytes);
        let accepted = encrypted_engine().open_chunk(&id, &bytes);
        prop_assert!(accepted.is_err());
    }
}
