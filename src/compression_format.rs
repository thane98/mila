use crate::errors::CompressionError;
use crate::{LZ10CompressionFormat, LZ13CompressionFormat};

type Result<T> = std::result::Result<T, CompressionError>;

#[derive(Clone)]
pub enum CompressionFormat {
    LZ10(LZ10CompressionFormat),
    LZ13(LZ13CompressionFormat),
}

impl CompressionFormat {
    pub fn is_compressed_filename(&self, filename: &str) -> bool {
        match self {
            CompressionFormat::LZ10(c) => c.is_compressed_filename(filename),
            CompressionFormat::LZ13(c) => c.is_compressed_filename(filename),
        }
    }

    pub fn compress(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        match self {
            CompressionFormat::LZ10(c) => c.compress(bytes),
            CompressionFormat::LZ13(c) => c.compress(bytes),
        }
    }

    pub fn decompress(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        match self {
            CompressionFormat::LZ10(c) => c.decompress(bytes),
            CompressionFormat::LZ13(c) => c.decompress(bytes),
        }
    }
}
