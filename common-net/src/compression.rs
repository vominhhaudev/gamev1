//! Network compression utilities for game state and messages

use std::io::{Read, Write};

#[cfg(feature = "compression")]
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
#[cfg(feature = "compression")]
use zstd::{Decoder, Encoder};
#[cfg(feature = "compression")]
use snap::{raw::{Decoder as SnapDecoder, Encoder as SnapEncoder}, read::FrameDecoder};

/// Compression algorithms available
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionAlgorithm {
    None,
    #[cfg(feature = "compression")]
    Lz4,
    #[cfg(feature = "compression")]
    Zstd,
    #[cfg(feature = "compression")]
    Snappy,
}

/// Compression level for algorithms that support it
#[derive(Debug, Clone, Copy)]
pub enum CompressionLevel {
    Fast = 1,
    Balanced = 6,
    Best = 19,
}

/// Configuration for compression
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub algorithm: CompressionAlgorithm,
    pub level: CompressionLevel,
    pub threshold: usize, // Minimum size to compress
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: CompressionAlgorithm::None,
            level: CompressionLevel::Balanced,
            threshold: 1024, // 1KB
        }
    }
}

/// Compressed data wrapper
#[derive(Debug, Clone)]
pub struct CompressedData {
    pub algorithm: CompressionAlgorithm,
    pub original_size: usize,
    pub compressed_size: usize,
    pub data: Vec<u8>,
}

impl CompressedData {
    pub fn compression_ratio(&self) -> f32 {
        if self.original_size == 0 {
            1.0
        } else {
            self.compressed_size as f32 / self.original_size as f32
        }
    }

    pub fn is_effective(&self) -> bool {
        self.compression_ratio() < 0.9 // Less than 90% of original size
    }
}

/// Compression utilities
pub struct Compression;

impl Compression {
    /// Compress data if beneficial
    pub fn compress(data: &[u8], config: &CompressionConfig) -> CompressedData {
        // Don't compress small data
        if data.len() < config.threshold {
            return CompressedData {
                algorithm: CompressionAlgorithm::None,
                original_size: data.len(),
                compressed_size: data.len(),
                data: data.to_vec(),
            };
        }

        match config.algorithm {
            CompressionAlgorithm::None => CompressedData {
                algorithm: CompressionAlgorithm::None,
                original_size: data.len(),
                compressed_size: data.len(),
                data: data.to_vec(),
            },
            #[cfg(feature = "compression")]
            CompressionAlgorithm::Lz4 => Self::compress_lz4(data, config.level),
            #[cfg(feature = "compression")]
            CompressionAlgorithm::Zstd => Self::compress_zstd(data, config.level)
                .unwrap_or_else(|_| CompressedData {
                    algorithm: CompressionAlgorithm::None,
                    original_size: data.len(),
                    compressed_size: data.len(),
                    data: data.to_vec(),
                }),
            #[cfg(feature = "compression")]
            CompressionAlgorithm::Snappy => Self::compress_snappy(data),
        }
    }

    /// Decompress data
    pub fn decompress(compressed: &CompressedData) -> Result<Vec<u8>, CompressionError> {
        match compressed.algorithm {
            CompressionAlgorithm::None => Ok(compressed.data.clone()),
            #[cfg(feature = "compression")]
            CompressionAlgorithm::Lz4 => Self::decompress_lz4(&compressed.data),
            #[cfg(feature = "compression")]
            CompressionAlgorithm::Zstd => Self::decompress_zstd(&compressed.data),
            #[cfg(feature = "compression")]
            CompressionAlgorithm::Snappy => Self::decompress_snappy(&compressed.data),
        }
    }

    #[cfg(feature = "compression")]
    fn compress_lz4(data: &[u8], level: CompressionLevel) -> CompressedData {
        let compressed = compress_prepend_size(data);
        CompressedData {
            algorithm: CompressionAlgorithm::Lz4,
            original_size: data.len(),
            compressed_size: compressed.len(),
            data: compressed,
        }
    }

    #[cfg(feature = "compression")]
    fn decompress_lz4(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        decompress_size_prepended(data)
            .map_err(|e| CompressionError::DecompressionFailed(format!("LZ4: {:?}", e)))
    }

    #[cfg(feature = "compression")]
    fn compress_zstd(data: &[u8], level: CompressionLevel) -> Result<CompressedData, CompressionError> {
        let mut encoder = Encoder::new(Vec::new(), level as i32)
            .map_err(|e| CompressionError::CompressionFailed(format!("Zstd encoder: {:?}", e)))?;

        encoder.write_all(data)
            .map_err(|e| CompressionError::CompressionFailed(format!("Zstd write: {:?}", e)))?;

        let compressed = encoder.finish()
            .map_err(|e| CompressionError::CompressionFailed(format!("Zstd finish: {:?}", e)))?;

        Ok(CompressedData {
            algorithm: CompressionAlgorithm::Zstd,
            original_size: data.len(),
            compressed_size: compressed.len(),
            data: compressed,
        })
    }

    #[cfg(feature = "compression")]
    fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        let mut decoder = Decoder::new(data)
            .map_err(|e| CompressionError::DecompressionFailed(format!("Zstd decoder: {:?}", e)))?;

        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| CompressionError::DecompressionFailed(format!("Zstd read: {:?}", e)))?;

        Ok(decompressed)
    }

    #[cfg(feature = "compression")]
    fn compress_snappy(data: &[u8]) -> CompressedData {
        let mut encoder = SnapEncoder::new();
        let compressed = encoder.compress_vec(data)
            .map_err(|e| CompressionError::CompressionFailed(format!("Snappy: {:?}", e)))
            .unwrap_or_else(|_| data.to_vec());

        CompressedData {
            algorithm: CompressionAlgorithm::Snappy,
            original_size: data.len(),
            compressed_size: compressed.len(),
            data: compressed,
        }
    }

    #[cfg(feature = "compression")]
    fn decompress_snappy(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        let mut decoder = SnapDecoder::new();
        decoder.decompress_vec(data)
            .map_err(|e| CompressionError::DecompressionFailed(format!("Snappy: {:?}", e)))
    }
}

/// Errors that can occur during compression/decompression
#[derive(Debug, thiserror::Error)]
pub enum CompressionError {
    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),

    #[error("Unsupported algorithm")]
    UnsupportedAlgorithm,

    #[error("Data corrupted")]
    DataCorrupted,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_none() {
        let data = b"Hello, World!";
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::None,
            ..Default::default()
        };

        let compressed = Compression::compress(data, &config);
        assert_eq!(compressed.algorithm, CompressionAlgorithm::None);
        assert_eq!(compressed.original_size, data.len());
        assert_eq!(compressed.compressed_size, data.len());

        let decompressed = Compression::decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_compression_lz4() {
        let data = b"This is a test message for LZ4 compression. ".repeat(10);
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Lz4,
            level: CompressionLevel::Fast,
            threshold: 0, // Force compression even for small data
        };

        let compressed = Compression::compress(data, &config);
        assert_eq!(compressed.algorithm, CompressionAlgorithm::Lz4);
        assert!(compressed.is_effective());

        let decompressed = Compression::decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_compression_zstd() {
        let data = b"This is a test message for Zstandard compression. ".repeat(5);
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Zstd,
            level: CompressionLevel::Balanced,
            threshold: 0,
        };

        let compressed = Compression::compress(data, &config);
        assert_eq!(compressed.algorithm, CompressionAlgorithm::Zstd);

        let decompressed = Compression::decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }
}
