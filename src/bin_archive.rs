use crate::encoded_strings::{to_shift_jis, EncodedStringReader};
use crate::errors::ArchiveError;
use crate::{Endian, EndianAwareReader, EndianAwareWriter};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

type Result<T> = std::result::Result<T, ArchiveError>;

#[derive(Debug)]
pub struct BinArchive {
    data: Vec<u8>,
    text: HashMap<usize, String>,
    pointers: HashMap<usize, usize>,
    labels: HashMap<usize, Vec<String>>,
    cstrings: HashMap<String, Vec<usize>>,
    endian: Endian,
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

fn filter_text_or_labels<T: Clone>(
    map: &HashMap<usize, T>,
    address: usize,
    count: usize,
) -> HashMap<usize, T> {
    let range = address..(address + count);
    map.iter()
        .filter(|(addr, _)| !range.contains(&addr))
        .map(|(addr, value)| (*addr, value.clone()))
        .collect()
}

fn filter_pointers(
    map: &HashMap<usize, usize>,
    address: usize,
    count: usize,
) -> HashMap<usize, usize> {
    let range = address..(address + count);
    map.iter()
        .filter(|(source, destination)| !(range.contains(&source) || range.contains(&destination)))
        .map(|(a, b)| (*a, *b))
        .collect()
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
    ge: bool,
) -> HashMap<usize, T> {
    map.iter()
        .map(|(addr, value)| {
            let pointer = *addr;
            let new_pointer = if pointer > address || (pointer >= address && ge) {
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
    ge: bool,
) -> HashMap<usize, usize> {
    map.iter()
        .map(|(source, destination)| {
            let new_source = adjust_pointer(*source, address, count, subtract);
            let new_destination = if *destination > address || (*destination >= address && ge) {
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
    pub fn new(endian: Endian) -> Self {
        BinArchive {
            data: Vec::new(),
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian,
        }
    }

    pub fn assert_equal_regions(
        &self,
        other: &BinArchive,
        source_start: usize,
        other_start: usize,
        length: usize,
    ) -> Result<()> {
        for i in (0..length).step_by(4) {
            let source_addr = source_start + i;
            let other_addr = other_start + i;

            // Compare text.
            if self.read_string(source_addr)? != other.read_string(other_addr)? {
                return Err(ArchiveError::ComparisonFailure(source_addr, other_addr));
            }
            let mut has_pointer_or_text = self.read_string(source_addr)?.is_some();

            // Compare pointers.
            // Can't reasonably compare pointer values, so we only look to make sure
            // that both cells either have or don't have a pointer.
            match self.read_pointer(source_addr)? {
                Some(_) => {
                    if other.read_pointer(other_addr)?.is_none() {
                        return Err(ArchiveError::ComparisonFailure(source_addr, other_addr));
                    } else {
                        has_pointer_or_text = true;
                    }
                }
                None => {
                    if other.read_pointer(other_addr)?.is_some() {
                        return Err(ArchiveError::ComparisonFailure(source_addr, other_addr));
                    }
                }
            }

            // Compare labels.
            if self.read_labels(source_addr)? != other.read_labels(other_addr)? {
                return Err(ArchiveError::ComparisonFailure(source_addr, other_addr));
            }

            // If this cell isn't a pointer of some kind, compare bytes.
            if !has_pointer_or_text && self.read_i32(source_addr)? != other.read_i32(other_addr)? {
                return Err(ArchiveError::ComparisonFailure(source_addr, other_addr));
            }
        }
        Ok(())
    }

    pub fn from_bytes(bytes: &[u8], endian: Endian) -> Result<Self> {
        if bytes.len() < 0x20 {
            return Err(ArchiveError::ArchiveTooSmall);
        }
        let mut cursor = Cursor::new(bytes);
        cursor.set_position(4);
        let data_size = cursor.read_u32(endian)?;
        let pointer_count = cursor.read_u32(endian)?;
        let label_count = cursor.read_u32(endian)?;
        let text_start = (data_size + (pointer_count * 4) + (label_count * 8)) as usize;
        if text_start + 0x20 > bytes.len() {
            return Err(ArchiveError::ArchiveTooSmall);
        }

        let mut archive = BinArchive::new(endian);
        cursor.seek(SeekFrom::Start(0x20))?;
        archive.data.resize(data_size as usize, 0);
        cursor.read_exact(&mut archive.data)?;
        for _ in 0..pointer_count {
            let pointer_address = cursor.read_u32(endian)? as usize;
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
            let address = cursor.read_u32(endian)?;
            let offset = cursor.read_u32(endian)? as usize;
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
        let mut cursor: Cursor<&mut [u8]> = Cursor::new(&mut data);

        let mut pointers: Vec<(usize, usize)> = self.pointers.clone().into_iter().collect();
        let mut cstrings: Vec<(&String, &Vec<usize>)> = self.cstrings.iter().collect();
        let mut labels: Vec<(&usize, &Vec<String>)> = self.labels.iter().collect();
        let mut text: Vec<(&usize, &String)> = self.text.iter().collect();

        let mut raw_cstrings: Vec<u8> = Vec::new();
        let mut offset_tracker: HashMap<String, usize> = HashMap::new();
        cstrings.sort_by(|a, b| a.0.cmp(b.0));
        for (text, addresses) in cstrings {
            let offset = add_text(&mut raw_cstrings, &mut offset_tracker, text)?;
            let text_address = self.data.len() + offset;
            for address in addresses {
                pointers.push((*address, text_address));
            }
        }
        while raw_cstrings.len() % 4 != 0 {
            raw_cstrings.push(0);
        }

        pointers.sort_by(|a, b| a.0.cmp(&b.0));
        for (source, destination) in pointers {
            cursor.seek(SeekFrom::Start(source as u64))?;
            cursor.write_u32(destination as u32, self.endian)?;
            raw_pointers.push(source as u32);
        }

        if let Endian::Big = self.endian {
            labels.sort_by(|a, b| a.1.cmp(b.1));
        } else {
            labels.sort_by(|a, b| a.0.cmp(b.0));
        }
        
        for (address, bucket) in labels {
            for label in bucket {
                let offset = add_text(&mut raw_text, &mut raw_text_offsets, label)?;
                raw_labels.push(*address as u32);
                raw_labels.push(offset as u32);
            }
        }

        
        text.sort_by(|a, b| a.0.cmp(b.0));
        let mut ptr_data_pairs: IndexMap<usize, Vec<u32>> = IndexMap::new();
        let text_start =
            self.data.len() + (self.pointers.len() + self.text.len() + raw_labels.len()) * 4;
        for (address, string) in text {
            let offset = add_text(&mut raw_text, &mut raw_text_offsets, string)?;
            let text_address = text_start + offset;
            cursor.seek(SeekFrom::Start(*address as u64))?;
            cursor.write_u32(text_address as u32, self.endian)?;
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
            + raw_cstrings.len()
            + (raw_pointers.len() * 4)
            + (raw_labels.len() * 4)
            + raw_text.len()
            + 0x20;
        bytes.resize(file_size, 0);
        let mut cursor: Cursor<&mut [u8]> = Cursor::new(&mut bytes);
        cursor.write_u32(file_size as u32, self.endian)?;
        cursor.write_u32(data.len() as u32 + raw_cstrings.len() as u32, self.endian)?;
        cursor.write_u32(raw_pointers.len() as u32, self.endian)?;
        cursor.write_u32((raw_labels.len() / 2) as u32, self.endian)?;
        cursor.seek(SeekFrom::Start(0x20))?;
        cursor.write(&data)?;
        cursor.write(&raw_cstrings)?;
        for pointer in raw_pointers {
            cursor.write_u32(pointer, self.endian)?;
        }
        for label_part in raw_labels {
            cursor.write_u32(label_part, self.endian)?;
        }
        cursor.write(&raw_text)?;
        Ok(bytes)
    }

    pub fn read_f32(&self, address: usize) -> Result<f32> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        Ok(self.endian.decode_f32(&self.data[address..address + 4])?)
    }

    pub fn read_u8(&self, address: usize) -> Result<u8> {
        validate_address(address, self.size(), false)?;
        Ok(self.data[address])
    }

    pub fn read_u16(&self, address: usize) -> Result<u16> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 2, self.size(), true)?;
        Ok(self.endian.decode_u16(&self.data[address..address + 2])?)
    }

    pub fn read_u32(&self, address: usize) -> Result<u32> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        Ok(self.endian.decode_u32(&self.data[address..address + 4])?)
    }

    pub fn read_i8(&self, address: usize) -> Result<i8> {
        validate_address(address, self.size(), false)?;
        Ok(self.data[address] as i8)
    }

    pub fn read_i16(&self, address: usize) -> Result<i16> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 2, self.size(), true)?;
        Ok(self.endian.decode_i16(&self.data[address..address + 2])?)
    }

    pub fn read_i32(&self, address: usize) -> Result<i32> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        Ok(self.endian.decode_i32(&self.data[address..address + 4])?)
    }

    pub fn read_bytes(&self, address: usize, amount: usize) -> Result<&[u8]> {
        validate_address(address, self.size(), false)?;
        validate_address(address + amount, self.size(), true)?;
        Ok(&self.data[address..(address + amount)])
    }

    pub fn read_string(&self, address: usize) -> Result<Option<String>> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        Ok(self.text.get(&address).map(|x| x.to_owned()))
    }

    pub fn read_pointer(&self, address: usize) -> Result<Option<usize>> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        Ok(self.pointers.get(&address).map(|x| x.to_owned()))
    }

