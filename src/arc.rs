use crate::bin_archive::BinArchive;
use crate::bin_streams::BinArchiveReader;
use crate::{ArcError, Endian};
use std::collections::HashMap;

type Result<T> = std::result::Result<T, ArcError>;

#[allow(dead_code)]
struct ArcEntry {
    name: String,
    index: u32,
    size: u32,
    address: u32,
}

pub fn from_bytes(bytes: &[u8]) -> Result<HashMap<String, Vec<u8>>> {
    // Read archive and labels.
    let archive = BinArchive::from_bytes(bytes, Endian::Little)?;
    let count_address = archive
        .find_label_address("Count")
        .ok_or(ArcError::NoCount)?;
    let info_address = archive.find_label_address("Info").ok_or(ArcError::NoInfo)?;
    let header_padding = if archive.read_u32(0)? == 0 { 0x60 } else { 0 };

    // Read metadata
    let mut entries: Vec<ArcEntry> = Vec::new();
    let mut reader = BinArchiveReader::new(&archive, count_address);
    let count = reader.read_u32()?;
    reader.seek(info_address);
    for _ in 0..count {
        let name = reader.read_string()?.ok_or(ArcError::MissingName)?;
        let index = reader.read_u32()?;
        let size = reader.read_u32()?;
        let address = reader.read_u32()? + header_padding;
        entries.push(ArcEntry {
            name,
            index,
            size,
            address,
        });
    }

    // Read files.
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    for entry in entries {
        reader.seek(entry.address as usize);
        let buffer = reader.read_bytes(entry.size as usize)?;
        files.insert(entry.name, buffer);
    }
    Ok(files)
}

#[cfg(test)]
mod test {
    use crate::utils::load_test_file;

    #[test]
    fn arc_from_bytes_test() {
        let raw_arc = load_test_file("ArcTest.arc");
        let test_file_1 = load_test_file("LZ13Test.bin");
        let test_file_2 = load_test_file("LZ13Test.bin.lz");
        let result = super::from_bytes(&raw_arc);
        assert!(result.is_ok());
        let files = result.unwrap();
        assert_eq!(2, files.len());
        assert!(files.contains_key("LZ13Test.bin"));
        assert!(files.contains_key("LZ13Test.bin.lz"));
        assert_eq!(&test_file_1, files.get("LZ13Test.bin").unwrap());
        assert_eq!(&test_file_2, files.get("LZ13Test.bin.lz").unwrap());
    }
}
