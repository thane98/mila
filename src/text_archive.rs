use crate::encoded_strings::{to_shift_jis, to_utf_16};
use crate::{BinArchive, BinArchiveReader, EncodedStringReader, TextArchiveError};
use linked_hash_map::LinkedHashMap;

type Result<T> = std::result::Result<T, TextArchiveError>;

fn write_shift_jis_string(bytes: &mut Vec<u8>, string: &str) -> Result<()> {
    bytes.extend(to_shift_jis(string)?);
    bytes.push(0);
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
    Ok(())
}

fn write_utf_16_string(bytes: &mut Vec<u8>, string: &str) -> Result<()> {
    bytes.extend(to_utf_16(string)?);
    bytes.push(0);
    bytes.push(0);
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
    Ok(())
}

pub struct TextArchive {
    title: String,
    entries: LinkedHashMap<String, String>,
    dirty: bool,
}

impl TextArchive {
    pub fn new() -> Self {
        TextArchive {
            title: "".to_string(),
            entries: LinkedHashMap::new(),
            dirty: false,
        }
    }

    pub fn get_entries(&self) -> &LinkedHashMap<String, String> {
        &self.entries
    }

    pub fn from_bytes(raw_archive: &[u8]) -> Result<Self> {
        let bin_archive = BinArchive::from_bytes(raw_archive)?;
        TextArchive::from_archive(&bin_archive)
    }

    pub fn from_archive(archive: &BinArchive) -> Result<Self> {
        let mut reader = BinArchiveReader::new(archive, 0);
        let mut text_archive = TextArchive::new();
        text_archive.title = reader.read_shift_jis_string()?;
        while reader.tell() < archive.size() {
            let labels = reader.read_labels()?.unwrap_or_else(Vec::new);
            let message = reader.read_utf_16_string()?;
            match labels.first() {
                Some(k) => {
                    text_archive.entries.insert(k.clone(), message);
                }
                _ => {}
            }
        }
        Ok(text_archive)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut bytes: Vec<u8> = Vec::new();
        let mut label_info: Vec<(&String, usize)> = Vec::new();
        write_shift_jis_string(&mut bytes, &self.title)?;
        for (key, value) in &self.entries {
            label_info.push((key, bytes.len()));
            write_utf_16_string(&mut bytes, value)?;
        }

        let mut archive = BinArchive::new();
        archive.allocate_at_end(bytes.len());
        archive.write_bytes(0, &bytes)?;
        for (label, address) in label_info {
            archive.write_label(address, label)?;
        }
        let bytes = archive.serialize()?;
        Ok(bytes)
    }

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, new_title: String) {
        self.title = new_title;
    }

    pub fn has_message(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn delete_message(&mut self, key: &str) {
        self.entries.remove(key);
    }

    pub fn get_message(&self, key: &str) -> Option<String> {
        match self.entries.get(key) {
            Some(value) => Some(value.replace("\n", "\\n")),
            None => None,
        }
    }

    pub fn set_message(&mut self, key: &str, message: &str) {
        let message = message.replace("\\n", "\n");
        let entry = self.entries.entry(key.to_string()).or_insert(String::new());
        *entry = message;
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::load_test_file;

    #[test]
    fn round_trip_serialization() {
        let bytes = load_test_file("TextArchive_Test.bin");
        let result = TextArchive::from_bytes(&bytes);
        assert!(result.is_ok());
        let text_archive = result.unwrap();
        let result = text_archive.serialize();
        assert!(result.is_ok());
        let serialized_bytes = result.unwrap();
        assert_eq!(serialized_bytes, bytes);
    }

    #[test]
    fn get_message() {
        let mut text_archive = TextArchive::new();
        text_archive.entries.insert(
            "my_key".to_string(),
            "My message\nhas newlines\n.".to_string(),
        );
        let message = text_archive.get_message("my_key");
        assert!(message.is_some());
        assert_eq!(message.unwrap(), "My message\\nhas newlines\\n.");
    }

    #[test]
    fn set_message() {
        let mut text_archive = TextArchive::new();
        text_archive.set_message("my_key", "My message\\nhas newlines\\n.");
        assert!(text_archive.is_dirty());
        let message = text_archive.entries.get("my_key");
        assert!(message.is_some());
        assert_eq!(message.unwrap(), "My message\nhas newlines\n.");
    }

    #[test]
    fn set_message_does_not_reorder_keys() {
        let mut archive = TextArchive::new();
        archive.entries.insert("Key1".to_string(), "Value1".to_string());
        archive.entries.insert("Key2".to_string(), "Value2".to_string());

        let keys: Vec<String> = archive.entries.keys().cloned().collect();
        assert_eq!(vec!["Key1".to_string(), "Key2".to_string()], keys);

        archive.set_message("Key1", "NewValue");
        let message = archive.entries.get("Key1");
        assert!(message.is_some());
        assert_eq!(message.unwrap(), "NewValue");

        let keys: Vec<String> = archive.entries.keys().cloned().collect();
        assert_eq!(vec!["Key1".to_string(), "Key2".to_string()], keys);
    }
}
