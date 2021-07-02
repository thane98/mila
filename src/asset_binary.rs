use crate::{ArchiveError, BinArchive, BinArchiveReader, BinArchiveWriter};

type Result<T> = std::result::Result<T, ArchiveError>;

// Credits to AmbiguousPresence for the original implementation.
#[derive(Default, Debug, Clone)]
pub struct AssetSpec {
    pub name: Option<String>,
    pub conditional1: Option<String>,
    pub conditional2: Option<String>,
    pub body_model: Option<String>,
    pub body_texture: Option<String>,
    pub head_model: Option<String>,
    pub head_texture: Option<String>,
    pub hair_model: Option<String>,

    pub hair_texture: Option<String>,
    pub outer_clothing_model: Option<String>,
    pub outer_clothing_texture: Option<String>,
    pub underwear_model: Option<String>,
    pub underwear_texture: Option<String>,
    pub mount_model: Option<String>,
    pub mount_texture: Option<String>,
    pub mount_outer_clothing_model: Option<String>,

    pub mount_outer_clothing_texture: Option<String>,
    pub weapon_model_dual: Option<String>,
    pub weapon_model: Option<String>,
    pub skeleton: Option<String>,
    pub mount_skeleton: Option<String>,
    pub accessory1_model: Option<String>,
    pub accessory1_texture: Option<String>,
    pub accessory2_model: Option<String>,

    pub accessory2_texture: Option<String>,
    pub accessory3_model: Option<String>,
    pub accessory3_texture: Option<String>,
    pub attack_animation: Option<String>,
    pub attack_animation2: Option<String>,
    pub visual_effect: Option<String>,
    pub hid: Option<String>,
    pub footstep_sound: Option<String>,

    pub clothing_sound: Option<String>,
    pub voice: Option<String>,
    pub hair_color: [u8; 4],
    pub use_hair_color: bool,
    pub skin_color: [u8; 4],
    pub use_skin_color: bool,
    pub weapon_trail_color: [u8; 4],
    pub use_weapon_trail_color: bool,
    pub model_size: f32,
    pub use_model_size: bool,
    pub head_size: f32,
    pub use_head_size: bool,
    pub pupil_y: f32,
    pub use_pupil_y: bool,

    pub unk3: u32,
    pub use_unk3: bool,
    pub unk4: u32,
    pub use_unk4: bool,
    pub unk5: u32,
    pub use_unk5: bool,
    pub unk6: u32,
    pub use_unk6: bool,
    pub bitflags: [u8; 4],
    pub use_bitflags: bool,
    pub unk7: u32,
    pub use_unk7: bool,
    pub unk8: u32,
    pub use_unk8: bool,
    pub unk9: u32,
    pub use_unk9: bool,

    pub unk10: u32,
    pub use_unk10: bool,
    pub unk11: u32,
    pub use_unk11: bool,
    pub unk12: u32,
    pub use_unk12: bool,
    pub unk13: u32,
    pub use_unk13: bool,
}

fn read_flag_str(
    reader: &mut BinArchiveReader,
    flags: &[u8],
    index: usize,
) -> Result<Option<String>> {
    let byte = index / 8;
    let bit_index = index % 8;
    if byte >= flags.len() || (flags[byte] & (1 << bit_index)) == 0 {
        Ok(None)
    } else {
        reader.read_string()
    }
}

fn write_flag_str(writer: &mut BinArchiveWriter, value: &Option<String>) -> Result<()> {
    match value {
        Some(value) => {
            writer.write_string(Some(value))?;
        }
        None => {}
    }
    Ok(())
}

fn read_color(reader: &mut BinArchiveReader) -> Result<[u8; 4]> {
    let mut arr: [u8; 4] = [0, 0, 0, 0];
    let bytes = reader.read_bytes(4)?;
    arr.copy_from_slice(&bytes);
    let tmp = arr[2];
    arr[2] = arr[0];
    arr[0] = tmp;
    Ok(arr)
}

fn write_color(color: &[u8; 4], writer: &mut BinArchiveWriter) -> Result<()> {
    let mut arr = color.clone();
    let tmp = arr[2];
    arr[2] = arr[0];
    arr[0] = tmp;
    writer.write_bytes(&arr)
}

