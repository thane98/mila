use byteorder::ReadBytesExt;
use encoding_rs::SHIFT_JIS;
use std::io::Cursor;
use crate::errors::EncodedStringsError;

type Result<T> = std::result::Result<T, EncodedStringsError>;

pub trait EncodedStringReader {
    fn read_shift_jis_string(&mut self) -> Result<String>;
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

impl EncodedStringReader for Cursor<&[u8]> {
    fn read_shift_jis_string(&mut self) -> Result<String> {
        read_shift_jis_impl(|| self.read_u8())
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
