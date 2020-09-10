mod errors;
mod bin_archive;
mod lz13;
mod encoded_strings;
mod compression_format;

#[cfg(test)]
mod utils;

pub use bin_archive::BinArchive;
pub use compression_format::CompressionFormat;
pub use lz13::LZ13CompressionFormat;
pub use errors::{ArchiveError, CompressionError, EncodedStringsError};