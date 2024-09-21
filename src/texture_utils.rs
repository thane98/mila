use crate::TextureDecodeError;

type Result<T> = std::result::Result<T, TextureDecodeError>;

// Some formats layout pixels in blocks ex. 4x4 block then next 4x4 block.
// Libraries like Pillow want sequential pixel data, so we need to rearrange the data.
pub fn block_to_sequential(
    data: &[u8],
    texture_width: usize,
    texture_height: usize,
    block_width: usize,
    block_height: usize,
) -> Result<Vec<u8>> {
    // Compute block dimensions.
    let block_size = block_width * block_height;
    let num_blocks_in_row = texture_width / block_width;
    let num_blocks_in_texture = (texture_width * texture_height) / block_size;

    // Rearrange.
    let mut sequential: Vec<u8> = vec![0; texture_width * texture_height];
    for block_number in 0..num_blocks_in_texture {
        let block_row = block_number / num_blocks_in_row;
        let block_column = block_number % num_blocks_in_row;
        for block_index in 0..block_size {
            let row_in_block = block_index / block_width;
            let column_in_block = block_index % block_width;
            let index_in_input = block_number * block_size + block_index;
            let index_in_output = block_row * texture_width * block_height
                + row_in_block * texture_width
                + block_column * block_width
                + column_in_block;
            if index_in_input < data.len() && index_in_output < sequential.len() {
                sequential[index_in_output] = data[index_in_input];
            }
        }
    }

    Ok(sequential)
}

pub fn align(value: usize, increment: usize) -> usize {
    if increment <= 1 {
        value
    } else {
        let tmp = value % increment;
        if tmp > 0 {
            value + (increment - tmp)
        } else {
            value
        }
    }
}

pub fn crop(input: &[u8], original_width: usize, width: usize, height: usize) -> Vec<u8> {
    let mut output: Vec<u8> = Vec::new();
    for r in 0..height {
        let base_index = r * original_width;
        output.extend_from_slice(&input[base_index..base_index + width]);
    }
    output
}