    pub fn read_labels(&self, address: usize) -> Result<Option<Vec<String>>> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        Ok(self.labels.get(&address).map(|x| x.to_owned()))
    }

    pub fn read_c_string(&self, address: usize) -> Result<Option<String>> {
        if let Some(ptr) = self.read_pointer(address)? {
            validate_address(ptr, self.size(), false)?;
            let mut cursor: Cursor<&[u8]> = Cursor::new(&self.data);
            cursor.set_position(ptr as u64);
            let text = cursor.read_shift_jis_string()?;
            Ok(Some(text))
        } else {
            Ok(None)
        }
    }

    pub fn delete_string(&mut self, address: usize) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        self.text.remove(&address);
        Ok(())
    }

    pub fn delete_pointer(&mut self, address: usize) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        self.pointers.remove(&address);
        Ok(())
    }

    pub fn delete_labels(&mut self, address: usize) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        self.labels.remove(&address);
        Ok(())
    }

    pub fn delete_label(&mut self, address: usize, index: usize) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
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
        let bytes = self.endian.encode_f32(value);
        self.data[address..address + 4].copy_from_slice(&bytes);
        Ok(())
    }

    pub fn write_u8(&mut self, address: usize, value: u8) -> Result<()> {
        validate_address(address, self.size(), false)?;
        self.data[address] = value;
        Ok(())
    }

    pub fn write_u16(&mut self, address: usize, value: u16) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 2, self.size(), true)?;
        let bytes = self.endian.encode_u16(value);
        self.data[address..address + 2].copy_from_slice(&bytes);
        Ok(())
    }

    pub fn write_u32(&mut self, address: usize, value: u32) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        let bytes = self.endian.encode_u32(value);
        self.data[address..address + 4].copy_from_slice(&bytes);
        Ok(())
    }

    pub fn write_i8(&mut self, address: usize, value: i8) -> Result<()> {
        validate_address(address, self.size(), false)?;
        self.data[address] = value as u8;
        Ok(())
    }

    pub fn write_i16(&mut self, address: usize, value: i16) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 2, self.size(), true)?;
        let bytes = self.endian.encode_i16(value);
        self.data[address..address + 2].copy_from_slice(&bytes);
        Ok(())
    }

    pub fn write_i32(&mut self, address: usize, value: i32) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        let bytes = self.endian.encode_i32(value);
        self.data[address..address + 4].copy_from_slice(&bytes);
        Ok(())
    }

    pub fn write_bytes(&mut self, address: usize, bytes: &[u8]) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + bytes.len(), self.size(), true)?;
        self.data[address..(address + bytes.len())].copy_from_slice(bytes);
        Ok(())
    }

    pub fn write_c_string(&mut self, address: usize, value: String) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + 4, self.size(), true)?;
        let bucket = self.cstrings.entry(value).or_insert_with(|| Vec::new());
        bucket.push(address);
        Ok(())
    }

    pub fn write_string(&mut self, address: usize, value: Option<&str>) -> Result<()> {
        match value {
            Some(value) => {
                validate_address(address, self.size(), false)?;
                validate_address(address + 4, self.size(), true)?;
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
                self.pointers.insert(address, value);
                Ok(())
            }
            None => self.delete_pointer(address),
        }
    }

    pub fn write_labels(&mut self, address: usize, labels: Vec<String>) -> Result<()> {
        validate_address(address, self.size(), true)?;
        self.labels.insert(address, labels);
        Ok(())
    }

    pub fn write_label(&mut self, address: usize, label: &str) -> Result<()> {
        validate_address(address, self.size(), true)?;
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

    pub fn allocate(&mut self, address: usize, amount_in_bytes: usize, ge: bool) -> Result<()> {
        validate_address(address, self.size(), true)?;
        validate_alignment(address, 4)?;
        validate_alignment(amount_in_bytes, 4)?;
        let bytes_to_insert: Vec<u8> = vec![0; amount_in_bytes];
        self.data
            .splice(address..address, bytes_to_insert.iter().cloned());
        let new_text = adjust_text(&self.text, address, amount_in_bytes, false);
        let new_labels = adjust_labels(&self.labels, address, amount_in_bytes, false, ge);
        let new_pointers = adjust_pointers(&self.pointers, address, amount_in_bytes, false, ge);
        self.text = new_text;
        self.labels = new_labels;
        self.pointers = new_pointers;
        Ok(())
    }

    pub fn deallocate(&mut self, address: usize, amount_in_bytes: usize, ge: bool) -> Result<()> {
        validate_address(address, self.size(), false)?;
        validate_address(address + amount_in_bytes, self.size(), true)?;
        validate_alignment(address, 4)?;
        validate_alignment(amount_in_bytes, 4)?;
        self.data.drain(address..(address + amount_in_bytes));
        let filtered_text = filter_text_or_labels(&self.text, address, amount_in_bytes);
        let filtered_labels = filter_text_or_labels(&self.labels, address, amount_in_bytes);
        let filtered_pointers = filter_pointers(&self.pointers, address, amount_in_bytes);
        let new_text = adjust_text(&filtered_text, address, amount_in_bytes, true);
        let new_labels = adjust_labels(&filtered_labels, address, amount_in_bytes, true, ge);
        let new_pointers = adjust_pointers(&filtered_pointers, address, amount_in_bytes, true, ge);
        self.text = new_text;
        self.labels = new_labels;
        self.pointers = new_pointers;
        Ok(())
    }

    pub fn truncate(&mut self, address: usize) -> Result<()> {
        if address >= self.data.len() {
            return Ok(());
        }
        let range = address..self.data.len();
        self.data.drain(range.clone());
        for i in range.step_by(4) {
            self.text.remove(&i);
            self.labels.remove(&i);
            self.pointers.remove(&i);
        }
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

    pub fn pointer_destinations(&self) -> HashSet<usize> {
        self.pointers.values().map(|v| *v).collect()
    }

    pub fn all_labels(&self) -> Vec<(usize, String)> {
        let mut result: Vec<(usize, String)> = Vec::new();
        for (k, v) in &self.labels {
            for label in v {
                result.push((*k, label.clone()));
            }
        }
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::BinArchive;
    use crate::utils::load_test_file;
    use crate::Endian;
    use maplit::hashmap;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn size() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let archive2 = BinArchive::new(Endian::Little);
        assert_eq!(archive.size(), 4);
        assert_eq!(archive2.size(), 0);
    }

    #[test]
    fn assert_equal_regions_success() {
        let labels = vec!["Owain".to_string(), "Severa".to_string()];
        let source = BinArchive {
            data: vec![5, 0, 0, 1, 8, 0, 0, 0, 12, 0, 0, 0],
            text: hashmap! {
                4 => "Test".to_string()
            },
            pointers: hashmap! {
                8 => 4
            },
            labels: hashmap! {
                0 => vec!["Assessment".to_string()],
                4 => labels.clone()
            },
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let other = BinArchive {
            data: vec![0, 0, 0, 0, 5, 0, 0, 1, 4, 12, 0, 1, 16, 12, 0, 2],
            text: hashmap! {
                8 => "Test".to_string()
            },
            pointers: hashmap! {
                12 => 0
            },
            labels: hashmap! {
                4 => vec!["Assessment".to_string()],
                8 => labels.clone()
            },
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };

        assert!(source.assert_equal_regions(&other, 0, 4, 12).is_ok());
    }

    #[test]
    fn assert_equal_regions_bytes_mismatch() {
        let source = BinArchive {
            data: vec![0, 0, 0, 0, 12, 1, 8, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let other = BinArchive {
            data: vec![12, 1, 7, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };

        assert!(source.assert_equal_regions(&other, 4, 0, 4).is_err());
    }

    #[test]
    fn assert_equal_regions_pointer_mismatch() {
        let source = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let other = BinArchive {
            data: vec![0, 0, 0, 0],
            text: HashMap::new(),
            pointers: hashmap! {
                0 => 4
            },
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };

        assert!(source.assert_equal_regions(&other, 4, 0, 4).is_err());
    }

    #[test]
    fn assert_equal_regions_string_mismatch() {
        let source = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0],
            text: hashmap! {
                4 => "Test".to_string()
            },
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let other = BinArchive {
            data: vec![0, 0, 0, 0],
            text: hashmap! {
                0 => "Exam".to_string()
            },
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };

        assert!(source.assert_equal_regions(&other, 4, 0, 4).is_err());
    }

    #[test]
    fn assert_equal_regions_label_mismatch() {
        let source = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: hashmap! {
                4 => vec!["Severa".to_string()]
            },
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let other = BinArchive {
            data: vec![0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: hashmap! {
                0 => vec!["Selena".to_string()]
            },
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };

        assert!(source.assert_equal_regions(&other, 4, 0, 4).is_err());
    }

    #[test]
    fn get_labels() {
        let labels = vec!["Owain".to_string(), "Severa".to_string()];
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: hashmap! {
                0 => vec!["Test".to_string()],
                4 => labels.clone()
            },
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let labels = archive.get_labels();
        assert_eq!(
            labels,
            vec![
                (0, "Test".to_string()),
                (4, "Owain".to_string()),
                (4, "Severa".to_string())
            ]
        );
    }

    #[test]
    fn read_f32() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0x3F, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.read_f32(4);
        let result2 = archive.read_f32(8);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), 0.5);
        assert!(result2.is_err());
    }

    #[test]
    fn read_u8() {
        let archive = BinArchive {
            data: vec![0, 23],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.read_u8(1);
        let result2 = archive.read_u8(2);
        assert!(result1.is_ok());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.read_u16(2);
        let result2 = archive.read_u16(8);
        let result3 = archive.read_u16(4);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), 0xFE14);
        assert!(result2.is_err());
        assert!(result3.is_err());
    }

    #[test]
    fn read_u32() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0x14, 0xFE, 0x15, 0xFE, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.read_u32(4);
        let result2 = archive.read_u32(8);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), 0xFE15FE14);
        assert!(result2.is_err());
    }

    #[test]
    fn read_i8() {
        let archive = BinArchive {
            data: vec![0, 23],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.read_i8(1);
        let result2 = archive.read_i8(2);
        assert!(result1.is_ok());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.read_i16(2);
        let result2 = archive.read_i16(8);
        let result3 = archive.read_i16(4);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), 0x1112);
        assert!(result2.is_err());
        assert!(result3.is_err());
    }

    #[test]
    fn read_i32() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0x14, 0x11, 0x15, 0x11, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.read_u32(4);
        let result2 = archive.read_u32(8);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), 0x11151114);
        assert!(result2.is_err());
    }

    #[test]
    fn read_bytes() {
        let archive = BinArchive {
            data: vec![0, 0x14, 0x11, 0x15, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.read_string(4);
        let result2 = archive.read_string(8);
        let result3 = archive.read_string(12);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), Some("test".to_string()));
        assert!(result2.is_err());
        assert!(result3.is_err());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.read_pointer(4);
        let result2 = archive.read_pointer(8);
        let result3 = archive.read_pointer(12);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), Some(0));
        assert!(result2.is_err());
        assert!(result3.is_err());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.read_labels(4);
        let result2 = archive.read_labels(8);
        let result3 = archive.read_labels(12);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), Some(labels));
        assert!(result2.is_err());
        assert!(result3.is_err());
    }

    #[test]
    fn read_c_string() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0x4, 0x41, 0x42, 0x43, 0x0, 0x0, 0x0, 0x0, 0x0],
            text: HashMap::new(),
            pointers: hashmap! {
                0 => 4,
            },
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Big,
        };
        let expected = Some(String::from("ABC"));
        let result1 = archive.read_c_string(0);
        let result2 = archive.read_c_string(8);
        let result3 = archive.read_c_string(100);
        assert!(result1.is_ok());
        assert_eq!(expected, result1.unwrap());
        assert!(result2.is_ok());
        assert_eq!(None, result2.unwrap());
        assert!(result3.is_err());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: HashMap<usize, String> = HashMap::new();
        let result1 = archive.delete_string(4);
        let result2 = archive.delete_string(8);
        let result3 = archive.delete_string(12);
        assert!(result1.is_ok());
        assert_eq!(archive.text, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: HashMap<usize, usize> = HashMap::new();
        let result1 = archive.delete_pointer(4);
        let result2 = archive.delete_pointer(8);
        let result3 = archive.delete_pointer(12);
        assert!(result1.is_ok());
        assert_eq!(archive.pointers, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: HashMap<usize, Vec<String>> = HashMap::new();
        let result1 = archive.delete_labels(4);
        let result2 = archive.delete_labels(8);
        let result3 = archive.delete_labels(12);
        assert!(result1.is_ok());
        assert_eq!(archive.labels, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: Vec<u8> = vec![0, 0, 0, 0, 0, 0, 0, 0x3F, 0];
        let result1 = archive.write_f32(4, 0.5);
        let result2 = archive.write_f32(9, 0.5);
        let result3 = archive.write_f32(8, 0.5);
        assert!(result1.is_ok());
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
    }

    #[test]
    fn write_u8() {
        let mut archive = BinArchive {
            data: vec![0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: Vec<u8> = vec![0, 0x23];
        let result1 = archive.write_u8(1, 0x23);
        let result2 = archive.write_u8(2, 0x23);
        assert!(result1.is_ok());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: Vec<u8> = vec![0, 0, 0x12, 0x11, 0];
        let result1 = archive.write_u16(2, 0x1112);
        let result2 = archive.write_u16(8, 0x1112);
        let result3 = archive.write_u16(4, 0x1112);
        assert!(result1.is_ok());
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
    }

    #[test]
    fn write_u32() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: Vec<u8> = vec![0, 0, 0, 0, 0x12, 0x11, 0x22, 0x23, 0];
        let result1 = archive.write_u32(4, 0x23221112);
        let result2 = archive.write_u32(8, 0x23221112);
        assert!(result1.is_ok());
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
    }

    #[test]
    fn write_i8() {
        let mut archive = BinArchive {
            data: vec![0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: Vec<u8> = vec![0, 0x23];
        let result1 = archive.write_i8(1, 0x23);
        let result2 = archive.write_i8(2, 0x23);
        assert!(result1.is_ok());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: Vec<u8> = vec![0, 0, 0x12, 0x11, 0];
        let result1 = archive.write_i16(2, 0x1112);
        let result2 = archive.write_i16(8, 0x1112);
        let result3 = archive.write_i16(4, 0x1112);
        assert!(result1.is_ok());
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
        assert!(result3.is_err());
    }

    #[test]
    fn write_i32() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: Vec<u8> = vec![0, 0, 0, 0, 0x12, 0x11, 0x22, 0x23, 0];
        let result1 = archive.write_i32(4, 0x23221112);
        let result2 = archive.write_i32(8, 0x23221112);
        assert!(result1.is_ok());
        assert_eq!(archive.data, expected);
        assert!(result2.is_err());
    }

    #[test]
    fn write_bytes() {
        let bytes: Vec<u8> = vec![0xFE, 0xFF];
        let mut archive = BinArchive {
            data: vec![0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: Vec<u8> = vec![0, 0xFE, 0xFF];
        let result1 = archive.write_bytes(1, &bytes);
        let result2 = archive.write_bytes(2, &bytes);
        let result3 = archive.write_bytes(3, &bytes);
        assert!(result1.is_ok());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: HashMap<usize, String> = hashmap! {
            4 => "test".to_string()
        };
        let result1 = archive.write_string(4, Some("test"));
        let result2 = archive.write_string(8, Some("test"));
        assert!(result1.is_ok());
        assert_eq!(archive.text, expected);
        assert!(result2.is_err());
    }

    #[test]
    fn write_pointer() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: HashMap<usize, usize> = hashmap! {
            4 => 0
        };
        let result1 = archive.write_pointer(4, Some(0));
        let result2 = archive.write_pointer(8, Some(0));
        assert!(result1.is_ok());
        assert_eq!(archive.pointers, expected);
        assert!(result2.is_err());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let expected: HashMap<usize, Vec<String>> = hashmap! {
            4 => labels.clone(),
            8 => labels.clone(),
        };
        let result1 = archive.write_labels(4, labels.clone());
        let result2 = archive.write_labels(8, labels.clone());
        assert!(result1.is_ok());
        assert_eq!(archive.labels, expected);
        assert!(result2.is_ok());
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
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
    fn pointer_destinations() {
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: hashmap! {
                4 => 0,
                0 => 4,
                8 => 0
            },
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };

        let mut expected = HashSet::new();
        expected.insert(0);
        expected.insert(4);
        assert_eq!(archive.pointer_destinations(), expected);
    }

    #[test]
    fn all_labels() {
        let expected: Vec<(usize, String)> = vec![
            (0, "test".to_string()),
            (4, "Owain".to_string()),
            (4, "Severa".to_string()),
            (8, "Selena".to_string()),
        ];
        let archive = BinArchive {
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: hashmap! {
                0 => vec!["test".to_string()],
                4 => vec![
                    "Owain".to_string(),
                    "Severa".to_string(),
                ],
                8 => vec!["Selena".to_string()]
            },
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        assert_eq!(archive.all_labels(), expected);
    }

    #[test]
    fn allocate_at_end() {
        let mut archive = BinArchive {
            data: vec![0, 0, 0, 0],
            text: HashMap::new(),
            pointers: HashMap::new(),
            labels: HashMap::new(),
            cstrings: HashMap::new(),
            endian: Endian::Little,
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
            cstrings: HashMap::new(),
            endian: Endian::Little,
        };
        let result1 = archive.allocate(2, 4, false);
        let result2 = archive.allocate(0, 3, false);
        let result3 = archive.allocate(8, 4, false);
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
    fn deallocate_mixed2() {
        test_deallocation(
            "ArchiveTest_Mixed2.bin",
            "ArchiveTest_Deallocate_Mixed2.bin",
            36,
            48,
        );
    }

    #[test]
    fn allocate_no_label_shift() {
        let bytes = load_test_file("Allocate_NoLabelShift.bin");
        let mut archive = BinArchive::from_bytes(&bytes, Endian::Little).unwrap();
        assert!(archive.allocate(0x8, 0x10, false).is_ok());
        assert_eq!(
            archive.read_labels(0x8).unwrap().unwrap(),
            vec!("TEST".to_string())
        );
        assert_eq!(
            archive.read_labels(0x1C).unwrap().unwrap(),
            vec!("TEST2".to_string())
        );
        assert!(archive.read_labels(0xC).unwrap().is_none());
    }

    #[test]
    fn allocate_no_destination_shift() {
        let bytes = load_test_file("Allocate_NoDestinationShift.bin");
        let mut archive = BinArchive::from_bytes(&bytes, Endian::Little).unwrap();
        assert!(archive.allocate(0x10, 0x10, false).is_ok());
        assert_eq!(archive.read_pointer(0x8).unwrap().unwrap(), 0x10);
        assert!(archive.allocate(0xC, 0x10, false).is_ok());
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
        let result = BinArchive::from_bytes(&bytes, Endian::Little);
        assert!(result.is_ok());
        let mut archive = result.unwrap();
        let result = archive.allocate(address, count, false);
        assert!(result.is_ok());
        let result = archive.serialize();
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(bytes, expected);
    }

    fn test_deallocation(
        source_file_name: &str,
        result_file_name: &str,
        address: usize,
        count: usize,
    ) {
        let bytes = load_test_file(source_file_name);
        let expected = load_test_file(result_file_name);
        let result = BinArchive::from_bytes(&bytes, Endian::Little);
        assert!(result.is_ok());
        let mut archive = result.unwrap();
        let result = archive.deallocate(address, count, false);
        assert!(result.is_ok());
        let result = archive.serialize();
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(bytes, expected);
    }

    fn test_archive_for_success(file_name: &str) {
        let bytes = load_test_file(file_name);
        let result = BinArchive::from_bytes(&bytes, Endian::Little);
        assert!(result.is_ok());
        let result = result.unwrap().serialize();
        assert!(result.is_ok());
        let result_bytes = result.unwrap();
        assert_eq!(result_bytes, bytes);
    }

    fn test_archive_for_error(file_name: &str) {
        let bytes = load_test_file(file_name);
        let result = BinArchive::from_bytes(&bytes, Endian::Little);
        assert!(result.is_err());
    }
}
