use backupforge_common::{
    hash::hash_data,
    types::{Chunk, ChunkId},
    Error, Result,
};

/// Chunking strategy for breaking data into chunks
#[derive(Debug, Clone)]
pub enum ChunkingStrategy {
    /// Fixed-size chunks (simple but less efficient dedup)
    Fixed { size: usize },
    /// Content-Defined Chunking (CDC) using rolling hash (better dedup)
    ContentDefined {
        min_size: usize,
        avg_size: usize,
        max_size: usize,
    },
}

impl Default for ChunkingStrategy {
    fn default() -> Self {
        Self::ContentDefined {
            min_size: 256 * 1024,      // 256 KB
            avg_size: 1024 * 1024,     // 1 MB
            max_size: 4 * 1024 * 1024, // 4 MB
        }
    }
}

/// Chunks data for storage and deduplication
pub struct Chunker {
    strategy: ChunkingStrategy,
}

impl Chunker {
    pub fn new(strategy: ChunkingStrategy) -> Self {
        Self { strategy }
    }

    /// Split data into chunks based on the chunking strategy
    pub fn chunk_data(&self, data: &[u8]) -> Result<Vec<Chunk>> {
        match &self.strategy {
            ChunkingStrategy::Fixed { size } => self.chunk_fixed(data, *size),
            ChunkingStrategy::ContentDefined {
                min_size,
                avg_size,
                max_size,
            } => self.chunk_cdc(data, *min_size, *avg_size, *max_size),
        }
    }

    /// Fixed-size chunking
    fn chunk_fixed(&self, data: &[u8], size: usize) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let end = std::cmp::min(offset + size, data.len());
            let chunk_data = data[offset..end].to_vec();
            let hash = hash_data(&chunk_data);

            chunks.push(Chunk {
                id: ChunkId::from_hash(&hash),
                size: chunk_data.len() as u64,
                hash,
                data: chunk_data,
            });

            offset = end;
        }

        Ok(chunks)
    }

    /// Content-defined chunking with FastCDC. Cut points depend on content, so an
    /// insertion only changes the chunks around it.
    fn chunk_cdc(
        &self,
        data: &[u8],
        min_size: usize,
        avg_size: usize,
        max_size: usize,
    ) -> Result<Vec<Chunk>> {
        use fastcdc::v2020::{
            FastCDC, AVERAGE_MAX, AVERAGE_MIN, MAXIMUM_MAX, MAXIMUM_MIN, MINIMUM_MAX, MINIMUM_MIN,
        };

        let fits = |v: usize, lo: u32, hi: u32| (lo as usize..=hi as usize).contains(&v);
        if !fits(min_size, MINIMUM_MIN, MINIMUM_MAX)
            || !fits(avg_size, AVERAGE_MIN, AVERAGE_MAX)
            || !fits(max_size, MAXIMUM_MIN, MAXIMUM_MAX)
            || !(min_size <= avg_size && avg_size <= max_size)
        {
            return Err(Error::InvalidConfig(format!(
                "Invalid chunk sizes: min {min_size}, avg {avg_size}, max {max_size}"
            )));
        }

        let mut chunks = Vec::new();
        for cut in FastCDC::new(data, min_size as u32, avg_size as u32, max_size as u32) {
            let slice = &data[cut.offset..cut.offset + cut.length];
            let hash = hash_data(slice);
            chunks.push(Chunk {
                id: ChunkId::from_hash(&hash),
                size: slice.len() as u64,
                hash,
                data: slice.to_vec(),
            });
        }
        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_chunking() {
        let data = vec![0u8; 10000];
        let chunker = Chunker::new(ChunkingStrategy::Fixed { size: 1000 });
        let chunks = chunker.chunk_data(&data).unwrap();

        assert_eq!(chunks.len(), 10);
        for chunk in &chunks {
            assert_eq!(chunk.size, 1000);
        }
    }

    #[test]
    fn test_cdc_chunking() {
        let data = vec![0u8; 10000];
        let chunker = Chunker::new(ChunkingStrategy::default());
        let chunks = chunker.chunk_data(&data).unwrap();

        assert!(!chunks.is_empty());
        // CDC should create at least one chunk
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_id_consistency() {
        let data = b"test data for chunking";
        let chunker = Chunker::new(ChunkingStrategy::Fixed { size: 10 });

        let chunks1 = chunker.chunk_data(data).unwrap();
        let chunks2 = chunker.chunk_data(data).unwrap();

        assert_eq!(chunks1.len(), chunks2.len());
        for (c1, c2) in chunks1.iter().zip(chunks2.iter()) {
            assert_eq!(c1.id, c2.id);
        }
    }

    #[test]
    fn chunks_always_reassemble_to_the_input() {
        let mut x: u64 = 0x1234_5678_9abc_def0;
        for len in [
            0usize, 1, 15, 16, 17, 63, 64, 65, 255, 256, 257, 5_000, 70_000,
        ] {
            let data: Vec<u8> = (0..len)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    (x >> 24) as u8
                })
                .collect();
            for strategy in [
                ChunkingStrategy::Fixed { size: 64 },
                ChunkingStrategy::ContentDefined {
                    min_size: 64,
                    avg_size: 256,
                    max_size: 1024,
                },
                ChunkingStrategy::default(),
            ] {
                let chunks = Chunker::new(strategy).chunk_data(&data).unwrap();
                let joined: Vec<u8> = chunks.iter().flat_map(|c| c.data.clone()).collect();
                assert_eq!(joined, data, "len {len}");
            }
        }
    }

    #[test]
    fn cdc_resynchronises_after_a_small_insertion() {
        let mut x: u64 = 0xdead_beef_cafe_f00d;
        let original: Vec<u8> = (0..300_000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x >> 24) as u8
            })
            .collect();
        let mut edited = vec![0xAA];
        edited.extend_from_slice(&original);

        // Small chunks so the sample has hundreds of chunks, not three.
        let chunker = Chunker::new(ChunkingStrategy::ContentDefined {
            min_size: 64,
            avg_size: 256,
            max_size: 1024,
        });
        let before: std::collections::HashSet<_> = chunker
            .chunk_data(&original)
            .unwrap()
            .into_iter()
            .map(|c| c.id)
            .collect();
        let after: Vec<_> = chunker.chunk_data(&edited).unwrap();
        let shared = after.iter().filter(|c| before.contains(&c.id)).count();
        // Fixed-size chunking would change every chunk; CDC keeps most of them.
        assert!(
            shared * 10 >= after.len() * 8,
            "{shared} of {} chunks shared",
            after.len()
        );
    }

    #[test]
    fn invalid_cdc_sizes_are_rejected() {
        let chunker = Chunker::new(ChunkingStrategy::ContentDefined {
            min_size: 16,
            avg_size: 64,
            max_size: 256,
        });
        assert!(chunker.chunk_data(&[1, 2, 3]).is_err());
    }
}
