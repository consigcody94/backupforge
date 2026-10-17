#![no_main]
//! The repository header is read from untrusted storage: parsing and unlocking
//! must never panic, whatever the bytes.

use backupforge_core::RepoConfig;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(cfg) = RepoConfig::from_bytes(data) {
        // Keep KDF cost bounded so the fuzzer stays fast: only try plaintext repos
        // and encrypted ones with tiny parameters.
        let cheap = cfg.kdf.as_ref().map_or(true, |k| k.memory_kib <= 64 && k.iterations <= 2);
        if cheap {
            let _ = cfg.unlock(Some("password"));
        }
    }
});