fn count_bits(byte: u8) -> usize {
    let mut count = 0;
    for i in 0..8 {
        if (byte & (1 << i)) != 0 {
            count += 1;
        }
    }
    count
}

impl AssetSpec {
    pub fn new() -> Self {
        AssetSpec {
            ..Default::default()
        }
    }

    pub fn from_stream(reader: &mut BinArchiveReader) -> Result<Self> {
        let mut flag_count = 3;
        let raw = reader.read_u8()?;
        if (raw & 0b1) == 1 {
            flag_count += 4;
        }
        let mut flags = vec![raw];
        flags.extend(reader.read_bytes(flag_count)?);

        let mut spec = AssetSpec::new();
        spec.name = reader.read_string()?;
        spec.conditional1 = read_flag_str(reader, &flags, 1)?;
        spec.conditional2 = read_flag_str(reader, &flags, 2)?;
        spec.body_model = read_flag_str(reader, &flags, 3)?;
        spec.body_texture = read_flag_str(reader, &flags, 4)?;
        spec.head_model = read_flag_str(reader, &flags, 5)?;
        spec.head_texture = read_flag_str(reader, &flags, 6)?;
        spec.hair_model = read_flag_str(reader, &flags, 7)?;

        spec.hair_texture = read_flag_str(reader, &flags, 8)?;
        spec.outer_clothing_model = read_flag_str(reader, &flags, 9)?;
        spec.outer_clothing_texture = read_flag_str(reader, &flags, 10)?;
        spec.underwear_model = read_flag_str(reader, &flags, 11)?;
        spec.underwear_texture = read_flag_str(reader, &flags, 12)?;
        spec.mount_model = read_flag_str(reader, &flags, 13)?;
        spec.mount_texture = read_flag_str(reader, &flags, 14)?;
        spec.mount_outer_clothing_model = read_flag_str(reader, &flags, 15)?;

        spec.mount_outer_clothing_texture = read_flag_str(reader, &flags, 16)?;
        spec.weapon_model_dual = read_flag_str(reader, &flags, 17)?;
        spec.weapon_model = read_flag_str(reader, &flags, 18)?;
        spec.skeleton = read_flag_str(reader, &flags, 19)?;
        spec.mount_skeleton = read_flag_str(reader, &flags, 20)?;
        spec.accessory1_model = read_flag_str(reader, &flags, 21)?;
        spec.accessory1_texture = read_flag_str(reader, &flags, 22)?;
        spec.accessory2_model = read_flag_str(reader, &flags, 23)?;

        spec.accessory2_texture = read_flag_str(reader, &flags, 24)?;
        spec.accessory3_model = read_flag_str(reader, &flags, 25)?;
        spec.accessory3_texture = read_flag_str(reader, &flags, 26)?;
        spec.attack_animation = read_flag_str(reader, &flags, 27)?;
        spec.attack_animation2 = read_flag_str(reader, &flags, 28)?;
        spec.visual_effect = read_flag_str(reader, &flags, 29)?;
        spec.hid = read_flag_str(reader, &flags, 30)?;
        spec.footstep_sound = read_flag_str(reader, &flags, 31)?;

        if flag_count > 3 {
            spec.clothing_sound = read_flag_str(reader, &flags, 32)?;
            spec.voice = read_flag_str(reader, &flags, 33)?;
            if (flags[4] & 0b100) != 0 {
                spec.use_hair_color = true;
                spec.hair_color = read_color(reader)?;
            }
            if (flags[4] & 0b1000) != 0 {
                spec.use_skin_color = true;
                spec.skin_color = read_color(reader)?;
            }
            if (flags[4] & 0b10000) != 0 {
                spec.use_weapon_trail_color = true;
                spec.weapon_trail_color = read_color(reader)?;
            }
            if (flags[4] & 0b100000) != 0 {
                spec.use_model_size = true;
                spec.model_size = reader.read_f32()?;
            }
            if (flags[4] & 0b1000000) != 0 {
                spec.use_head_size = true;
                spec.head_size = reader.read_f32()?;
            }
            if (flags[4] & 0b10000000) != 0 {
                spec.use_pupil_y = true;
                spec.pupil_y = reader.read_f32()?;
            }
            if (flags[5] & 0b1) != 0 {
                spec.unk3 = reader.read_u32()?;
                spec.use_unk3 = true;
            }
            if (flags[5] & 0b10) != 0 {
                spec.unk4 = reader.read_u32()?;
                spec.use_unk4 = true;
            }
            if (flags[5] & 0b100) != 0 {
                spec.unk5 = reader.read_u32()?;
                spec.use_unk5 = true;
            }
            if (flags[5] & 0b1000) != 0 {
                spec.unk6 = reader.read_u32()?;
                spec.use_unk6 = true;
            }
            if (flags[5] & 0b10000) != 0 {
                spec.bitflags = read_color(reader)?;
                spec.use_bitflags = true;
            }
            if (flags[5] & 0b100000) != 0 {
                spec.unk7 = reader.read_u32()?;
                spec.use_unk7 = true;
            }
            if (flags[5] & 0b1000000) != 0 {
                spec.unk8 = reader.read_u32()?;
                spec.use_unk8 = true;
            }
            if (flags[5] & 0b10000000) != 0 {
                spec.unk9 = reader.read_u32()?;
                spec.use_unk9 = true;
            }
            if (flags[6] & 0b1) != 0 {
                spec.unk10 = reader.read_u32()?;
                spec.use_unk10 = true;
            }
            if (flags[6] & 0b10) != 0 {
                spec.unk11 = reader.read_u32()?;
                spec.use_unk11 = true;
            }
            if (flags[6] & 0b100) != 0 {
                spec.unk12 = reader.read_u32()?;
                spec.use_unk12 = true;
            }
            if (flags[6] & 0b1000) != 0 {
                spec.unk13 = reader.read_u32()?;
                spec.use_unk13 = true;
            }
        }

        Ok(spec)
    }

