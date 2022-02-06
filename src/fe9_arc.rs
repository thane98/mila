use std::io::{Cursor, Read};
use linked_hash_map::LinkedHashMap;
use byteorder::{ReadBytesExt, BigEndian};
use crate::encoded_strings::{EncodedStringReader, to_shift_jis};

type Result<T> = std::result::Result<T, crate::ArchiveError>;

const BASE_HEADER_SIZE: usize = 8;
const PADDING_BOUNDARY: usize = 32;
const MAGIC: u32 = 0x7061636B;
const METADATA_SIZE: usize = 0x10;

struct EntryMetadata {
    name_address: u32,
    file_address: u32,
    file_size_unpadded: u32,
}

pub fn parse(raw: &[u8]) -> Result<LinkedHashMap<String, Vec<u8>>> {
    let mut cursor = Cursor::new(raw);

    // Validate magic number.
    let magic = cursor.read_u32::<BigEndian>()?;
    if magic != MAGIC {
        todo!()
    }

    // Retrieve the file count.
    let file_count = cursor.read_u16::<BigEndian>()?;

    // Read entry metadata.
    let mut entry_metadata = Vec::new();
    cursor.set_position(0x8);
    for _ in 0..file_count {
        entry_metadata.push(EntryMetadata::read(&mut cursor)?);
    }

    // Read the files.
    let mut entries: LinkedHashMap<String, Vec<u8>> = LinkedHashMap::new();
    for entry in entry_metadata {
        cursor.set_position(entry.name_address as u64);
        let name = cursor.read_shift_jis_string()?;
        cursor.set_position(entry.file_address as u64);
        let mut contents = vec![0; entry.file_size_unpadded as usize];
        cursor.read_exact(&mut contents)?;
        entries.insert(name, contents);
    }
    Ok(entries)
}

pub fn serialize(contents: &LinkedHashMap<String, Vec<u8>>) -> Result<Vec<u8>> {
    let header_length = BASE_HEADER_SIZE + contents.len() * METADATA_SIZE;

    // Three sections: header, text (file names), contents.
    // Start with text since we need info from it to fill out the header.
    let mut raw_text = Vec::new();
    let mut text_addresses = Vec::new();
    for k in contents.keys() {
        let offset = header_length + raw_text.len();
        text_addresses.push(offset);
        let raw = to_shift_jis(k)?;
        raw_text.extend(raw);
        raw_text.push(0);
    }
    while (header_length + raw_text.len()) % PADDING_BOUNDARY != 0 {
        raw_text.push(0);
    }

    //Compute file addresses.
    let mut next_file_address = header_length + raw_text.len();
    let mut raw_files = Vec::new();
    let mut file_info = Vec::new();
    for raw_file in contents.values() {
        file_info.push((next_file_address, raw_file.len()));
        raw_files.extend(raw_file);
        while (header_length + raw_text.len() + raw_files.len()) % PADDING_BOUNDARY != 0 {
            raw_files.push(0);
        }
        next_file_address = header_length + raw_text.len() + raw_files.len();
    }

    // Assemble the file.
    let mut archive: Vec<u8> = Vec::new();
    archive.extend(MAGIC.to_be_bytes().iter());
    archive.extend((contents.len() as u16).to_be_bytes().iter());
    archive.push(0);
    archive.push(0);
    for i in 0..contents.len() {
        archive.resize(archive.len() + 4, 0);
        archive.extend((text_addresses[i] as u32).to_be_bytes().iter());
        let (file_address, file_size_unpadded) = file_info[i];
        archive.extend((file_address as u32).to_be_bytes().iter());
        archive.extend((file_size_unpadded as u32).to_be_bytes().iter());
    }
    archive.extend(raw_text);
    archive.extend(raw_files);
    Ok(archive)
}

impl EntryMetadata {
    pub fn read(cursor: &mut Cursor<&[u8]>) -> Result<Self> {
        let _unknown = cursor.read_u32::<BigEndian>()?;
        let name_address = cursor.read_u32::<BigEndian>()?;
        let file_address = cursor.read_u32::<BigEndian>()?;
        let file_size_unpadded = cursor.read_u32::<BigEndian>()?;
        Ok(EntryMetadata {
            name_address,
            file_address,
            file_size_unpadded
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::load_test_file;

    #[test]
    fn round_trip() {
        let expected1: Vec<u8> = vec![1, 2, 3, 4, 5];
        let expected2: Vec<u8> = vec![6, 7, 8, 9, 10, 11];
        let raw_file = load_test_file("FE9Arc.bin");
        let arc = parse(&raw_file).unwrap();
        assert_eq!(2, arc.len());
        assert_eq!(expected1, arc.get("FE9ArcTest1.bin").unwrap().clone());
        assert_eq!(expected2, arc.get("FE9ArcTest2.bin").unwrap().clone());
        
        let serialized = serialize(&arc).unwrap();
        assert_eq!(raw_file, serialized);
    }
}