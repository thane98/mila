use std::cmp::min;

use crate::CompressionError;
use crate::lz13::get_occurrence_length;

type Result<T> = std::result::Result<T, CompressionError>;

#[derive(Debug, Clone)]
pub struct LZ10CompressionFormat;

impl LZ10CompressionFormat {
    pub fn is_compressed_filename(&self, filename: &str) -> bool {
        filename.ends_with(".cms") || filename.ends_with(".cmp")
    }

    pub fn compress(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::new();
        buf.push(0x10);
        buf.push((bytes.len() & 0xFF) as u8);
        buf.push(((bytes.len() >> 8) & 0xFF) as u8);
        buf.push(((bytes.len() >> 16) & 0xFF) as u8);

        let mut out_buffer = [0; 8 * 2 + 1];
        let mut buffer_length = 1;
        let mut buffered_blocks = 0;
        let mut read_bytes = 0;
        while read_bytes < bytes.len() {
            if buffered_blocks == 8 {
                buf.extend_from_slice(&out_buffer[0..buffer_length]);
                out_buffer[0] = 0;
                buffer_length = 1;
                buffered_blocks = 0;
            }

            let old_length = min(read_bytes, 0x1000);
            let (length, disp) = get_occurrence_length(
                bytes, 
                read_bytes, 
                min(bytes.len() - read_bytes, 0x12), 
                read_bytes - old_length, 
                old_length
            );

            if length < 3 {
                out_buffer[buffer_length] = bytes[read_bytes];
                buffer_length += 1;
                read_bytes += 1;
            } else {
                read_bytes += length as usize;
                out_buffer[0] |= (1 << (7 - buffered_blocks)) as u8;
                out_buffer[buffer_length] = (((length - 3) << 4) & 0xF0) as u8;
                out_buffer[buffer_length] |= (((disp - 1) >> 8) & 0x0F) as u8;
                buffer_length += 1;
                out_buffer[buffer_length] = ((disp - 1) & 0xFF) as u8;
                buffer_length += 1;
            }
            buffered_blocks += 1;
        }
        if buffered_blocks > 0 {
            buf.extend_from_slice(&out_buffer[0..buffer_length]);
        }
        Ok(buf)
    }

    pub fn decompress(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        match nintendo_lz::decompress_arr(bytes) {
            Ok(decompressed_data) => Ok(decompressed_data),
            Err(_) => Err(CompressionError::InvalidInput("LZ10".to_string())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::load_test_file;

    #[test]
    fn lz10_round_trip_success() {
        let decompressed = load_test_file("LZ10Test.bin");
        let lz10 = LZ10CompressionFormat {};
        let compressed = lz10.compress(&decompressed);
        assert!(compressed.is_ok());
        let compressed = compressed.unwrap();
        let actual_decompressed = lz10.decompress(&compressed);
        assert!(actual_decompressed.is_ok());
        assert_eq!(decompressed, actual_decompressed.unwrap());
    }
}
