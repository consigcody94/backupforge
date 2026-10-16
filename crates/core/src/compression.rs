use backupforge_common::{Result, Error};
use serde::{Deserialize, Serialize};

/// Compression algorithms supported
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    None,
    Zstd(i32),  // Compression level (1-22)
    Lz4,
}

impl CompressionAlgorithm {
    /// One-byte tag stored with each chunk so restores do not depend on config.
    pub fn tag(&self) -> u8 {
        match self {
            CompressionAlgorithm::None => 0,
            CompressionAlgorithm::Zstd(_) => 1,
            CompressionAlgorithm::Lz4 => 2,
        }
    }

    pub fn from_tag(tag: u8) -> Result<Self> {
        match tag {
            0 => Ok(CompressionAlgorithm::None),
            1 => Ok(CompressionAlgorithm::Zstd(0)),
            2 => Ok(CompressionAlgorithm::Lz4),
            other => Err(Error::Serialization(format!("Unknown compression tag {}", other))),
        }
    }
}

/// Upper bound on one decompressed chunk, to stop decompression bombs.
pub const MAX_DECOMPRESSED_CHUNK: usize = 128 * 1024 * 1024;

impl Default for CompressionAlgorithm {
    fn default() -> Self {
        Self::Zstd(3) // Level 3 is a good balance of speed/ratio
    }
}

/// Compressor for backup data
pub struct Compressor {
    algorithm: CompressionAlgorithm,
}

impl Compressor {
    pub fn new(algorithm: CompressionAlgorithm) -> Self {
        Self { algorithm }
    }

    pub fn algorithm(&self) -> CompressionAlgorithm {
        self.algorithm
    }

    /// Compress data
    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),

            CompressionAlgorithm::Zstd(level) => {
                zstd::bulk::compress(data, level)
                    .map_err(|e| Error::Unknown(format!("Zstd compression failed: {}", e)))
            }

            CompressionAlgorithm::Lz4 => {
                let mut compressed = Vec::new();
                let mut encoder = lz4::EncoderBuilder::new()
                    .build(&mut compressed)
                    .map_err(|e| Error::Unknown(format!("LZ4 encoder creation failed: {}", e)))?;

                std::io::copy(&mut std::io::Cursor::new(data), &mut encoder)
                    .map_err(|e| Error::Io(e))?;

                let (_output, result) = encoder.finish();
                result.map_err(|e| Error::Io(e))?;

                Ok(compressed)
            }
        }
    }

    /// Decompress data
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),

            CompressionAlgorithm::Zstd(_) => {
                zstd::bulk::decompress(data, MAX_DECOMPRESSED_CHUNK)
                    .map_err(|e| Error::Unknown(format!("Zstd decompression failed: {}", e)))
            }

            CompressionAlgorithm::Lz4 => {
                let mut decompressed = Vec::new();
                let mut decoder = lz4::Decoder::new(std::io::Cursor::new(data))
                    .map_err(|e| Error::Io(e))?;

                use std::io::Read;
                decoder
                    .take(MAX_DECOMPRESSED_CHUNK as u64 + 1)
                    .read_to_end(&mut decompressed)
                    .map_err(Error::Io)?;

                if decompressed.len() > MAX_DECOMPRESSED_CHUNK {
                    return Err(Error::Serialization("Decompressed chunk too large".to_string()));
                }

                Ok(decompressed)
            }
        }
    }

    /// Get compression ratio estimate (compressed_size / original_size)
    pub fn estimate_ratio(&self, original_size: usize, compressed_size: usize) -> f64 {
        if original_size == 0 {
            return 1.0;
        }
        compressed_size as f64 / original_size as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zstd_compression() {
        let compressor = Compressor::new(CompressionAlgorithm::Zstd(3));
        let original = b"Hello, World! This is a test of compression. ".repeat(100);

        let compressed = compressor.compress(&original).unwrap();
        assert!(compressed.len() < original.len());

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(original.to_vec(), decompressed);
    }

    #[test]
    fn test_lz4_compression() {
        let compressor = Compressor::new(CompressionAlgorithm::Lz4);
        let original = b"Hello, World! This is a test of compression. ".repeat(100);

        let compressed = compressor.compress(&original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(original.to_vec(), decompressed);
    }

    #[test]
    fn test_no_compression() {
        let compressor = Compressor::new(CompressionAlgorithm::None);
        let data = b"test data";

        let compressed = compressor.compress(data).unwrap();
        assert_eq!(data.to_vec(), compressed);

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_compression_ratio() {
        let compressor = Compressor::new(CompressionAlgorithm::Zstd(3));
        let ratio = compressor.estimate_ratio(1000, 500);
        assert_eq!(ratio, 0.5);
    }
}
