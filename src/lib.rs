mod asset_binary;
mod bin_archive;
mod bin_streams;
mod compression_format;
mod encoded_strings;
mod endian_aware_io;
mod errors;
mod etc1;
mod game;
mod language;
mod layered_filesystem;
mod localization;
mod lz10;
mod lz13;
mod pixel_encodings;
mod text_archive;
mod texture;
mod texture_decoder;
mod texture_utils;

pub mod arc;
pub mod bch;
pub mod cgfx;
pub mod ctpk;
pub mod tpl;

#[cfg(test)]
mod utils;

use endian_aware_io::{EndianAwareReader, EndianAwareWriter};

pub use asset_binary::{AssetBinary, AssetSpec};
pub use bin_archive::BinArchive;
pub use bin_streams::{BinArchiveReader, BinArchiveWriter};
pub use compression_format::CompressionFormat;
pub use encoded_strings::EncodedStringReader;
pub use endian_aware_io::Endian;
pub use etc1::decode;
pub use game::Game;
pub use language::Language;
pub use layered_filesystem::LayeredFilesystem;
pub use lz10::LZ10CompressionFormat;
pub use lz13::LZ13CompressionFormat;
pub use pixel_encodings::ColorFormat;
pub use text_archive::{TextArchive, TextArchiveFormat};
pub use texture::Texture;

pub use errors::{
    ArcError, ArchiveError, CompressionError, DialogueError, EncodedStringsError,
    EndianAwareIOError, LayeredFilesystemError, LocalizationError, TextArchiveError,
    TextureDecodeError, TextureParseError,
};
pub use localization::{
    FE10PathLocalizer, FE13PathLocalizer, FE14PathLocalizer, FE15PathLocalizer, NoOpPathLocalizer,
    PathLocalizer,
};
