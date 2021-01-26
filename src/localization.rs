use std::path::Path;
use crate::errors::LocalizationError;
use crate::Language;

type Result<T> = std::result::Result<T, LocalizationError>;


fn get_parent_as_string(path: &Path) -> Result<String> {
    let parent = path.parent();
    match parent {
        Some(value) => Ok(value.to_str().unwrap().to_string()),
        None => Err(LocalizationError::MissingParent(path.to_path_buf())),
    }
}

fn get_file_name(path: &Path) -> Result<String> {
    let file_name_opt = path.file_name();
    match file_name_opt {
        Some(value) => Ok(value.to_str().unwrap().to_string()),
        None => Err(LocalizationError::MissingFileName(path.to_path_buf())),
    }
}

pub struct NoOpPathLocalizer;
pub struct FE13PathLocalizer;
pub struct FE14PathLocalizer;
pub struct FE15PathLocalizer;

pub enum PathLocalizer {
    NoOp(NoOpPathLocalizer),
    FE13(FE13PathLocalizer),
    FE14(FE14PathLocalizer),
    FE15(FE15PathLocalizer),
}

impl PathLocalizer {
    pub fn localize(&self, path: &str, language: &Language) -> Result<String> {
        match self {
            PathLocalizer::NoOp(p) => p.localize(path),
            PathLocalizer::FE13(p) => p.localize(path, language),
            PathLocalizer::FE14(p) => p.localize(path, language),
            PathLocalizer::FE15(p) => p.localize(path, language),
        }
    }
}

impl NoOpPathLocalizer {
    fn localize(&self, path: &str) -> Result<String> {
        Ok(path.to_string())
    }
}

impl FE13PathLocalizer {
    fn localize(&self, path: &str, language: &Language) -> Result<String> {
        let mut result = String::new();
        let path_info = Path::new(path);
        let dir_name = get_parent_as_string(path_info)?;
        let file_name = get_file_name(path_info)?;
        result.push_str(&dir_name);
        match language {
            Language::EnglishNA => result.push_str("/E/"),
            Language::EnglishEU => result.push_str("/U/"),
            Language::Japanese => result.push_str("/"),
            Language::Spanish => result.push_str("/S/"),
            Language::French => result.push_str("/F/"),
            Language::German => result.push_str("/G/"),
            Language::Italian => result.push_str("/I/"),
            Language::Dutch => {
                return Err(LocalizationError::UnsupportedLanguage);
            }
        }
        result.push_str(&file_name);
        Ok(result)
    }
}

impl FE14PathLocalizer {
    fn localize(&self, path: &str, language: &Language) -> Result<String> {
        let mut result = String::new();
        let path_info = Path::new(path);
        let dir_name = get_parent_as_string(path_info)?;
        let file_name = get_file_name(path_info)?;
        result.push_str(&dir_name);
        match language {
            Language::EnglishNA => result.push_str("/@E/"),
            Language::EnglishEU => result.push_str("/@U/"),
            Language::Japanese => result.push_str("/"),
            Language::Spanish => result.push_str("/@S/"),
            Language::French => result.push_str("/@F/"),
            Language::German => result.push_str("/@G/"),
            Language::Italian => result.push_str("/@I/"),
            Language::Dutch => {
                return Err(LocalizationError::UnsupportedLanguage);
            }
        }
        result.push_str(&file_name);
        Ok(result)
    }
}

impl FE15PathLocalizer {
    fn localize(&self, path: &str, language: &Language) -> Result<String> {
        let mut result = String::new();
        let path_info = Path::new(path);
        let dir_name = get_parent_as_string(path_info)?;
        let file_name = get_file_name(path_info)?;
        result.push_str(&dir_name);
        match language {
            Language::EnglishNA => result.push_str("/@NOA_EN/"),
            Language::EnglishEU => result.push_str("/@NOE_EN/"),
            Language::Japanese => result.push_str("/@J/"),
            Language::Spanish => result.push_str("/@NOE_SP/"),
            Language::French => result.push_str("/@NOE_FR/"),
            Language::German => result.push_str("/@NOE_GE/"),
            Language::Italian => result.push_str("/@NOE_IT/"),
            Language::Dutch => result.push_str("/@NOE_DU/"),
        }
        result.push_str(&file_name);
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Language;

    #[test]
    fn localize_fe13_japanese_path() {
        let localizer = FE13PathLocalizer {};
        let path = localizer.localize("m/GameData.bin.lz", &Language::Japanese);
        assert!(path.is_ok());
        assert_eq!(&path.unwrap(), "m/GameData.bin.lz");
    }

    #[test]
    fn localize_fe13_spanish_path() {
        let localizer = FE13PathLocalizer {};
        let path = localizer.localize("m/GameData.bin.lz", &Language::Spanish);
        assert!(path.is_ok());
        assert_eq!(&path.unwrap(), "m/S/GameData.bin.lz");
    }

    #[test]
    fn localize_fe14_japanese_path() {
        let localizer = FE14PathLocalizer {};
        let path = localizer.localize("m/GameData.bin.lz", &Language::Japanese);
        assert!(path.is_ok());
        assert_eq!(&path.unwrap(), "m/GameData.bin.lz");
    }

    #[test]
    fn localize_fe14_spanish_path() {
        let localizer = FE14PathLocalizer {};
        let path = localizer.localize("m/GameData.bin.lz", &Language::Spanish);
        assert!(path.is_ok());
        assert_eq!(&path.unwrap(), "m/@S/GameData.bin.lz");
    }

    #[test]
    fn localize_fe15_japanese_path() {
        let localizer = FE15PathLocalizer {};
        let path = localizer.localize("m/GameData.bin.lz", &Language::Japanese);
        assert!(path.is_ok());
        assert_eq!(&path.unwrap(), "m/@J/GameData.bin.lz");
    }

    #[test]
    fn localize_fe15_spanish_path() {
        let localizer = FE15PathLocalizer {};
        let path = localizer.localize("m/GameData.bin.lz", &Language::Spanish);
        assert!(path.is_ok());
        assert_eq!(&path.unwrap(), "m/@NOE_SP/GameData.bin.lz");
    }
}
