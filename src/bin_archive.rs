use crate::encoded_strings::{EncodedStringReader, to_shift_jis};
use crate::errors::ArchiveError;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

type Result<T> = std::result::Result<T, ArchiveError>;

#[derive(Debug)]
pub struct BinArchive {
    data: Vec<u8>,
    text: HashMap<usize, String>,
    pointers: HashMap<usize, usize>,
    labels: HashMap<usize, Vec<String>>,
}

fn validate_address(address: usize, size: usize, end_is_valid: bool) -> Result<()> {
    if (end_is_valid && address > size) || (!end_is_valid && address >= size) {
        Err(ArchiveError::OutOfBoundsAddress(address, size))
    } else {
        Ok(())
    }
}

fn validate_alignment(value: usize, bytes: usize) -> Result<()> {
    if value % bytes != 0 {
        Err(ArchiveError::UnalignedValue(value, bytes))
    } else {
        Ok(())
    }
}

fn add_text(
    raw_text: &mut Vec<u8>,
    raw_text_offsets: &mut HashMap<String, usize>,
    text: &String,
) -> Result<usize> {
    match raw_text_offsets.get(text) {
        Some(value) => Ok(*value),
        None => {
            let offset = raw_text.len();
            raw_text.extend(to_shift_jis(text)?);
            raw_text.push(0);
            raw_text_offsets.insert(text.clone(), offset);
            Ok(offset)
        }
    }
}

fn adjust_pointer(pointer: usize, address: usize, count: usize, subtract: bool) -> usize {
    if pointer >= address {
        if subtract {
            pointer - count
        } else {
            pointer + count
        }
    } else {
        pointer
    }
}

fn adjust_text<T: Clone>(
    map: &HashMap<usize, T>,
    address: usize,
    count: usize,
    subtract: bool,
) -> HashMap<usize, T> {
    map.iter()
        .map(|(addr, value)| {
            let new_pointer = adjust_pointer(*addr, address, count, subtract);
            (new_pointer, value.clone())
        })
        .collect()
}

fn adjust_labels<T: Clone>(
    map: &HashMap<usize, T>,
    address: usize,
    count: usize,
    subtract: bool,
) -> HashMap<usize, T> {
    map.iter()
        .map(|(addr, value)| {
            let pointer = *addr;
            let new_pointer = if pointer > address {
                if subtract {
                    pointer - count
                } else {
                    pointer + count
                }
            } else {
                pointer
            };
            (new_pointer, value.clone())
        })
        .collect()
}

fn adjust_pointers(
    map: &HashMap<usize, usize>,
    address: usize,
    count: usize,
    subtract: bool,
) -> HashMap<usize, usize> {
    map.iter()
        .map(|(source, destination)| {
            let new_source = adjust_pointer(*source, address, count, subtract);
            let new_destination = if *destination > address {
                if subtract {
                    destination - count
                } else {
                    destination + count
                }
            } else {
                *destination
            };
            (new_source, new_destination)
        })
        .collect()
}

