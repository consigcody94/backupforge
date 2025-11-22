use backupforge_common::{hash::hash_data, types::{Chunk, ChunkId}, Result, Error};
use bytes::Bytes;

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

    /// Content-Defined Chunking using FastCDC algorithm
    fn chunk_cdc(
        &self,
        data: &[u8],
        min_size: usize,
        avg_size: usize,
        max_size: usize,
    ) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let mut offset = 0;

        // Mask for rolling hash boundary detection
        let mask = (avg_size - 1) as u64;
        let data_len = data.len();

        while offset < data_len {
            let remaining = data_len - offset;

            // If remaining data is smaller than min_size, create final chunk
            if remaining <= min_size {
                let chunk_data = data[offset..].to_vec();
                let hash = hash_data(&chunk_data);

                chunks.push(Chunk {
                    id: ChunkId::from_hash(&hash),
                    size: chunk_data.len() as u64,
                    hash,
                    data: chunk_data,
                });
                break;
            }

            // Find chunk boundary using rolling hash
            let mut chunk_end = offset + min_size;
            let search_end = std::cmp::min(offset + max_size, data_len);

            let mut hash: u64 = 0;
            for i in (offset + min_size)..search_end {
                // Simple rolling hash (Rabin fingerprint simplified)
                hash = hash.wrapping_mul(31).wrapping_add(data[i] as u64);

                // Check for chunk boundary
                if (hash & mask) == 0 {
                    chunk_end = i + 1;
                    break;
                }

                // Force boundary at max_size
                if i == search_end - 1 {
                    chunk_end = search_end;
                }
            }

            let chunk_data = data[offset..chunk_end].to_vec();
            let chunk_hash = hash_data(&chunk_data);

            chunks.push(Chunk {
                id: ChunkId::from_hash(&chunk_hash),
                size: chunk_data.len() as u64,
                hash: chunk_hash,
                data: chunk_data,
            });

            offset = chunk_end;
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
        assert!(chunks.len() >= 1);
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
}
