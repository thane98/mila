use byteorder::ReadBytesExt;
use encoding_rs::{SHIFT_JIS, UTF_16LE};
use std::io::Cursor;
use crate::EncodedStringsError;

type Result<T> = std::result::Result<T, EncodedStringsError>;

pub trait EncodedStringReader {
    fn read_shift_jis_string(&mut self) -> Result<String>;

    fn read_utf_16_string(&mut self) -> Result<String>;
}

fn read_shift_jis_impl<F, E>(mut read_u8: F) -> Result<String>
where
    F: FnMut() -> std::result::Result<u8, E>,
{
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        match read_u8() {
            Ok(value) => {
                if value == 0 {
                    break;
                } else {
                    buffer.push(value);
                }
            }
            Err(_) => return Err(EncodedStringsError::UnterminatedString),
        }
    }
    let (result, _, has_errors) = SHIFT_JIS.decode(buffer.as_slice());
    if has_errors {
        Err(EncodedStringsError::DecodingFailed("SHIFT-JIS".to_string()))
    } else {
        Ok(result.into())
    }
}

fn read_utf_16_impl<F, E: std::fmt::Debug>(mut read_u8: F) -> Result<String>
where
    F: FnMut() -> std::result::Result<u8, E>,
{
    let mut buffer: Vec<u8> = Vec::new();
    loop {
        let next_byte_result1 = read_u8();
        let next_byte_result2 = read_u8();
        if next_byte_result1.is_err() || next_byte_result2.is_err() {
            return Err(EncodedStringsError::UnterminatedString);
        }
        let next_byte1 = next_byte_result1.unwrap();
        let next_byte2 = next_byte_result2.unwrap();
        if next_byte1 == 0 && next_byte2 == 0 {
            break;
        }
        buffer.push(next_byte1);
        buffer.push(next_byte2);
    }

    let (result, _enc, errors) = UTF_16LE.decode(buffer.as_slice());
    return if errors {
        Err(EncodedStringsError::DecodingFailed("UTF-16".to_string()))
    } else {
        Ok(result.into())
    };
}

impl EncodedStringReader for Cursor<&[u8]> {
    fn read_shift_jis_string(&mut self) -> Result<String> {
        read_shift_jis_impl(|| self.read_u8())
    }

    fn read_utf_16_string(&mut self) -> Result<String> {
        read_utf_16_impl(|| self.read_u8())
    }
}

impl<'a> EncodedStringReader for crate::BinArchiveReader<'a> {
    fn read_shift_jis_string(&mut self) -> Result<String> {
        let result = read_shift_jis_impl(|| self.read_u8())?;
        while self.tell() % 4 != 0 {
            self.skip(1);
        }
        Ok(result)
    }

    fn read_utf_16_string(&mut self) -> Result<String> {
        let result = read_utf_16_impl(|| self.read_u8())?;
        while self.tell() % 4 != 0 {
            self.skip(1);
        }
        Ok(result)
    }
}

pub fn to_shift_jis(string: &str) -> Result<Vec<u8>> {
    let (result, _, has_errors) = SHIFT_JIS.encode(string);
    if has_errors {
        Err(EncodedStringsError::EncodingFailed(
            string.to_string(),
            "SHIFT-JIS".to_string(),
        ))
    } else {
        Ok(result.into())
    }
}

pub fn to_utf_16(string: &str) -> Result<Vec<u8>> {
    let bytes: Vec<[u8; 2]> = string.encode_utf16().map(|x| x.to_le_bytes()).collect();
    let mut buffer: Vec<u8> = Vec::new();
    for entry in bytes {
        buffer.push(entry[0]);
        buffer.push(entry[1]);
    }
    Ok(buffer)
}
