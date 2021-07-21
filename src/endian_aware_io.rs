use std::{convert::TryFrom, io::{Cursor, Read, Write}};

use crate::EndianAwareIOError;

type Result<T> = std::result::Result<T, EndianAwareIOError>;

#[derive(Debug, Clone, Copy)]
pub enum Endian {
    Little,
    Big,
}

pub trait EndianAwareReader {
    fn read_u16(&mut self, endian: Endian) -> Result<u16>;

    fn read_u32(&mut self, endian: Endian) -> Result<u32>;
}

pub trait EndianAwareWriter {
    fn write_u16(&mut self, value: u16, endian: Endian) -> Result<()>;

    fn write_u32(&mut self, value: u32, endian: Endian) -> Result<()>;
}

impl Endian {
    pub fn decode_u16(&self, bytes: &[u8]) -> Result<u16> {
        let arr = <[u8; 2]>::try_from(bytes).map_err(|_| EndianAwareIOError::ConversionError)?;
        Ok(match self {
            Endian::Little => u16::from_le_bytes(arr),
            Endian::Big => u16::from_be_bytes(arr),
        })
    }

    pub fn decode_u32(&self, bytes: &[u8]) -> Result<u32> {
        let arr = <[u8; 4]>::try_from(bytes).map_err(|_| EndianAwareIOError::ConversionError)?;
        Ok(match self {
            Endian::Little => u32::from_le_bytes(arr),
            Endian::Big => u32::from_be_bytes(arr),
        })
    }

    pub fn decode_i16(&self, bytes: &[u8]) -> Result<i16> {
        let arr = <[u8; 2]>::try_from(bytes).map_err(|_| EndianAwareIOError::ConversionError)?;
        Ok(match self {
            Endian::Little => i16::from_le_bytes(arr),
            Endian::Big => i16::from_be_bytes(arr),
        })
    }

    pub fn decode_i32(&self, bytes: &[u8]) -> Result<i32> {
        let arr = <[u8; 4]>::try_from(bytes).map_err(|_| EndianAwareIOError::ConversionError)?;
        Ok(match self {
            Endian::Little => i32::from_le_bytes(arr),
            Endian::Big => i32::from_be_bytes(arr),
        })
    }

    pub fn decode_f32(&self, bytes: &[u8]) -> Result<f32> {
        let arr = <[u8; 4]>::try_from(bytes).map_err(|_| EndianAwareIOError::ConversionError)?;
        Ok(match self {
            Endian::Little => f32::from_le_bytes(arr),
            Endian::Big => f32::from_be_bytes(arr),
        })
    }

    pub fn encode_u16(&self, value: u16) -> Vec<u8> {
        match self {
            Endian::Little => value.to_le_bytes().to_vec(),
            Endian::Big => value.to_be_bytes().to_vec(),
        }
    }

    pub fn encode_u32(&self, value: u32) -> Vec<u8> {
        match self {
            Endian::Little => value.to_le_bytes().to_vec(),
            Endian::Big => value.to_be_bytes().to_vec(),
        }
    }

    pub fn encode_i16(&self, value: i16) -> Vec<u8> {
        match self {
            Endian::Little => value.to_le_bytes().to_vec(),
            Endian::Big => value.to_be_bytes().to_vec(),
        }
    }

    pub fn encode_i32(&self, value: i32) -> Vec<u8> {
        match self {
            Endian::Little => value.to_le_bytes().to_vec(),
            Endian::Big => value.to_be_bytes().to_vec(),
        }
    }

    pub fn encode_f32(&self, value: f32) -> Vec<u8> {
        match self {
            Endian::Little => value.to_le_bytes().to_vec(),
            Endian::Big => value.to_be_bytes().to_vec(),
        }
    }
}

impl EndianAwareReader for Cursor<&[u8]> {
    fn read_u16(&mut self, endian: Endian) -> Result<u16> {
        let mut buf: Vec<u8> = vec![0; 2];
        self.read_exact(&mut buf)?;
        endian.decode_u16(&buf)
    }

