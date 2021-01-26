use crate::errors::CompressionError;
use nintendo_lz::decompress_arr;
use std::cmp::min;

type Result<T> = std::result::Result<T, CompressionError>;

fn get_occurrence_length(
    bytes: &[u8],
    new_ptr: usize,
    new_length: usize,
    old_ptr: usize,
    old_length: usize,
) -> (i32, usize) {
    if new_length == 0 || old_length == 0 {
        return (0, 0);
    }

    let mut disp = 0;
    let mut max_length = 0;
    for i in 0..(old_length - 1) {
        let current_old_start = old_ptr + i;
        let mut current_length = 0;
        for j in 0..new_length {
            if bytes[current_old_start + j] != bytes[new_ptr + j] {
                break;
            }
            current_length += 1;
        }
        if current_length > max_length {
            max_length = current_length;
            disp = old_length - i;
            if max_length == new_length {
                break;
            }
        }
    }
    (max_length as i32, disp)
}

pub struct LZ13CompressionFormat;

impl LZ13CompressionFormat {
    pub fn is_compressed_filename(&self, filename: &str) -> bool {
        filename.ends_with(".lz")
    }

    pub fn compress(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        // First, create the header.
        let mut result: Vec<u8> = Vec::new();
        let length = bytes.len();
        let lz13_length = bytes.len() + 1;
        result.reserve(9 + length + ((length - 1) >> 3)); // For performance, reserve space to avoid resizing.
        result.push(0x13);
        result.push((lz13_length & 0xFF) as u8);
        result.push(((lz13_length >> 8) & 0xFF) as u8);
        result.push(((lz13_length >> 16) & 0xFF) as u8);
        result.push(0x11);
        result.push((length & 0xFF) as u8);
        result.push(((length >> 8) & 0xFF) as u8);
        result.push(((length >> 16) & 0xFF) as u8);

        // Begin compressing using the DSDecmp algorithm.
        let mut out_buffer: Vec<u8> = Vec::new();
        out_buffer.reserve_exact(8 * 4 + 1);
        out_buffer.push(0);
        let mut buffered_blocks = 0;
        let mut read_bytes = 0;
        while read_bytes < bytes.len() {
            // Dump out buffer contents
            if buffered_blocks == 8 {
                result.append(&mut out_buffer);
                out_buffer.push(0);
                buffered_blocks = 0;
            }

            let old_length = min(read_bytes, 0x1000);
            let (length, disp) = get_occurrence_length(
                bytes,
                read_bytes,
                min(bytes.len() - read_bytes, 0x1000),
                read_bytes - old_length,
                old_length,
            );

            if length < 3 {
                out_buffer.push(bytes[read_bytes]);
                read_bytes += 1;
            } else {
                read_bytes += length as usize;
                out_buffer[0] |= (1 << (7 - buffered_blocks)) as u8;
                if length > 0x110 {
                    out_buffer.push(0x10 | (((length - 0x111) >> 12) & 0x0F) as u8);
                    out_buffer.push((((length - 0x111) >> 4) & 0xFF) as u8);
                    out_buffer.push((((length - 0x111) << 4) & 0xF0) as u8);
                } else if length > 0x10 {
                    out_buffer.push((((length - 0x111) >> 4) & 0x0F) as u8);
                    out_buffer.push((((length - 0x111) << 4) & 0xF0) as u8);
                } else {
                    out_buffer.push((((length - 1) << 4) & 0xF0) as u8);
                }
                let last_index = out_buffer.len() - 1;
                out_buffer[last_index] |= (((disp - 1) >> 8) & 0x0F) as u8;
                out_buffer.push(((disp - 1) & 0xFF) as u8);
            }
            buffered_blocks += 1;
        }
        if buffered_blocks > 0 {
            result.append(&mut out_buffer);
        }
        Ok(result)
    }

    pub fn decompress(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        if bytes[0] == 0 {
            let mut result: Vec<u8> = Vec::new();
            result.extend_from_slice(&bytes[4..]);
            Ok(result)
        } else {
            let truncated_input = if bytes[0] == 0x13 { &bytes[4..] } else { bytes };

            match decompress_arr(&truncated_input) {
                Ok(decompressed_data) => Ok(decompressed_data),
                Err(_) => Err(CompressionError::InvalidInput("LZ13".to_string())),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::load_test_file;
    
    #[test]
    fn lz13_decompress_success() {
        let compressed = load_test_file("LZ13Test.bin.lz");
        let decompressed = load_test_file("LZ13Test.bin");
        let lz13 = LZ13CompressionFormat{};
        let actual_decompressed = lz13.decompress(&compressed);
        assert!(actual_decompressed.is_ok());
        assert_eq!(actual_decompressed.unwrap(), decompressed);
    }

    #[test]
    fn lz13_compress_success() {
        let compressed = load_test_file("LZ13Test.bin.lz");
        let decompressed = load_test_file("LZ13Test.bin");
        let lz13 = LZ13CompressionFormat{};
        let actual_compressed = lz13.compress(&decompressed);
        assert!(actual_compressed.is_ok());
        assert_eq!(actual_compressed.unwrap(), compressed);
    }
}
