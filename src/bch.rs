use crate::texture::Texture;
use crate::texture_decoder;
use crate::TextureParseError;
use byteorder::{LittleEndian, ReadBytesExt};
use encoding_rs::UTF_8;
use std::io::prelude::*;
use std::io::{Cursor, Read, Seek, SeekFrom};

type Result<T> = std::result::Result<T, TextureParseError>;

#[allow(dead_code)]
pub struct Header {
    magic_id: u32,
    backward_compatibility: u8,
    forward_compatibility: u8,
    version: u16,
    contents_address: u32,
    strings_address: u32,
    commands_address: u32,
    raw_data_address: u32,
    raw_ext_address: u32, // if BackwardCompatibility > 0x20
    relocation_address: u32,
    contents_length: u32,
    strings_length: u32,
    commands_length: u32,
    raw_data_length: u32,
    raw_ext_length: u32, // if BackwardCompatibility > 0x20
    relocation_length: u32,
    uninit_data_length: u32,
    uninit_commands_length: u32,
}

impl Header {
    fn new(reader: &mut Cursor<&[u8]>) -> Result<Self> {
        let magic_id = reader.read_u32::<LittleEndian>()?;
        if magic_id != 0x484342 {
            return Err(TextureParseError::BadMagicNumber);
        }
        let backward_compatibility = reader.read_u8()?;
        let forward_compatibility = reader.read_u8()?;
        let version = reader.read_u16::<LittleEndian>()?;
        let contents_address = reader.read_u32::<LittleEndian>()?;
        let strings_address = reader.read_u32::<LittleEndian>()?;
        let commands_address = reader.read_u32::<LittleEndian>()?;
        let raw_data_address = reader.read_u32::<LittleEndian>()?;
        let raw_ext_address = if backward_compatibility > 20 {
            reader.read_u32::<LittleEndian>()?
        } else {
            0
        };
        let relocation_address = reader.read_u32::<LittleEndian>()?;
        let contents_length = reader.read_u32::<LittleEndian>()?;
        let strings_length = reader.read_u32::<LittleEndian>()?;
        let commands_length = reader.read_u32::<LittleEndian>()?;
        let raw_data_length = reader.read_u32::<LittleEndian>()?;
        let raw_ext_length = if backward_compatibility > 20 {
            reader.read_u32::<LittleEndian>()?
        } else {
            0
        };
        let relocation_length = reader.read_u32::<LittleEndian>()?;
        let uninit_data_length = reader.read_u32::<LittleEndian>()?;
        let uninit_commands_length = reader.read_u32::<LittleEndian>()?;

        Ok(Header {
            magic_id,
            backward_compatibility,
            forward_compatibility,
            version,
            contents_address,
            strings_address,
            commands_address,
            raw_data_address,
            raw_ext_address,
            relocation_address,
            contents_length,
            strings_length,
            commands_length,
            raw_data_length,
            raw_ext_length,
            relocation_length,
            uninit_data_length,
            uninit_commands_length,
        })
    }
}
pub struct ContentTable {
    textures_ptr_table_offset: u32,
    textures_ptr_table_entries: u32,
}

impl ContentTable {
    fn new(reader: &mut Cursor<&[u8]>, contents_address: u32) -> Result<Self> {
        reader.seek(SeekFrom::Start((contents_address + 0x24).into()))?;
        let textures_ptr_table_offset = reader.read_u32::<LittleEndian>()? + contents_address;
        let textures_ptr_table_entries = reader.read_u32::<LittleEndian>()?;

        Ok(ContentTable {
            textures_ptr_table_offset,
            textures_ptr_table_entries,
        })
    }
}

pub fn read(file: &[u8]) -> Result<Vec<Texture>> {
    let mut reader = Cursor::new(file);

    let header = Header::new(&mut reader)?;

    reader.seek(SeekFrom::Start(header.contents_address.into()))?;
    let content_table = ContentTable::new(&mut reader, header.contents_address)?;

    let mut bch: Vec<Texture> = Vec::new();

    for entry in 0..content_table.textures_ptr_table_entries {
        reader.seek(SeekFrom::Start(
            (content_table.textures_ptr_table_offset + entry * 4).into(),
        ))?;

        let dest = reader.read_u32::<LittleEndian>()?;
        reader.seek(SeekFrom::Start((dest + header.contents_address).into()))?;

        let tex_unit0_commands_offset =
            reader.read_u32::<LittleEndian>()? + header.commands_address;
        reader.seek(SeekFrom::Current(24))?;

        let name_offset = reader.read_u32::<LittleEndian>()?;

        // Read filename
        reader.seek(SeekFrom::Start(
            (header.strings_address + name_offset).into(),
        ))?;
        let mut filename_buffer: Vec<u8> = Vec::new();
        reader.read_until(0x0, &mut filename_buffer)?;
        filename_buffer.pop(); // Get rid of the null terminator.
        let (result, _, errors) = UTF_8.decode(filename_buffer.as_slice());
        if errors {
            return Err(TextureParseError::BadText);
        }
        let filename: String = result.into();

        reader.seek(SeekFrom::Start(tex_unit0_commands_offset.into()))?;
        let height = reader.read_u16::<LittleEndian>()? as usize;
        let width = reader.read_u16::<LittleEndian>()? as usize;
        reader.seek(SeekFrom::Current(0xC))?;
        let data_offset = reader.read_u32::<LittleEndian>()? + header.raw_data_address;
        reader.seek(SeekFrom::Current(0x4))?;
        let pixel_format = reader.read_u32::<LittleEndian>()?;

        reader.seek(SeekFrom::Start(data_offset.into()))?;
        let mut pixel_data: Vec<u8> = vec![
            0;
            (texture_decoder::get_pixel_format_bpp(pixel_format) * width as f32 * height as f32)
                as usize
        ];
        reader.read_exact(&mut pixel_data)?;
        let pixel_data =
            texture_decoder::decode_pixel_data(&pixel_data, width, height, pixel_format)?;
        bch.push(Texture {
            filename,
            height,
            width,
            pixel_data,
        });
    }
    Ok(bch)
}
