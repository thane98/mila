use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("Input is not compressed using {0}.")]
    InvalidInput(String),
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
    #[error("Files do not match at source address '0x{0:X}' other address '0x{1:X}'.")]
    ComparisonFailure(usize, usize),

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
    EndianAwareIOError(#[from] EndianAwareIOError),

    #[error(transparent)]
    EncodingStringsError(#[from] EncodedStringsError),

    #[error("Other error: {0}")]
    OtherError(String),
}

#[derive(Error, Debug)]
pub enum LocalizationError {
    #[error("Unsupported language.")]
    UnsupportedLanguage,

    #[error("Expected parent in path '{0}'.")]
    MissingParent(std::path::PathBuf),

    #[error("Expected file name in path '{0}'.")]
    MissingFileName(std::path::PathBuf),

    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum LayeredFilesystemError {
    #[error("Cannot create a filesystem with no layers.")]
    NoLayers,

    #[error("Filesystem contains no writeable layers.")]
    NoWriteableLayers,

    #[error("File '{0}' does not exist. Attempted to find it using the following paths: '[{1}]'")]
    FileNotFound(String, String),

    #[error("Failed to read file '{0}' due to nested error: {1}")]
    ReadError(String, String),

    #[error("Failed to write file '{0}' due to nested error: {1}")]
    WriteError(String, String),

    #[error("Unsupported game.")]
    UnsupportedGame,

    #[error(transparent)]
    PatternError(#[from] glob::PatternError),

    #[error(transparent)]
    LocalizationError(#[from] LocalizationError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    CompressionError(#[from] CompressionError),

    #[error(transparent)]
    ArchiveError(#[from] ArchiveError),

    #[error(transparent)]
    TextArchiveError(#[from] TextArchiveError),

    #[error(transparent)]
    TextureParseError(#[from] TextureParseError),

    #[error(transparent)]
    ArcError(#[from] ArcError),

    #[error("Other error: {0}")]
    OtherError(String),
}

#[derive(Error, Debug)]
pub enum TextArchiveError {
    #[error("Malformed text archive - message has no key.")]
    MissingKey,

    #[error(transparent)]
    ArchiveError(#[from] crate::ArchiveError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    EncodingStringsError(#[from] crate::EncodedStringsError),

    #[error("Other error: {0}")]
    OtherError(String),
}

#[derive(Error, Debug)]
pub enum DialogueError {
    #[error("{0}")]
    ParseError(String),

    #[error("Unexpected rule.")]
    BadRule,

    #[error("An undefined error occurred.")]
    UndefinedError,
}

#[derive(Error, Debug)]
pub enum TextureDecodeError {
    #[error("Unsupported format.")]
    UnsupportedFormat,

    #[error("Unaligned data.")]
    UnalignedData,

    #[error("Attempted to perform an indexed operation on an unindexed color format.")]
    NotIndexed,

    #[error("Format requires a palette for encoding and decoding.")]
    NoPalette,

    #[error("Requested index in palette is out of bounds.")]
    OutOfBoundsIndex,

    #[error("Block size is larger than texture dimensions.")]
    BadBlockSize,

    #[error("Texture dimensions are not consistent with input size.")]
    BadDimensions,

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    EndianAwareIOError(#[from] EndianAwareIOError),
}

#[derive(Error, Debug)]
pub enum TextureParseError {
    #[error("Invalid magic number.")]
    BadMagicNumber,

    #[error("Failed to decode text.")]
    BadText,

    #[error("Parser error: {0}")]
    ParserError(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    TextureDecodeError(#[from] TextureDecodeError),
}

#[derive(Error, Debug)]
pub enum ArcError {
    #[error("Arc entry has no name.")]
    MissingName,

    #[error("Arc has no count label.")]
    NoCount,

    #[error("Arc has no info label.")]
    NoInfo,

    #[error(transparent)]
    ArchiveError(#[from] ArchiveError),
}

#[derive(Error, Debug)]
pub enum EndianAwareIOError {
    #[error("Error converting slice to array")]
    ConversionError,

    #[error(transparent)]
    IOError(#[from] std::io::Error),
}
