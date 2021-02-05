use crate::{arc, bch, cgfx, ctpk, LayeredFilesystemError, TextArchive, Texture};
use crate::{
    BinArchive, CompressionFormat, FE13PathLocalizer, FE14PathLocalizer, FE15PathLocalizer, Game,
    LZ13CompressionFormat, Language, PathLocalizer,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, LayeredFilesystemError>;

pub struct LayeredFilesystem {
    layers: Vec<String>,
    compression_format: CompressionFormat,
    path_localizer: PathLocalizer,
    game: Game,
    language: Language,
}

impl Clone for LayeredFilesystem {
    fn clone(&self) -> Self {
        LayeredFilesystem::new(self.layers.clone(), self.language, self.game).unwrap()
    }
}

fn texture_vec_to_map(textures: Vec<Texture>) -> HashMap<String, Texture> {
    textures
        .into_iter()
        .map(|t| (t.filename.clone(), t))
        .collect()
}

impl LayeredFilesystem {
    pub fn new(layers: Vec<String>, language: Language, game: Game) -> Result<Self> {
        if layers.is_empty() {
            return Err(LayeredFilesystemError::NoLayers);
        }
        let compression_format: CompressionFormat = match game {
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
            Game::FE13 => CompressionFormat::LZ13(LZ13CompressionFormat {}),
            Game::FE14 => CompressionFormat::LZ13(LZ13CompressionFormat {}),
            Game::FE15 => CompressionFormat::LZ13(LZ13CompressionFormat {}),
        };
        let path_localizer: PathLocalizer = match game {
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
            Game::FE13 => PathLocalizer::FE13(FE13PathLocalizer {}),
            Game::FE14 => PathLocalizer::FE14(FE14PathLocalizer {}),
            Game::FE15 => PathLocalizer::FE15(FE15PathLocalizer {}),
        };

        let mut canonical_layers: Vec<String> = Vec::new();
        for layer in &layers {
            canonical_layers.push(dunce::canonicalize(layer)?.display().to_string());
        }

        Ok(LayeredFilesystem {
            layers: canonical_layers,
            compression_format,
            path_localizer,
            game,
            language,
        })
    }

    pub fn list(&self, path: &str, glob: Option<&str>) -> Result<Vec<String>> {
        let mut result: HashSet<String> = HashSet::new();
        for layer in &self.layers {
            // TODO: Instead of eating the error, check if dir exists before listing.
            match self.list_dir(layer, path, glob.clone()) {
                Ok(paths) => result.extend(paths.into_iter()),
                Err(_) => {}
            }
        }
        let mut result: Vec<String> = result.into_iter().collect();
        result.sort();
        Ok(result)
    }

    fn list_dir(&self, layer: &str, subdir: &str, glob: Option<&str>) -> Result<Vec<String>> {
        // TODO: Clean up this mess.
        let mut layer_str = String::new();
        layer_str.push_str(layer);
        layer_str.push(std::path::MAIN_SEPARATOR);
        let mut path = PathBuf::new();
        path.push(layer);
        path.push(subdir);
        let mut canonical = dunce::canonicalize(path)?.display().to_string();
        canonical.push(std::path::MAIN_SEPARATOR);

        let pattern = if let Some(p) = glob { p } else { "**/*" };
        let pattern = format!("{}{}", canonical, pattern);
        Ok(glob::glob(&pattern)?
            .filter_map(|r| r.ok())
            .map(|p| p.display().to_string().replace(&layer_str, ""))
            .collect())
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
                let bytes = match std::fs::read(&path_buf) {
                    Ok(b) => b,
                    Err(err) => {
                        return Err(LayeredFilesystemError::ReadError(
                            actual_path.to_string(),
                            err.to_string(),
                        ))
                    }
                };
                if self.compression_format.is_compressed_filename(path) {
                    return Ok(self.compression_format.decompress(&bytes)?);
                } else {
                    return Ok(bytes);
                }
            }
        }
        Err(LayeredFilesystemError::FileNotFound(actual_path))
    }

    pub fn read_arc(&self, path: &str, localized: bool) -> Result<HashMap<String, Vec<u8>>> {
        let bytes = self.read(path, localized)?;
        let arc = arc::from_bytes(&bytes)?;
        Ok(arc)
    }

    pub fn read_archive(&self, path: &str, localized: bool) -> Result<BinArchive> {
        let bytes = self.read(path, localized)?;
        let archive = BinArchive::from_bytes(&bytes)?;
        Ok(archive)
    }

    pub fn read_text_archive(&self, path: &str, localized: bool) -> Result<TextArchive> {
        let bytes = self.read(path, localized)?;
        let archive = TextArchive::from_bytes(&bytes)?;
        Ok(archive)
    }

    pub fn read_bch_textures(
        &self,
        path: &str,
        localized: bool,
    ) -> Result<HashMap<String, Texture>> {
        let bytes = self.read(path, localized)?;
        Ok(texture_vec_to_map(bch::read(&bytes)?))
    }

    pub fn read_ctpk_textures(
        &self,
        path: &str,
        localized: bool,
    ) -> Result<HashMap<String, Texture>> {
        let bytes = self.read(path, localized)?;
        Ok(texture_vec_to_map(ctpk::read(&bytes)?))
    }

    pub fn read_cgfx_textures(
        &self,
        path: &str,
        localized: bool,
    ) -> Result<HashMap<String, Texture>> {
        let bytes = self.read(path, localized)?;
        Ok(texture_vec_to_map(cgfx::read(&bytes)?))
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
            self.write_with_error_handling(&path_buf, &contents)
        } else {
            self.write_with_error_handling(&path_buf, bytes)
        }
    }

    fn write_with_error_handling(&self, path: &PathBuf, bytes: &[u8]) -> Result<()> {
        match std::fs::write(path, bytes) {
            Ok(_) => Ok(()),
            Err(err) => Err(LayeredFilesystemError::WriteError(
                path.display().to_string(),
                err.to_string(),
            )),
        }
    }

    pub fn write_archive(&self, path: &str, archive: &BinArchive, localized: bool) -> Result<()> {
        let bytes = archive.serialize()?;
        self.write(path, &bytes, localized)
    }

    pub fn write_text_archive(
        &self,
        path: &str,
        archive: &TextArchive,
        localized: bool,
    ) -> Result<()> {
        let bytes = archive.serialize()?;
        self.write(path, &bytes, localized)
    }

    pub fn localizer(&self) -> PathLocalizer {
        self.path_localizer
    }

    pub fn language(&self) -> Language {
        self.language
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn list() {
        // TODO: Current assertions are primitive.
        //       In the future, we should actually check the paths we get back.
        let mut test_dir_1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_dir_1.push("resources/test/FSListTest1");
        let mut test_dir_2 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_dir_2.push("resources/test/FSListTest2");
        let fs = LayeredFilesystem::new(
            vec![
                test_dir_1.display().to_string(),
                test_dir_2.display().to_string(),
            ],
            Language::EnglishNA,
            Game::FE15,
        )
        .unwrap();
        let all_files = fs.list("Subdir/", None).unwrap();
        let text = fs.list("Subdir/", Some("**/*.txt")).unwrap();
        assert_eq!(4, all_files.len());
        assert_eq!(2, text.len());
    }

    #[test]
    fn write_and_read() {
        // Create temporary directories.
        // Add a test file to the first layer.
        let layer1 = tempfile::tempdir().unwrap();
        let layer2 = tempfile::tempdir().unwrap();
        let layer1_path = layer1.path().to_string_lossy().to_string();
        let layer2_path = layer2.path().to_string_lossy().to_string();
        std::fs::create_dir_all(layer1.path().join("m/@E")).unwrap();
        std::fs::write(
            layer1.path().join("m/@E/GameData.txt"),
            "Original".as_bytes(),
        )
        .unwrap();

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