    fn compute_flags(&self) -> (Vec<u8>, usize) {
        let mut flags: Vec<u8> = vec![0; 8];
        flags[0] |= if self.conditional1.is_none() { 0 } else { 0b10 };
        flags[0] |= if self.conditional2.is_none() {
            0
        } else {
            0b100
        };
        flags[0] |= if self.body_model.is_none() { 0 } else { 0b1000 };
        flags[0] |= if self.body_texture.is_none() {
            0
        } else {
            0b10000
        };
        flags[0] |= if self.head_model.is_none() {
            0
        } else {
            0b100000
        };
        flags[0] |= if self.head_texture.is_none() {
            0
        } else {
            0b1000000
        };
        flags[0] |= if self.hair_model.is_none() {
            0
        } else {
            0b10000000
        };

        flags[1] |= if self.hair_texture.is_none() { 0 } else { 0b1 };
        flags[1] |= if self.outer_clothing_model.is_none() {
            0
        } else {
            0b10
        };
        flags[1] |= if self.outer_clothing_texture.is_none() {
            0
        } else {
            0b100
        };
        flags[1] |= if self.underwear_model.is_none() {
            0
        } else {
            0b1000
        };
        flags[1] |= if self.underwear_texture.is_none() {
            0
        } else {
            0b10000
        };
        flags[1] |= if self.mount_model.is_none() {
            0
        } else {
            0b100000
        };
        flags[1] |= if self.mount_texture.is_none() {
            0
        } else {
            0b1000000
        };
        flags[1] |= if self.mount_outer_clothing_model.is_none() {
            0
        } else {
            0b10000000
        };

        flags[2] |= if self.mount_outer_clothing_texture.is_none() {
            0
        } else {
            0b1
        };
        flags[2] |= if self.weapon_model_dual.is_none() {
            0
        } else {
            0b10
        };
        flags[2] |= if self.weapon_model.is_none() {
            0
        } else {
            0b100
        };
        flags[2] |= if self.skeleton.is_none() { 0 } else { 0b1000 };
        flags[2] |= if self.mount_skeleton.is_none() {
            0
        } else {
            0b10000
        };
        flags[2] |= if self.accessory1_model.is_none() {
            0
        } else {
            0b100000
        };
        flags[2] |= if self.accessory1_texture.is_none() {
            0
        } else {
            0b1000000
        };
        flags[2] |= if self.accessory2_model.is_none() {
            0
        } else {
            0b10000000
        };

        flags[3] |= if self.accessory2_texture.is_none() {
            0
        } else {
            0b1
        };
        flags[3] |= if self.accessory3_model.is_none() {
            0
        } else {
            0b10
        };
        flags[3] |= if self.accessory3_texture.is_none() {
            0
        } else {
            0b100
        };
        flags[3] |= if self.attack_animation.is_none() {
            0
        } else {
            0b1000
        };
        flags[3] |= if self.attack_animation2.is_none() {
            0
        } else {
            0b10000
        };
        flags[3] |= if self.visual_effect.is_none() {
            0
        } else {
            0b100000
        };
        flags[3] |= if self.hid.is_none() { 0 } else { 0b1000000 };
        flags[3] |= if self.footstep_sound.is_none() {
            0
        } else {
            0b10000000
        };

        flags[4] |= if self.clothing_sound.is_none() {
            0
        } else {
            0b1
        };
        flags[4] |= if self.voice.is_none() { 0 } else { 0b10 };
        flags[4] |= if !self.use_hair_color { 0 } else { 0b100 };
        flags[4] |= if !self.use_skin_color { 0 } else { 0b1000 };
        flags[4] |= if !self.use_weapon_trail_color {
            0
        } else {
            0b10000
        };
        flags[4] |= if !self.use_model_size { 0 } else { 0b100000 };
        flags[4] |= if !self.use_head_size { 0 } else { 0b1000000 };
        flags[4] |= if !self.use_pupil_y { 0 } else { 0b10000000 };

        flags[5] |= if !self.use_unk3 { 0 } else { 0b1 };
        flags[5] |= if !self.use_unk4 { 0 } else { 0b10 };
        flags[5] |= if !self.use_unk5 { 0 } else { 0b100 };
        flags[5] |= if !self.use_unk6 { 0 } else { 0b1000 };
        flags[5] |= if !self.use_bitflags { 0 } else { 0b10000 };
        flags[5] |= if !self.use_unk7 { 0 } else { 0b100000 };
        flags[5] |= if !self.use_unk8 { 0 } else { 0b1000000 };
        flags[5] |= if !self.use_unk9 { 0 } else { 0b10000000 };

        flags[6] |= if !self.use_unk10 { 0 } else { 0b1 };
        flags[6] |= if !self.use_unk11 { 0 } else { 0b10 };
        flags[6] |= if !self.use_unk12 { 0 } else { 0b100 };
        flags[6] |= if !self.use_unk13 { 0 } else { 0b1000 };

        if flags[4] == 0 && flags[5] == 0 && flags[6] == 0 {
            flags.resize(4, 0);
        }
        let mut size = flags.len() + 4;
        for flag in &flags {
            size += count_bits(*flag) * 4;
        }
        if flags.len() > 4 {
            flags[0] |= 1;
        }
        (flags, size)
    }

