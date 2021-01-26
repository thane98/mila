mod asset_binary;
mod bin_archive;
mod bin_streams;
mod compression_format;
mod encoded_strings;
mod errors;
mod etc1;
mod game;
mod language;
mod layered_filesystem;
mod localization;
mod lz13;
mod text_archive;
mod texture;
mod texture_decoder;

pub mod arc;
pub mod bch;
pub mod cgfx;
pub mod ctpk;

#[cfg(test)]
mod utils;

pub use asset_binary::{AssetBinary, AssetSpec};
pub use bin_archive::BinArchive;
pub use bin_streams::{BinArchiveReader, BinArchiveWriter};
pub use compression_format::CompressionFormat;
pub use encoded_strings::EncodedStringReader;
pub use etc1::decode;
pub use game::Game;
pub use language::Language;
pub use layered_filesystem::LayeredFilesystem;
pub use lz13::LZ13CompressionFormat;
pub use text_archive::TextArchive;
pub use texture::Texture;

pub use errors::{
    ArcError, ArchiveError, CompressionError, DialogueError, EncodedStringsError,
    LayeredFilesystemError, LocalizationError, TextArchiveError, TextureDecodeError,
    TextureParseError,
};
pub use localization::{
    FE13PathLocalizer, FE14PathLocalizer, FE15PathLocalizer, NoOpPathLocalizer, PathLocalizer,
};
