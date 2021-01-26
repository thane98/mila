use strum_macros::EnumString;

#[derive(Copy, Clone, Debug, EnumString)]
pub enum Language {
    EnglishNA,
    EnglishEU,
    Japanese,
    Spanish,
    French,
    Italian,
    German,
    Dutch,
}