    pub fn append(&self, archive: &mut BinArchive) -> Result<()> {
        let (flags, size) = self.compute_flags();
        let address = archive.size();
        archive.allocate_at_end(size);
        let mut writer = BinArchiveWriter::new(archive, address);
        writer.write_bytes(&flags)?;
        writer.write_string(self.name.as_deref())?;

        write_flag_str(&mut writer, &self.conditional1)?;
        write_flag_str(&mut writer, &self.conditional2)?;
        write_flag_str(&mut writer, &self.body_model)?;
        write_flag_str(&mut writer, &self.body_texture)?;
        write_flag_str(&mut writer, &self.head_model)?;
        write_flag_str(&mut writer, &self.head_texture)?;
        write_flag_str(&mut writer, &self.hair_model)?;

        write_flag_str(&mut writer, &self.hair_texture)?;
        write_flag_str(&mut writer, &self.outer_clothing_model)?;
        write_flag_str(&mut writer, &self.outer_clothing_texture)?;
        write_flag_str(&mut writer, &self.underwear_model)?;
        write_flag_str(&mut writer, &self.underwear_texture)?;
        write_flag_str(&mut writer, &self.mount_model)?;
        write_flag_str(&mut writer, &self.mount_texture)?;
        write_flag_str(&mut writer, &self.mount_outer_clothing_model)?;

        write_flag_str(&mut writer, &self.mount_outer_clothing_texture)?;
        write_flag_str(&mut writer, &self.weapon_model_dual)?;
        write_flag_str(&mut writer, &self.weapon_model)?;
        write_flag_str(&mut writer, &self.skeleton)?;
        write_flag_str(&mut writer, &self.mount_skeleton)?;
        write_flag_str(&mut writer, &self.accessory1_model)?;
        write_flag_str(&mut writer, &self.accessory1_texture)?;
        write_flag_str(&mut writer, &self.accessory2_model)?;

        write_flag_str(&mut writer, &self.accessory2_texture)?;
        write_flag_str(&mut writer, &self.accessory3_model)?;
        write_flag_str(&mut writer, &self.accessory3_texture)?;
        write_flag_str(&mut writer, &self.attack_animation)?;
        write_flag_str(&mut writer, &self.attack_animation2)?;
        write_flag_str(&mut writer, &self.visual_effect)?;
        write_flag_str(&mut writer, &self.hid)?;
        write_flag_str(&mut writer, &self.footstep_sound)?;

        if flags.len() > 4 {
            write_flag_str(&mut writer, &self.clothing_sound)?;
            write_flag_str(&mut writer, &self.voice)?;
            if self.use_hair_color {
                write_color(&self.hair_color, &mut writer)?;
            }
            if self.use_skin_color {
                write_color(&self.skin_color, &mut writer)?;
            }
            if self.use_weapon_trail_color {
                write_color(&self.weapon_trail_color, &mut writer)?;
            }
            if self.use_model_size {
                writer.write_f32(self.model_size)?;
            }
            if self.use_head_size {
                writer.write_f32(self.head_size)?;
            }
            if self.use_pupil_y {
                writer.write_f32(self.pupil_y)?;
            }
            if self.use_unk3 {
                writer.write_u32(self.unk3)?;
            }
            if self.use_unk4 {
                writer.write_u32(self.unk4)?;
            }
            if self.use_unk5 {
                writer.write_u32(self.unk5)?;
            }
            if self.use_unk6 {
                writer.write_u32(self.unk6)?;
            }
            if self.use_bitflags {
                write_color(&self.bitflags, &mut writer)?;
            }
            if self.use_unk7 {
                writer.write_u32(self.unk7)?;
            }
            if self.use_unk8 {
                writer.write_u32(self.unk8)?;
            }
            if self.use_unk9 {
                writer.write_u32(self.unk9)?;
            }
            if self.use_unk10 {
                writer.write_u32(self.unk10)?;
            }
            if self.use_unk11 {
                writer.write_u32(self.unk11)?;
            }
            if self.use_unk12 {
                writer.write_u32(self.unk12)?;
            }
            if self.use_unk13 {
                writer.write_u32(self.unk13)?;
            }
        }
        Ok(())
    }
}

