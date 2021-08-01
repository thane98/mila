use std::io::Cursor;

use crate::CompressionError;

type Result<T> = std::result::Result<T, CompressionError>;

pub struct LZ10CompressionFormat;

impl LZ10CompressionFormat {
    pub fn is_compressed_filename(&self, filename: &str) -> bool {
        filename.ends_with(".cms") || filename.ends_with(".cmp")
    }

    pub fn compress(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::new();
        match nintendo_lz::compress(bytes, &mut Cursor::new(&mut buf), nintendo_lz::CompressionLevel::LZ10) {
            Ok(_) => Ok(buf),
            Err(_) => Err(CompressionError::InvalidInput("LZ10".to_string())),
        }
    }

    pub fn decompress(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        match nintendo_lz::decompress_arr(&bytes) {
            Ok(decompressed_data) => Ok(decompressed_data),
            Err(_) => Err(CompressionError::InvalidInput("LZ10".to_string())),
        }
    }
}
