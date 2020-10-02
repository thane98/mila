use crate::errors::LayeredFilesystemError;
use crate::{
    BinArchive, CompressionFormat, FE13PathLocalizer, FE14PathLocalizer, FE15PathLocalizer, Game,
    LZ13CompressionFormat, Language, PathLocalizer,
};
use std::path::Path;

type Result<T> = std::result::Result<T, LayeredFilesystemError>;

pub struct LayeredFilesystem {
    layers: Vec<String>,
    compression_format: Box<dyn CompressionFormat>,
    path_localizer: Box<dyn PathLocalizer>,
    language: Language,
}

impl LayeredFilesystem {
    pub fn new(layers: Vec<String>, language: Language, game: Game) -> Result<Self> {
        if layers.is_empty() {
            return Err(LayeredFilesystemError::NoLayers);
        }
        let compression_format: Box<dyn CompressionFormat> = match game {
            Game::FE9 => {
                return Err(LayeredFilesystemError::UnsupportedGame);
            }
            Game::FE10 => {
                return Err(LayeredFilesystemError::UnsupportedGame);
            }
            Game::FE11 => {
                return Err(LayeredFilesystemError::UnsupportedGame);
            }
            Game::FE12 => {
                return Err(LayeredFilesystemError::UnsupportedGame);
            }
            Game::FE13 => Box::new(LZ13CompressionFormat {}),
            Game::FE14 => Box::new(LZ13CompressionFormat {}),
            Game::FE15 => Box::new(LZ13CompressionFormat {}),
        };
        let path_localizer: Box<dyn PathLocalizer> = match game {
            Game::FE9 => {
                return Err(LayeredFilesystemError::UnsupportedGame);
            }
            Game::FE10 => {
                return Err(LayeredFilesystemError::UnsupportedGame);
            }
            Game::FE11 => {
                return Err(LayeredFilesystemError::UnsupportedGame);
            }
            Game::FE12 => {
                return Err(LayeredFilesystemError::UnsupportedGame);
            }
            Game::FE13 => Box::new(FE13PathLocalizer {}),
            Game::FE14 => Box::new(FE14PathLocalizer {}),
            Game::FE15 => Box::new(FE15PathLocalizer {}),
        };
        Ok(LayeredFilesystem {
            layers,
            compression_format,
            path_localizer,
            language,
        })
    }

    pub fn read(&self, path: &str, localized: bool) -> Result<Vec<u8>> {
        let actual_path = if localized {
            self.path_localizer.localize(path, &self.language)?
        } else {
            path.to_string()
        };
        for layer in self.layers.iter().rev() {
            let path_buf = Path::new(layer).join(&actual_path);
            if path_buf.exists() {
                let bytes = std::fs::read(&path_buf)?;
                if self.compression_format.is_compressed_filename(path) {
                    return Ok(self.compression_format.decompress(&bytes)?);
                } else {
                    return Ok(bytes);
                }
            }
        }
        Err(LayeredFilesystemError::FileNotFound(actual_path))
    }

    pub fn read_archive(&self, path: &str, localized: bool) -> Result<BinArchive> {
        let bytes = self.read(path, localized)?;
        let archive = BinArchive::from_bytes(&bytes)?;
        Ok(archive)
    }

    pub fn write(&self, path: &str, bytes: &[u8], localized: bool) -> Result<()> {
        let actual_path = if localized {
            self.path_localizer.localize(path, &self.language)?
        } else {
            path.to_string()
        };
        let layer = self
            .layers
            .last()
            .ok_or(LayeredFilesystemError::NoWriteableLayers)?;

        let path_buf = Path::new(layer).join(&actual_path);
        match path_buf.parent() {
            Some(parent) => std::fs::create_dir_all(parent)?,
            None => {}
        }
        if self.compression_format.is_compressed_filename(path) {
            let contents = self.compression_format.compress(bytes)?;
            std::fs::write(path_buf, contents)?;
        } else {
            std::fs::write(path_buf, bytes)?;
        }
        Ok(())
    }

    pub fn write_archive(&self, path: &str, archive: &BinArchive, localized: bool) -> Result<()> {
        let bytes = archive.serialize()?;
        self.write(path, &bytes, localized)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn write_and_read() {
        // Create temporary directories.
        // Add a test file to the first layer.
        let layer1 = tempfile::tempdir().unwrap();
        let layer2 = tempfile::tempdir().unwrap();
        let layer1_path = layer1.path().to_string_lossy().to_string();
        let layer2_path = layer2.path().to_string_lossy().to_string();
        std::fs::create_dir_all(layer1.path().join("m/@E")).unwrap();
        std::fs::write(layer1.path().join("m/@E/GameData.txt"), "Original".as_bytes()).unwrap();

        // Create the layered filesystem.
        let fs = LayeredFilesystem::new(
            vec![layer1_path, layer2_path],
            Language::EnglishNA,
            Game::FE14,
        );
        assert!(fs.is_ok());

        // Read the original file.
        let fs = fs.unwrap();
        let result = fs.read("m/GameData.txt", true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Original".as_bytes());

        // Write a new file.
        let result = fs.write("m/GameData.txt", "MyString".as_bytes(), true);
        assert!(result.is_ok());
        assert!(layer2.path().join("m/@E/GameData.txt").exists());
        
        // Test that the layer1 file was NOT overwritten.
        let result = std::fs::read(layer1.path().join("m/@E/GameData.txt"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Original".as_bytes());

        // Read the new/layer2 file.
        let result = fs.read("m/GameData.txt", true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "MyString".as_bytes());
    }
}
