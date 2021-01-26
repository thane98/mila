use crate::{BinArchive, ArchiveError};

type Result<T> = std::result::Result<T, ArchiveError>;

pub struct BinArchiveReader<'a> {
    archive: &'a BinArchive,
    position: usize
}

pub struct BinArchiveWriter<'a> {
    archive: &'a mut BinArchive,
    position: usize
}

impl<'a> BinArchiveReader<'a> {
    pub fn new(archive: &'a BinArchive, position: usize) -> Self {
        BinArchiveReader {
            archive,
            position
        }
    }

    pub fn archive(&self) -> &'a BinArchive {
        self.archive
    }

    pub fn seek(&mut self, position: usize) {
        self.position = position;
    }

    pub fn skip(&mut self, amount: usize) {
        self.position += amount;
    }

    pub fn tell(&self) -> usize {
        return self.position;
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        let value = self.archive.read_u8(self.position)?;
        self.position += 1;
        Ok(value)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let value = self.archive.read_u16(self.position)?;
        self.position += 2;
        Ok(value)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let value = self.archive.read_u32(self.position)?;
        self.position += 4;
        Ok(value)
    }

    pub fn read_i8(&mut self) -> Result<i8> {
        let value = self.read_u8()?;
        Ok(value as i8)
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        let value = self.read_u16()?;
        Ok(value as i16)
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        let value = self.read_u32()?;
        Ok(value as i32)
    }

    pub fn read_bytes(&mut self, count: usize) -> Result<Vec<u8>> {
        let mut result: Vec<u8> = Vec::new();
        for _ in 0..count {
            result.push(self.read_u8()?);
        }
        Ok(result)
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        let value = self.archive.read_f32(self.position)?;
        self.position += 4;
        Ok(value)
    }

    pub fn read_string(&mut self) -> Result<Option<String>> {
        let value = self.archive.read_string(self.position)?;
        self.position += 4;
        Ok(value)
    }

    pub fn read_label(&mut self, index: usize) -> Result<Option<String>> {
        Ok(match self.archive.read_labels(index)? {
            Some(bucket) => bucket.first().map(|x| x.to_owned()),
            None => None
        })
    }

    pub fn read_labels(&mut self) -> Result<Option<Vec<String>>> {
        self.archive.read_labels(self.position)
    }

    pub fn read_pointer(&mut self) -> Result<Option<usize>> {
        let value = self.archive.read_pointer(self.position)?;
        self.position += 4;
        Ok(value)
    }
}

impl<'a> BinArchiveWriter<'a> {
    pub fn new(archive: &'a mut BinArchive, position: usize) -> Self {
        BinArchiveWriter {
            archive,
            position
        }
    }

    pub fn size(&self) -> usize {
        self.archive.size()
    }

    pub fn seek(&mut self, position: usize) {
        self.position = position;
    }

    pub fn skip(&mut self, amount: usize) {
        self.position += amount;
    }

    pub fn tell(&self) -> usize {
        return self.position;
    }

    pub fn length(&self) -> usize {
        return self.archive.size()
    }

    pub fn allocate(&mut self, amount: usize) -> Result<()> {
        if self.position == self.archive.size() {
            self.archive.allocate_at_end(amount);
        } else {
            self.archive.allocate(self.position, amount)?;
        }
        Ok(())
    }

    pub fn allocate_at_end(&mut self, amount: usize) {
        self.archive.allocate_at_end(amount)
    }

    pub fn write_u8(&mut self, value: u8) -> Result<()> {
        self.archive.write_u8(self.position, value)?;
        self.position += 1;
        Ok(())
    }

    pub fn write_u16(&mut self, value: u16) -> Result<()> {
        self.archive.write_u16(self.position, value)?;
        self.position += 2;
        Ok(())
    }

    pub fn write_u32(&mut self, value: u32) -> Result<()> {
        self.archive.write_u32(self.position, value)?;
        self.position += 4;
        Ok(())
    }

    pub fn write_i8(&mut self, value: i8) -> Result<()> {
        self.write_u8(value as u8)
    }

    pub fn write_i16(&mut self, value: i16) -> Result<()> {
        self.write_u16(value as u16)
    }

    pub fn write_i32(&mut self, value: i32) -> Result<()> {
        self.write_u32(value as u32)
    }

    pub fn write_bytes(&mut self, value: &[u8]) -> Result<()> {
        for byte in value {
            self.write_u8(*byte)?;
        }
        Ok(())
    }

    pub fn write_f32(&mut self, value: f32) -> Result<()> {
        self.archive.write_f32(self.position, value)?;
        self.position += 4;
        Ok(())
    }

    pub fn write_string(&mut self, value: Option<&str>) -> Result<()> {
        self.archive.write_string(self.position, value)?;
        self.position += 4;
        Ok(())
    }

    pub fn write_label(&mut self, value: &str) -> Result<()> {
        self.archive.write_label(self.position, value)
    }

    pub fn write_pointer(&mut self, value: Option<usize>) -> Result<()> {
        self.archive.write_pointer(self.position, value)?;
        self.position += 4;
        Ok(())
    }
}
