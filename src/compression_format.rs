use crate::errors::CompressionError;
use crate::LZ13CompressionFormat;

type Result<T> = std::result::Result<T, CompressionError>;

pub enum CompressionFormat {
    LZ13(LZ13CompressionFormat),
}

impl CompressionFormat {
    pub fn is_compressed_filename(&self, filename: &str) -> bool {
        match self {
            CompressionFormat::LZ13(c) => c.is_compressed_filename(filename),
        }
    }

    pub fn compress(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        match self {
            CompressionFormat::LZ13(c) => c.compress(bytes),
        }
    }

    pub fn decompress(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        match self {
            CompressionFormat::LZ13(c) => c.decompress(bytes),
        }
    }
}