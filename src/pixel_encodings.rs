use crate::{Endian, TextureDecodeError};

type Result<T> = std::result::Result<T, TextureDecodeError>;

#[derive(Debug, Clone, Copy)]
pub enum ColorFormat {
    RGBA8,
    RGB5A3,
    CI8,
    Unrecognized,
}

pub fn decode_rgb5a3_pixel(value: u16) -> Vec<u8> {
    if value & 0x8000 == 0 {
        let a = 0x20 * ((value >> 12) & 0x7);
        let r = 0x11 * ((value >> 8) & 0xF);
        let g = 0x11 * ((value >> 4) & 0xF);
        let b = 0x11 * (value & 0xF);
        vec![r as u8, g as u8, b as u8, a as u8]
    } else {
        let a = 0xFF;
        let r = 0x8 * ((value >> 10) & 0xFF);
        let g = 0x8 * ((value >> 5) & 0x1F);
        let b = 0x8 * (value & 0x1F);
        vec![r as u8, g as u8, b as u8, a as u8]
    }
}

// TODO: Current logic assumes we have integral bytes per pixel, not always the case.
impl ColorFormat {
    pub fn decode(&self, pixel_data: &[u8]) -> Result<Vec<u8>> {
        if let ColorFormat::Unrecognized = self {
            return Err(TextureDecodeError::UnsupportedFormat);
        }
        if self.is_indexed_format() {
            return Err(TextureDecodeError::NoPalette);
        }

        let step_size = self.bytes_per_pixel();
        if pixel_data.len() % step_size != 0 {
            return Err(TextureDecodeError::UnalignedData);
        }

        let mut decoded: Vec<u8> = Vec::new();
        for i in (0..pixel_data.len()).step_by(step_size) {
            match self {
                ColorFormat::RGBA8 => {
                    decoded.extend_from_slice(&pixel_data[i..i + 4]);
                }
                ColorFormat::RGB5A3 => {
                    let value = Endian::Big.decode_u16(&pixel_data[i..i + 2])?;
                    decoded.extend(decode_rgb5a3_pixel(value));
                }
                _ => {}
            }
        }
        Ok(decoded)
    }

    pub fn decode_indexed(&self, pixel_data: &[u8], rgba_palette: &[u8]) -> Result<Vec<u8>> {
        if let ColorFormat::Unrecognized = self {
            return Err(TextureDecodeError::UnsupportedFormat);
        }
        if !self.is_indexed_format() {
            return Err(TextureDecodeError::NotIndexed);
        }

        let step_size = self.bytes_per_pixel();
        if pixel_data.len() % step_size != 0 || rgba_palette.len() % 4 != 0 {
            return Err(TextureDecodeError::UnalignedData);
        }

        let num_colors_in_palette = rgba_palette.len() / 4;
        let mut decoded: Vec<u8> = Vec::new();
        for i in (0..pixel_data.len()).step_by(step_size) {
            let index = match self {
                ColorFormat::CI8 => pixel_data[i] as usize,
                _ => 0,
            };
            if index >= num_colors_in_palette {
                return Err(TextureDecodeError::OutOfBoundsIndex);
            }
            let real_index = index * 4;
            decoded.extend_from_slice(&rgba_palette[real_index..real_index + 4]);
        }
        Ok(decoded)
    }

    pub fn is_indexed_format(&self) -> bool {
        match self {
            ColorFormat::RGBA8 => false,
            ColorFormat::RGB5A3 => false,
            ColorFormat::CI8 => true,
            ColorFormat::Unrecognized => false,
        }
    }

    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            ColorFormat::RGBA8 => 4,
            ColorFormat::RGB5A3 => 2,
            ColorFormat::CI8 => 1,
            ColorFormat::Unrecognized => 0,
        }
    }
}
