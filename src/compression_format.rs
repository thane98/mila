use crate::errors::CompressionError;

type Result<T> = std::result::Result<T, CompressionError>;

pub trait CompressionFormat {
    fn is_compressed_filename(&self, filename: &str) -> bool;

    fn compress(&self, bytes: &[u8]) -> Result<Vec<u8>>;

    fn decompress(&self, bytes: &[u8]) -> Result<Vec<u8>>;
}