impl BinArchive {
    pub fn new() -> Self {
        BinArchive {
            data: Vec::new(),
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 0x20 {
            return Err(ArchiveError::ArchiveTooSmall);
        }
        let mut cursor = Cursor::new(bytes);
        let file_size = cursor.read_u32::<LittleEndian>()?;
        let data_size = cursor.read_u32::<LittleEndian>()?;
        let pointer_count = cursor.read_u32::<LittleEndian>()?;
        let label_count = cursor.read_u32::<LittleEndian>()?;
        let text_start = (data_size + (pointer_count * 4) + (label_count * 8)) as usize;
        if file_size as usize != bytes.len() {
            return Err(ArchiveError::SizeMismatch);
        }
        if text_start + 0x20 > bytes.len() {
            return Err(ArchiveError::ArchiveTooSmall);
        }

        let mut archive = BinArchive::new();
        cursor.seek(SeekFrom::Start(0x20))?;
        archive.data.resize(data_size as usize, 0);
        cursor.read_exact(&mut archive.data)?;
        for _ in 0..pointer_count {
            let pointer_address = cursor.read_u32::<LittleEndian>()? as usize;
            let pointer_value = archive.read_u32(pointer_address as usize)? as usize;
            if pointer_value > data_size as usize {
                let original_position = cursor.position();
                cursor.seek(SeekFrom::Start((pointer_value + 0x20) as u64))?;
                let string = cursor.read_shift_jis_string()?;
                cursor.seek(SeekFrom::Start(original_position))?;
                archive.write_string(pointer_address, Some(&string))?;
            } else {
                archive.write_pointer(pointer_address, Some(pointer_value))?;
            }
        }

        for _ in 0..label_count {
            let address = cursor.read_u32::<LittleEndian>()?;
            let offset = cursor.read_u32::<LittleEndian>()? as usize;
            let text_address = text_start + offset + 0x20;
            let original_position = cursor.position();
            cursor.seek(SeekFrom::Start(text_address as u64))?;
            let string = cursor.read_shift_jis_string()?;
            cursor.seek(SeekFrom::Start(original_position))?;
            archive.write_label(address as usize, &string)?;
        }
        Ok(archive)
    }

    pub fn get_labels(&self) -> Vec<(usize, String)> {
        let mut keys: Vec<(usize, String)> = Vec::new();
        for (k, v) in &self.labels {
            for s in v {
                keys.push((*k, s.clone()));
            }
        }
        keys.sort();
        keys
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut data = self.data.clone();
        let mut raw_pointers: Vec<u32> = Vec::new();
        let mut raw_labels: Vec<u32> = Vec::new();
        let mut raw_text: Vec<u8> = Vec::new();
        let mut raw_text_offsets: HashMap<String, usize> = HashMap::new();
        let mut cursor = Cursor::new(&mut data);

        let mut pointers: Vec<(&usize, &usize)> = self.pointers.iter().collect();
        pointers.sort_by(|a, b| a.0.cmp(b.0));
        for (source, destination) in pointers {
            cursor.seek(SeekFrom::Start(*source as u64))?;
            cursor.write_u32::<LittleEndian>(*destination as u32)?;
            raw_pointers.push(*source as u32);
        }

        let mut labels: Vec<(&usize, &Vec<String>)> = self.labels.iter().collect();
        labels.sort_by(|a, b| a.0.cmp(b.0));
        for (address, bucket) in labels {
            for label in bucket {
                let offset = add_text(&mut raw_text, &mut raw_text_offsets, label)?;
                raw_labels.push(*address as u32);
                raw_labels.push(offset as u32);
            }
        }

        let mut text: Vec<(&usize, &String)> = self.text.iter().collect();
        text.sort_by(|a, b| a.0.cmp(b.0));
        let mut ptr_data_pairs: LinkedHashMap<usize, Vec<u32>> = LinkedHashMap::new();
        let text_start =
            self.data.len() + (self.pointers.len() + self.text.len() + raw_labels.len()) * 4;
        for (address, string) in text {
            let offset = add_text(&mut raw_text, &mut raw_text_offsets, string)?;
            let text_address = text_start + offset;
            cursor.seek(SeekFrom::Start(*address as u64))?;
            cursor.write_u32::<LittleEndian>(text_address as u32)?;
            match ptr_data_pairs.get_mut(&offset) {
                Some(bucket) => {
                    bucket.push(*address as u32);
                }
                None => {
                    let mut bucket: Vec<u32> = Vec::new();
                    bucket.push(*address as u32);
                    ptr_data_pairs.insert(offset, bucket);
                }
            }
        }
        for mut ptr_data_pair in ptr_data_pairs {
            ptr_data_pair.1.sort();
            for ptr in ptr_data_pair.1 {
                raw_pointers.push(ptr);
            }
        }

        let mut bytes: Vec<u8> = Vec::new();
        let file_size = self.data.len()
            + (raw_pointers.len() * 4)
            + (raw_labels.len() * 4)
            + raw_text.len()
            + 0x20;
        bytes.resize(file_size, 0);
        let mut cursor = Cursor::new(&mut bytes);
        cursor.write_u32::<LittleEndian>(file_size as u32)?;
        cursor.write_u32::<LittleEndian>(data.len() as u32)?;
        cursor.write_u32::<LittleEndian>(raw_pointers.len() as u32)?;
        cursor.write_u32::<LittleEndian>((raw_labels.len() / 2) as u32)?;
        cursor.seek(SeekFrom::Start(0x20))?;
        cursor.write(&data)?;
        for pointer in raw_pointers {
            cursor.write_u32::<LittleEndian>(pointer)?;
        }
        for label_part in raw_labels {
            cursor.write_u32::<LittleEndian>(label_part)?;
        }
        cursor.write(&raw_text)?;
        Ok(bytes)
    }

    pub fn read_f32(&self, address: usize) -> Result<f32> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        let mut cursor = Cursor::new(&self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        Ok(cursor.read_f32::<LittleEndian>()?)
    }

    pub fn read_u8(&self, address: usize) -> Result<u8> {
        validate_address(address, self.size(), false)?;
        let mut cursor = Cursor::new(&self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        Ok(cursor.read_u8()?)
    }

    pub fn read_u16(&self, address: usize) -> Result<u16> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 2, self.size(), true)?;
        validate_alignment(address, 2)?;
        let mut cursor = Cursor::new(&self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        Ok(cursor.read_u16::<LittleEndian>()?)
    }

    pub fn read_u32(&self, address: usize) -> Result<u32> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        let mut cursor = Cursor::new(&self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        Ok(cursor.read_u32::<LittleEndian>()?)
    }

    pub fn read_i8(&self, address: usize) -> Result<i8> {
        validate_address(address, self.size(), false)?;
        let mut cursor = Cursor::new(&self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        Ok(cursor.read_i8()?)
    }

    pub fn read_i16(&self, address: usize) -> Result<i16> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 2, self.size(), true)?;
        validate_alignment(address, 2)?;
        let mut cursor = Cursor::new(&self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        Ok(cursor.read_i16::<LittleEndian>()?)
    }

    pub fn read_i32(&self, address: usize) -> Result<i32> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        let mut cursor = Cursor::new(&self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        Ok(cursor.read_i32::<LittleEndian>()?)
    }

    pub fn read_bytes(&self, address: usize, amount: usize) -> Result<&[u8]> {
        validate_address(address, self.size(), false)?;
        validate_address(address + amount, self.size(), true)?;
        Ok(&self.data[address..(address + amount)])
    }

    pub fn read_string(&self, address: usize) -> Result<Option<String>> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        Ok(self.text.get(&address).map(|x| x.to_owned()))
    }