    fn read_u32(&mut self, endian: Endian) -> Result<u32> {
        let mut buf: Vec<u8> = vec![0; 4];
        self.read_exact(&mut buf)?;
        endian.decode_u32(&buf)
    }
}

impl EndianAwareWriter for Cursor<&mut [u8]> {
    fn write_u16(&mut self, value: u16, endian: Endian) -> Result<()> {
        self.write(&endian.encode_u16(value))?;
        Ok(())
    }

    fn write_u32(&mut self, value: u32, endian: Endian) -> Result<()> {
        self.write(&endian.encode_u32(value))?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn decode_u16() {
        assert_eq!(
            0xFE14,
            Endian::Little.decode_u16(&vec![0x14, 0xFE]).unwrap()
        );
        assert_eq!(0xFE14, Endian::Big.decode_u16(&vec![0xFE, 0x14]).unwrap());
    }

    #[test]
    fn decode_u32() {
        assert_eq!(
            0xFE131415,
            Endian::Little
                .decode_u32(&vec![0x15, 0x14, 0x13, 0xFE])
                .unwrap()
        );
        assert_eq!(
            0xFE131415,
            Endian::Big
                .decode_u32(&vec![0xFE, 0x13, 0x14, 0x15])
                .unwrap()
        );
    }

    #[test]
    fn decode_i16() {
        assert_eq!(
            0x1314,
            Endian::Little.decode_i16(&vec![0x14, 0x13]).unwrap()
        );
        assert_eq!(0x1314, Endian::Big.decode_i16(&vec![0x13, 0x14]).unwrap());
    }

    #[test]
    fn decode_i32() {
        assert_eq!(
            0x11121314,
            Endian::Little.decode_i32(&vec![0x14, 0x13, 0x12, 0x11]).unwrap()
        );
        assert_eq!(
            0x11121314,
            Endian::Big.decode_i32(&vec![0x11, 0x12, 0x13, 0x14]).unwrap()
        );
    }

    #[test]
    fn decode_f32() {
        assert_eq!(
            0.5,
            Endian::Little.decode_f32(&vec![0x00, 0x00, 0x00, 0x3F]).unwrap()
        );
        assert_eq!(
            0.5,
            Endian::Big.decode_f32(&vec![0x3F, 0x00, 0x00, 0x00]).unwrap()
        );
    }

    #[test]
    fn encode_u16() {
        assert_eq!(vec![0x14, 0xFE], Endian::Little.encode_u16(0xFE14));
        assert_eq!(vec![0xFE, 0x14], Endian::Big.encode_u16(0xFE14));
    }

    #[test]
    fn encode_u32() {
        assert_eq!(vec![0x13, 0x12, 0x14, 0xFE], Endian::Little.encode_u32(0xFE141213));
        assert_eq!(vec![0xFE, 0x14, 0x12, 0x13], Endian::Big.encode_u32(0xFE141213));
    }

    #[test]
    fn encode_i16() {
        assert_eq!(vec![0x12, 0x11], Endian::Little.encode_i16(0x1112));
        assert_eq!(vec![0x11, 0x12], Endian::Big.encode_i16(0x1112));
    }

    #[test]
    fn encode_i32() {
        assert_eq!(vec![0x15, 0x14, 0x13, 0x12], Endian::Little.encode_i32(0x12131415));
        assert_eq!(vec![0x12, 0x13, 0x14, 0x15], Endian::Big.encode_i32(0x12131415));
    }

    #[test]
    fn encode_f32() {
        assert_eq!(vec![0x00, 0x00, 0x00, 0x3F], Endian::Little.encode_f32(0.5));
        assert_eq!(vec![0x3F, 0x00, 0x00, 0x00], Endian::Big.encode_f32(0.5));
    }
}
