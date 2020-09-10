use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("Input is not compressed using {0}.")]
    InvalidInput(String)
}

#[derive(Error, Debug)]
pub enum EncodedStringsError {
    #[error("Fell out of buffer while reading a null-terminated string.")]
    UnterminatedString,

    #[error("Failed to encode string {0} with encoding {1}.")]
    EncodingFailed(String, String),

    #[error("Unable to decode {0} string.")]
    DecodingFailed(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("The archive size specified in the header is incorrect.")]
    SizeMismatch,

    #[error("The archive is not big enough to support the size required by the header.")]
    ArchiveTooSmall,

    #[error("Out of bounds address '0x{0:x}' with archive of size '0x{1:x}'.")]
    OutOfBoundsAddress(usize, usize),

    #[error("Unaligned value '{0}' should be aligned to {1} bytes.")]
    UnalignedValue(usize, usize),

    #[error("Index '{1}' is out of bounds for label bucket of size '{0}'.")]
    LabelIndexOutOfBounds(usize, usize),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    EncodingStringsError(#[from] EncodedStringsError),
}