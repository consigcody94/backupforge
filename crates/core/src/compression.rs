use backupforge_common::{Result, Error};
use serde::{Deserialize, Serialize};

/// Compression algorithms supported
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    None,
    Zstd(i32),  // Compression level (1-22)
    Lz4,
}

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
                zstd::bulk::decompress(data, 128 * 1024 * 1024) // Max 128MB decompressed
                    .map_err(|e| Error::Unknown(format!("Zstd decompression failed: {}", e)))
            }

            CompressionAlgorithm::Lz4 => {
                let mut decompressed = Vec::new();
                let mut decoder = lz4::Decoder::new(std::io::Cursor::new(data))
                    .map_err(|e| Error::Io(e))?;

                std::io::copy(&mut decoder, &mut decompressed)
                    .map_err(|e| Error::Io(e))?;

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
