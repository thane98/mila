use std::io::Cursor;

use binread::{BinRead, BinReaderExt, FilePtr32};

use crate::{pixel_encodings::ColorFormat, texture_utils, Texture, TextureParseError};

type Result<T> = std::result::Result<T, TextureParseError>;

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u32)]
pub enum TplImageFormat {
    I4 = 0,
    I8 = 1,
    IA4 = 2,
    IA8 = 3,
    RGB565 = 4,
    RGB5A3 = 5,
    RGBA8 = 6,
    CI4 = 8,
    CI8 = 9,
    CI14X2 = 10,
    CMPR = 14,
}

#[derive(BinRead, Debug, Clone, Copy)]
#[br(repr = u32)]
pub enum TplPaletteFormat {
    IA8 = 0,
    RGB565 = 1,
    RGB5A3 = 2,
}

#[derive(BinRead)]
#[br(magic = 0x0020AF30)]
pub struct Tpl {
    pub image_count: u32,
    #[br(parse_with = FilePtr32::parse, count = image_count)]
    pub images: Vec<TplImageTableItem>,
}

#[derive(BinRead)]
pub struct TplImageTableItem {
    #[br(parse_with = FilePtr32::parse)]
    pub image: TplImage,
    #[br(parse_with = FilePtr32::parse)]
    pub palette: TplPalette,
}

#[derive(BinRead)]
pub struct TplPalette {
    pub entry_count: u16,
    pub unpacked: u8,
    pub padding: u8,
    pub format: TplPaletteFormat,
    #[br(parse_with = FilePtr32::parse, count = format.byte_size_of_palette(entry_count))]
    pub palette_data: Vec<u8>,
}

#[derive(BinRead)]
pub struct TplImage {
    pub height: u16,
    pub width: u16,
    pub format: TplImageFormat,
    #[br(parse_with = FilePtr32::parse, count = format.byte_size_of_image(height, width))]
    pub image_data: Vec<u8>,
    pub wrap_s: u32,
    pub wrap_t: u32,
    pub min_filter: u32,
    pub mag_filter: u32,
    pub lod_bias: f32,
    pub edge_lod_enable: u8,
    pub min_lod: u8,
    pub max_lod: u8,
    pub unpacked: u8,
}

impl Tpl {
    pub fn extract_textures(raw_input: &[u8]) -> Result<Vec<Texture>> {
        // First, parse the file.
        let mut cursor = Cursor::new(raw_input);
        let tpl: Tpl = cursor
            .read_be()
            .map_err(|e| TextureParseError::ParserError(format!("{:?}", e)))?;

        // Decode textures.
        let mut textures: Vec<Texture> = Vec::new();
        for image in &tpl.images {
            // TODO: Palette is optional
            // Decode the palette.
            let palette_format = ColorFormat::from(image.palette.format);
            let rgba_palette = palette_format.decode(&image.palette.palette_data)?;

            // Decode the image.
            let image_header = &image.image;
            let image_format = ColorFormat::from(image_header.format);
            let (block_width, block_height) = image_header.format.block_dimensions();
            let image_width = image_header.width as usize;
            let image_height = image_header.height as usize;
            let aligned_image_width = texture_utils::align(image_width, block_width);
            let aligned_image_height = texture_utils::align(image_height, block_height);
            let sequential_image_data = texture_utils::block_to_sequential(
                &image_header.image_data,
                aligned_image_width,
                aligned_image_height,
                block_width,
                block_height,
            )?;
            // TODO: Can we get rid of cropping by handling unaligned images in block_to_sequential?
            let cropped_image = texture_utils::crop(
                &sequential_image_data,
                aligned_image_width,
                image_width,
                image_height,
            );
            let decoded_image_data = image_format.decode_indexed(&cropped_image, &rgba_palette)?;
            textures.push(Texture {
                filename: String::new(),
                height: image_height,
                width: image_width,
                pixel_data: decoded_image_data,
            });
        }

        Ok(textures)
    }
}

impl TplImageFormat {
    pub fn byte_size_of_image(&self, height: u16, width: u16) -> usize {
        let height = height as usize;
        let width = width as usize;
        let (block_width, block_height) = self.block_dimensions();
        let base_num_bytes =
            texture_utils::align(height, block_height) * texture_utils::align(width, block_width);
        match self {
            TplImageFormat::I4 => base_num_bytes / 2,
            TplImageFormat::I8 => base_num_bytes,
            TplImageFormat::IA4 => base_num_bytes,
            TplImageFormat::IA8 => base_num_bytes * 2,
            TplImageFormat::RGB565 => base_num_bytes * 2,
            TplImageFormat::RGB5A3 => base_num_bytes * 2,
            TplImageFormat::RGBA8 => base_num_bytes * 4,
            TplImageFormat::CI4 => base_num_bytes / 2,
            TplImageFormat::CI8 => base_num_bytes,
            TplImageFormat::CI14X2 => base_num_bytes * 2,
            TplImageFormat::CMPR => base_num_bytes,
        }
    }

    pub fn block_dimensions(&self) -> (usize, usize) {
        match self {
            TplImageFormat::I4 => (8, 8),
            TplImageFormat::I8 => (8, 4),
            TplImageFormat::IA4 => (8, 4),
            TplImageFormat::IA8 => (4, 4),
            TplImageFormat::RGB565 => (4, 4),
            TplImageFormat::RGB5A3 => (4, 4),
            TplImageFormat::RGBA8 => (4, 4),
            TplImageFormat::CI4 => (8, 8),
            TplImageFormat::CI8 => (8, 4),
            TplImageFormat::CI14X2 => (4, 4),
            TplImageFormat::CMPR => (8, 8),
        }
    }
}

impl TplPaletteFormat {
    pub fn byte_size_of_palette(&self, num_entries: u16) -> usize {
        let num_entries = num_entries as usize;
        match self {
            TplPaletteFormat::IA8 => num_entries * 2,
            TplPaletteFormat::RGB565 => num_entries * 2,
            TplPaletteFormat::RGB5A3 => num_entries * 2,
        }
    }
}

impl From<TplPaletteFormat> for ColorFormat {
    fn from(format: TplPaletteFormat) -> Self {
        match format {
            TplPaletteFormat::RGB5A3 => ColorFormat::RGB5A3,
            _ => ColorFormat::Unrecognized,
        }
    }
}

impl From<TplImageFormat> for ColorFormat {
    fn from(format: TplImageFormat) -> Self {
        match format {
            TplImageFormat::RGB5A3 => ColorFormat::RGB5A3,
            TplImageFormat::RGBA8 => ColorFormat::RGBA8,
            TplImageFormat::CI8 => ColorFormat::CI8,
            _ => ColorFormat::Unrecognized,
        }
    }
}
