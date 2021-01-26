use strum_macros::EnumString;

#[derive(Copy, Clone, Debug, EnumString)]
pub enum Game {
    FE9,
    FE10,
    FE11,
    FE12,
    FE13,
    FE14,
    FE15,
}