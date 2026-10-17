#![no_main]
//! Chunking must never panic and chunks must reassemble to the input.

use backupforge_core::{Chunker, ChunkingStrategy};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    for strategy in [
        ChunkingStrategy::Fixed { size: 64 },
        ChunkingStrategy::ContentDefined { min_size: 64, avg_size: 256, max_size: 1024 },
    ] {
        let chunks = Chunker::new(strategy).chunk_data(data).unwrap();
        let joined: Vec<u8> = chunks.iter().flat_map(|c| c.data.clone()).collect();
        assert_eq!(joined, data);
    }
});