pub struct AssetBinary {
    pub flags: u32,
    pub specs: Vec<AssetSpec>,
}

impl AssetBinary {
    pub fn new() -> Self {
        AssetBinary {
            flags: 0,
            specs: Vec::new(),
        }
    }

    pub fn from_archive(archive: &BinArchive) -> Result<Self> {
        let mut binary = AssetBinary::new();
        let mut reader = BinArchiveReader::new(archive, 0);
        binary.flags = reader.read_u32()?;

        // Read until we hit a malformed spec or EOF.
        let mut error = false;
        while !error {
            match AssetSpec::from_stream(&mut reader) {
                Ok(spec) => {
                    binary.specs.push(spec);
                }
                Err(_) => {
                    error = true;
                }
            }
        }
        Ok(binary)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut archive = BinArchive::new();
        archive.allocate_at_end(4);
        archive.write_u32(0, self.flags)?;
        for spec in &self.specs {
            spec.append(&mut archive)?;
        }
        archive.allocate_at_end(4);
        archive.serialize()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::load_test_file;

    #[test]
    fn round_trip() {
        let file = load_test_file("AssetBinary_Test.bin");
        let archive = BinArchive::from_bytes(&file).unwrap();
        let asset_binary = AssetBinary::from_archive(&archive);
        assert!(asset_binary.is_ok());
        let asset_binary = asset_binary.unwrap();
        let bytes = asset_binary.serialize().unwrap();
        assert_eq!(file, bytes);
    }
}
