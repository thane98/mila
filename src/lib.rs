mod bin_archive;
mod bin_streams;
mod compression_format;
mod encoded_strings;
mod errors;
mod game;
mod language;
mod layered_filesystem;
mod localization;
mod lz13;
mod text_archive;

#[cfg(test)]
mod utils;

pub use bin_archive::BinArchive;
pub use bin_streams::{BinArchiveReader, BinArchiveWriter};
pub use compression_format::CompressionFormat;
pub use encoded_strings::EncodedStringReader;
pub use errors::{
    ArchiveError, CompressionError, EncodedStringsError, LayeredFilesystemError, LocalizationError,
    TextArchiveError,
};
pub use game::Game;
pub use language::Language;
pub use layered_filesystem::LayeredFilesystem;
pub use localization::{
    FE13PathLocalizer, FE14PathLocalizer, FE15PathLocalizer, NoOpPathLocalizer, PathLocalizer,
};
pub use lz13::LZ13CompressionFormat;
pub use text_archive::TextArchive;