    pub fn read_pointer(&self, address: usize) -> Result<Option<usize>> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        Ok(self.pointers.get(&address).map(|x| x.to_owned()))
    }

    pub fn read_labels(&self, address: usize) -> Result<Option<Vec<String>>> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        Ok(self.labels.get(&address).map(|x| x.to_owned()))
    }

    pub fn delete_string(&mut self, address: usize) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        self.text.remove(&address);
        Ok(())
    }

    pub fn delete_pointer(&mut self, address: usize) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        self.pointers.remove(&address);
        Ok(())
    }

    pub fn delete_labels(&mut self, address: usize) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        self.labels.remove(&address);
        Ok(())
    }

    pub fn delete_label(&mut self, address: usize, index: usize) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        match self.labels.get_mut(&address) {
            Some(bucket) => {
                if index < bucket.len() {
                    bucket.remove(index);
                    Ok(())
                } else {
                    Err(ArchiveError::LabelIndexOutOfBounds(index, bucket.len()))
                }
            }
            None => Ok(()),
        }
    }

    pub fn write_f32(&mut self, address: usize, value: f32) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        let mut cursor = Cursor::new(&mut self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        cursor.write_f32::<LittleEndian>(value)?;
        Ok(())
    }

    pub fn write_u8(&mut self, address: usize, value: u8) -> Result<()> {
        validate_address(address, self.size(), false)?;
        let mut cursor = Cursor::new(&mut self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        cursor.write_u8(value)?;
        Ok(())
    }

    pub fn write_u16(&mut self, address: usize, value: u16) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 2, self.size(), true)?;
        validate_alignment(address, 2)?;
        let mut cursor = Cursor::new(&mut self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        cursor.write_u16::<LittleEndian>(value)?;
        Ok(())
    }

    pub fn write_u32(&mut self, address: usize, value: u32) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        let mut cursor = Cursor::new(&mut self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        cursor.write_u32::<LittleEndian>(value)?;
        Ok(())
    }

    pub fn write_i8(&mut self, address: usize, value: i8) -> Result<()> {
        validate_address(address, self.size(), false)?;
        let mut cursor = Cursor::new(&mut self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        cursor.write_i8(value)?;
        Ok(())
    }

    pub fn write_i16(&mut self, address: usize, value: i16) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 2, self.size(), true)?;
        validate_alignment(address, 2)?;
        let mut cursor = Cursor::new(&mut self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        cursor.write_i16::<LittleEndian>(value)?;
        Ok(())
    }

    pub fn write_i32(&mut self, address: usize, value: i32) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        validate_alignment(address, 4)?;
        let mut cursor = Cursor::new(&mut self.data);
        cursor.seek(SeekFrom::Start(address as u64))?;
        cursor.write_i32::<LittleEndian>(value)?;
        Ok(())
    }

    pub fn write_bytes(&mut self, address: usize, bytes: &[u8]) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + bytes.len(), self.size(), true)?;
        self.data[address..(address + bytes.len())].copy_from_slice(bytes);
        Ok(())
    }

    pub fn write_string(&mut self, address: usize, value: Option<&str>) -> Result<()> {
        match value {
            Some(value) => {
                validate_address(address, self.size(), false)?;
                validate_address(address + 4, self.size(), true)?;
                validate_alignment(address, 4)?;
                self.text.insert(address, value.to_owned());
                Ok(())
            }
            None => self.delete_string(address),
        }
    }

    pub fn write_pointer(&mut self, address: usize, value: Option<usize>) -> Result<()> {
        match value {
            Some(value) => {
                validate_address(address, self.size(), false)?;
                validate_address(address + 4, self.size(), true)?;
                validate_alignment(address, 4)?;
                self.pointers.insert(address, value);
                Ok(())
            }
            None => self.delete_pointer(address),
        }
    }

    pub fn write_labels(&mut self, address: usize, labels: Vec<String>) -> Result<()> {
        validate_address(address, self.size(), true)?;
        validate_alignment(address, 4)?;
        self.labels.insert(address, labels);
        Ok(())
    }

    pub fn write_label(&mut self, address: usize, label: &str) -> Result<()> {
        validate_address(address, self.size(), true)?;
        validate_alignment(address, 4)?;
        match self.labels.get_mut(&address) {
            Some(bucket) => {
                bucket.push(label.to_owned());
                Ok(())
            }
            None => {
                let mut bucket: Vec<String> = Vec::new();
                bucket.push(label.to_owned());
                self.labels.insert(address, bucket);
                Ok(())
            }
        }
    }

    pub fn allocate_at_end(&mut self, amount_in_bytes: usize) {
        for _ in 0..amount_in_bytes {
            self.data.push(0);
        }
    }

    pub fn allocate(&mut self, address: usize, amount_in_bytes: usize) -> Result<()> {
        validate_address(address, self.size(), true)?;
        validate_alignment(address, 4)?;
        validate_alignment(amount_in_bytes, 4)?;
        let bytes_to_insert: Vec<u8> = vec![0; amount_in_bytes];
        self.data
            .splice(address..address, bytes_to_insert.iter().cloned());
        let new_text = adjust_text(&self.text, address, amount_in_bytes, false);
        let new_labels = adjust_labels(&self.labels, address, amount_in_bytes, false);
        let new_pointers = adjust_pointers(&self.pointers, address, amount_in_bytes, false);
        self.text = new_text;
        self.labels = new_labels;
        self.pointers = new_pointers;
        Ok(())
    }

    pub fn find_label_address(&self, target: &str) -> Option<usize> {
        for (address, bucket) in &self.labels {
            for label in bucket {
                if label == target {
                    return Some(*address);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::BinArchive;
    use crate::utils::load_test_file;
    use maplit::hashmap;
    use std::collections::HashMap;

    #[test]
    fn size() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let archive2 = BinArchive::new();
        assert_eq!(archive.size(), 4);
        assert_eq!(archive2.size(), 0);
    }

    #[test]
    fn get_labels() {
        let labels = vec![
            "Owain".to_string(),
            "Severa".to_string()
        ];
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: hashmap! {
                0 => vec!["Test".to_string()],
                4 => labels.clone()
            },
        };
        let labels = archive.get_labels();
        assert_eq!(labels, vec![(0, "Test".to_string()), (4, "Owain".to_string()), (4, "Severa".to_string())]);
    }

    #[test]
    fn read_f32() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0x3F, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let result1 = archive.read_f32(4);
        let result2 = archive.read_f32(2);
        let result3 = archive.read_f32(9);
        let result4 = archive.read_f32(8);
        assert!(result1.is_ok(), true);
        assert_eq!(result1.unwrap(), 0.5);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn read_u8() {
        let archive = BinArchive {
            data: vec![0, 23],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let result1 = archive.read_u8(1);
        let result2 = archive.read_u8(2);
        assert!(result1.is_ok(), true);
        assert_eq!(result1.unwrap(), 23);
        assert!(result2.is_err());
    }

    #[test]
    fn read_u16() {
        let archive = BinArchive {
            data: vec![0, 0, 0x14, 0xFE, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let result1 = archive.read_u16(2);
        let result2 = archive.read_u16(1);
        let result3 = archive.read_u16(8);
        let result4 = archive.read_u16(4);
        assert!(result1.is_ok(), true);
        assert_eq!(result1.unwrap(), 0xFE14);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn read_u32() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0x14, 0xFE, 0x15, 0xFE, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let result1 = archive.read_u32(4);
        let result2 = archive.read_u32(2);
        let result3 = archive.read_u32(9);
        let result4 = archive.read_u32(8);
        assert!(result1.is_ok(), true);
        assert_eq!(result1.unwrap(), 0xFE15FE14);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn read_i8() {
        let archive = BinArchive {
            data: vec![0, 23],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let result1 = archive.read_i8(1);
        let result2 = archive.read_i8(2);
        assert!(result1.is_ok(), true);
        assert_eq!(result1.unwrap(), 23);
        assert!(result2.is_err());
    }

    #[test]
    fn read_i16() {
        let archive = BinArchive {
            data: vec![0, 0, 0x12, 0x11, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let result1 = archive.read_i16(2);
        let result2 = archive.read_i16(1);
        let result3 = archive.read_i16(8);
        let result4 = archive.read_i16(4);
        assert!(result1.is_ok(), true);
        assert_eq!(result1.unwrap(), 0x1112);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn read_i32() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0x14, 0x11, 0x15, 0x11, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let result1 = archive.read_u32(4);
        let result2 = archive.read_u32(2);
        let result3 = archive.read_u32(9);
        let result4 = archive.read_u32(8);
        assert!(result1.is_ok(), true);
        assert_eq!(result1.unwrap(), 0x11151114);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn read_bytes() {
        let archive = BinArchive {
            data: vec![0, 0x14, 0x11, 0x15, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected1: Vec<u8> = vec![0x14, 0x11, 0x15];
        let result1 = archive.read_bytes(1, 3);
        let result2 = archive.read_bytes(1, 6);
        let result3 = archive.read_bytes(6, 1);
        assert!(result1.is_ok());
        assert_eq!(expected1, result1.unwrap());
        assert!(result2.is_err());
        assert!(result3.is_err());
    }

    #[test]
    fn read_string() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: hashmap! {
                4 => "test".to_string()
            },
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let result1 = archive.read_string(4);
        let result2 = archive.read_string(2);
        let result3 = archive.read_string(8);
        let result4 = archive.read_string(12);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), Some("test".to_string()));
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn read_pointer() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: hashmap! {
                4 => 0
            },
            labels: HashMap::new(),
        };
        let result1 = archive.read_pointer(4);
        let result2 = archive.read_pointer(2);
        let result3 = archive.read_pointer(8);
        let result4 = archive.read_pointer(12);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), Some(0));
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn read_labels() {
        let labels = vec![
            "Owain".to_string(),
            "Severa".to_string(),
            "Inigo".to_string(),
        ];
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: hashmap! {
                4 => labels.clone()
            },
        };
        let result1 = archive.read_labels(4);
        let result2 = archive.read_labels(2);
        let result3 = archive.read_labels(8);
        let result4 = archive.read_labels(12);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), Some(labels));
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn delete_string() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: hashmap! {
                4 => "test".to_string()
            },
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: HashMap<usize, String> = HashMap::new();
        let result1 = archive.delete_string(4);
        let result2 = archive.delete_string(2);
        let result3 = archive.delete_string(8);
        let result4 = archive.delete_string(12);
        assert!(result1.is_ok());
        assert_eq!(archive.text, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn delete_pointer() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: hashmap! {
                4 => 0
            },
            labels: HashMap::new(),
        };
        let expected: HashMap<usize, usize> = HashMap::new();
        let result1 = archive.delete_pointer(4);
        let result2 = archive.delete_pointer(2);
        let result3 = archive.delete_pointer(8);
        let result4 = archive.delete_pointer(12);
        assert!(result1.is_ok());
        assert_eq!(archive.pointers, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn delete_labels() {
        let labels = vec![
            "Owain".to_string(),
            "Severa".to_string(),
            "Inigo".to_string(),
        ];
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: hashmap! {
                4 => labels.clone()
            },
        };
        let expected: HashMap<usize, Vec<String>> = HashMap::new();
        let result1 = archive.delete_labels(4);
        let result2 = archive.delete_labels(2);
        let result3 = archive.delete_labels(8);
        let result4 = archive.delete_labels(12);
        assert!(result1.is_ok());
        assert_eq!(archive.labels, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn delete_label() {
        let labels = vec![
            "Owain".to_string(),
            "Severa".to_string(),
            "Inigo".to_string(),
        ];
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: hashmap! {
                4 => labels.clone()
            },
        };
        let expected: HashMap<usize, Vec<String>> = hashmap! {
            4 => vec!["Owain".to_string(), "Inigo".to_string()]
        };
        let result1 = archive.delete_label(4, 1);
        let result2 = archive.delete_label(4, 2);
        assert!(result1.is_ok());
        assert_eq!(archive.labels, expected);
        assert!(result2.is_err());
    }

    #[test]
    fn write_f32() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0x3F, 0];
        let result1 = archive.write_f32(4, 0.5);
        let result2 = archive.write_f32(2, 0.5);
        let result3 = archive.write_f32(9, 0.5);
        let result4 = archive.write_f32(8, 0.5);
        assert!(result1.is_ok(), true);
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn write_u8() {
        let mut archive = BinArchive {
            data: vec![0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: Vec<u8> = vec![0, 0x23];
        let result1 = archive.write_u8(1, 0x23);
        let result2 = archive.write_u8(2, 0x23);
        assert!(result1.is_ok(), true);
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
    }

    #[test]
    fn write_u16() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: Vec<u8> = vec![0, 0, 0x12, 0x11, 0];
        let result1 = archive.write_u16(2, 0x1112);
        let result2 = archive.write_u16(1, 0x1112);
        let result3 = archive.write_u16(8, 0x1112);
        let result4 = archive.write_u16(4, 0x1112);
        assert!(result1.is_ok(), true);
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn write_u32() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: Vec<u8> = vec![0, 0, 0, 0, 0x12, 0x11, 0x22, 0x23, 0];
        let result1 = archive.write_u32(4, 0x23221112);
        let result2 = archive.write_u32(1, 0x23221112);
        let result3 = archive.write_u32(8, 0x23221112);
        let result4 = archive.write_u32(2, 0x23221112);
        assert!(result1.is_ok(), true);
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn write_i8() {
        let mut archive = BinArchive {
            data: vec![0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: Vec<u8> = vec![0, 0x23];
        let result1 = archive.write_i8(1, 0x23);
        let result2 = archive.write_i8(2, 0x23);
        assert!(result1.is_ok(), true);
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
    }

    #[test]
    fn write_i16() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: Vec<u8> = vec![0, 0, 0x12, 0x11, 0];
        let result1 = archive.write_i16(2, 0x1112);
        let result2 = archive.write_i16(1, 0x1112);
        let result3 = archive.write_i16(8, 0x1112);
        let result4 = archive.write_i16(4, 0x1112);
        assert!(result1.is_ok(), true);
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn write_i32() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: Vec<u8> = vec![0, 0, 0, 0, 0x12, 0x11, 0x22, 0x23, 0];
        let result1 = archive.write_i32(4, 0x23221112);
        let result2 = archive.write_i32(1, 0x23221112);
        let result3 = archive.write_i32(8, 0x23221112);
        let result4 = archive.write_i32(2, 0x23221112);
        assert!(result1.is_ok(), true);
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn write_bytes() {
        let bytes: Vec<u8> = vec![0xFE, 0xFF];
        let mut archive = BinArchive {
            data: vec![0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: Vec<u8> = vec![0, 0xFE, 0xFF];
        let result1 = archive.write_bytes(1, &bytes);
        let result2 = archive.write_bytes(2, &bytes);
        let result3 = archive.write_bytes(3, &bytes);
        assert!(result1.is_ok(), true);
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
    }

    #[test]
    fn write_string() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: HashMap<usize, String> = hashmap! {
            4 => "test".to_string()
        };
        let result1 = archive.write_string(4, Some("test"));
        let result2 = archive.write_string(3, Some("test"));
        let result3 = archive.write_string(8, Some("test"));
        let result4 = archive.write_string(9, Some("test"));
        assert!(result1.is_ok(), true);
        assert_eq!(archive.text, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn write_pointer() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: HashMap<usize, usize> = hashmap! {
            4 => 0
        };
        let result1 = archive.write_pointer(4, Some(0));
        let result2 = archive.write_pointer(3, Some(0));
        let result3 = archive.write_pointer(8, Some(0));
        let result4 = archive.write_pointer(9, Some(0));
        assert!(result1.is_ok(), true);
        assert_eq!(archive.pointers, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn write_labels() {
        let labels = vec![
            "Owain".to_string(),
            "Severa".to_string(),
            "Inigo".to_string(),
        ];
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: HashMap<usize, Vec<String>> = hashmap! {
            4 => labels.clone(),
            8 => labels.clone(),
        };
        let result1 = archive.write_labels(4, labels.clone());
        let result2 = archive.write_labels(3, labels.clone());
        let result3 = archive.write_labels(8, labels.clone());
        let result4 = archive.write_labels(9, labels.clone());
        assert!(result1.is_ok());
        assert_eq!(archive.labels, expected);
        assert!(result2.is_err());
        assert!(result3.is_ok());
        assert!(result4.is_err());
    }

    #[test]
    fn write_label() {
        let labels1 = vec!["Owain".to_string()];
        let labels2 = vec![
            "Owain".to_string(),
            "Severa".to_string(),
            "Inigo".to_string(),
        ];
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: hashmap! {
                4 => labels1
            },
        };
        let expected: HashMap<usize, Vec<String>> = hashmap! {
            0 => vec!["test".to_string()],
            4 => labels2.clone()
        };
        let result1 = archive.write_label(4, "Severa");
        let result2 = archive.write_label(4, "Inigo");
        let result3 = archive.write_label(0, "test");
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
        assert_eq!(archive.labels, expected);
    }

    #[test]
    fn find_label_address() {
        let labels = vec![
            "Owain".to_string(),
            "Severa".to_string(),
            "Inigo".to_string(),
        ];
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: hashmap! {
                0 => vec!["test".to_string()],
                4 => labels
            },
        };
        let search1 = archive.find_label_address("Selena");
        let search2 = archive.find_label_address("Severa");
        let search3 = archive.find_label_address("test");
        assert!(search1.is_none());
        assert!(search2.is_some());
        assert_eq!(search2.unwrap(), 4);
        assert!(search3.is_some());
        assert_eq!(search3.unwrap(), 0);
    }

    #[test]
    fn allocate_at_end() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let expected: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0];
        archive.allocate_at_end(4);
        assert_eq!(archive.size(), 8);
        assert_eq!(archive.data, expected);
    }

    #[test]
    fn allocate_validation() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
        };
        let result1 = archive.allocate(2, 4);
        let result2 = archive.allocate(0, 3);
        let result3 = archive.allocate(8, 4);
        assert!(result1.is_err());
        assert!(result2.is_err());
        assert!(result3.is_err());
    }

    #[test]
    fn allocate_mixed2() {
        test_allocation(
            "ArchiveTest_Mixed2.bin",
            "ArchiveTest_Allocate_Mixed2.bin",
            24,
            24,
        );
    }

    #[test]
    fn allocate_no_label_shift() {
        let bytes = load_test_file("Allocate_NoLabelShift.bin");
        let mut archive = BinArchive::from_bytes(&bytes).unwrap();
        assert!(archive.allocate(0x8, 0x10).is_ok());
        assert_eq!(archive.read_labels(0x8).unwrap().unwrap(), vec!("TEST".to_string()));
        assert_eq!(archive.read_labels(0x1C).unwrap().unwrap(), vec!("TEST2".to_string()));
        assert!(archive.read_labels(0xC).unwrap().is_none());
    }

    #[test]
    fn allocate_no_destination_shift() {
        let bytes = load_test_file("Allocate_NoDestinationShift.bin");
        let mut archive = BinArchive::from_bytes(&bytes).unwrap();
        assert!(archive.allocate(0x10, 0x10).is_ok());
        assert_eq!(archive.read_pointer(0x8).unwrap().unwrap(), 0x10);
        assert!(archive.allocate(0xC, 0x10).is_ok());
        assert_eq!(archive.read_pointer(0x8).unwrap().unwrap(), 0x20);
    }

    #[test]
    fn from_bytes_bad_internal_pointer() {
        test_archive_for_error("ArchiveTest_BadInternalPointer.bin");
    }

    #[test]
    fn from_bytes_bad_size() {
        test_archive_for_error("ArchiveTest_BadSize.bin");
    }

    #[test]
    fn from_bytes_file_size_mismatch() {
        test_archive_for_error("ArchiveTest_FileSizeMismatch.bin");
    }

    #[test]
    fn round_trip_only_text() {
        test_archive_for_success("ArchiveTest_OnlyText.bin");
    }

    #[test]
    fn round_trip_mixed1() {
        test_archive_for_success("ArchiveTest_Mixed1.bin");
    }

    #[test]
    fn round_trip_mixed2() {
        test_archive_for_success("ArchiveTest_Mixed2.bin");
    }

    fn test_allocation(
        source_file_name: &str,
        result_file_name: &str,
        address: usize,
        count: usize,
    ) {
        let bytes = load_test_file(source_file_name);
        let expected = load_test_file(result_file_name);
        let result = BinArchive::from_bytes(&bytes);
        assert!(result.is_ok());
        let mut archive = result.unwrap();
        let result = archive.allocate(address, count);
        assert!(result.is_ok());
        let result = archive.serialize();
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(bytes, expected);
    }

    fn test_archive_for_success(file_name: &str) {
        let bytes = load_test_file(file_name);
        let result = BinArchive::from_bytes(&bytes);
        assert!(result.is_ok());
        let result = result.unwrap().serialize();
        assert!(result.is_ok());
        let result_bytes = result.unwrap();
        assert_eq!(result_bytes, bytes);
    }

    fn test_archive_for_error(file_name: &str) {
        let bytes = load_test_file(file_name);
        let result = BinArchive::from_bytes(&bytes);
        assert!(result.is_err());
    }
}
