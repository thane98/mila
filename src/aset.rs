use crate::{ArchiveError, BinArchive, BinArchiveReader, BinArchiveWriter, Endian};

type Result<T> = std::result::Result<T, ArchiveError>;

pub const FE14_ANIMATION_NAMES: &[&str] = &[
    "label",
    "ready",
    "idle_normal",
    "pre_battle_3",
    "idle_dying",
    "run",
    "backstep",
    "forward_step",
    "attack_1",
    "attack_2",
    "attack_t",
    "attack_s",
    "attack_c",
    "attack_f",
    "shoot",
    "shoot_c",
    "evasion",
    "dmg_none",
    "dmg_mid",
    "dmg_high",
    "die",
    "start",
    "win",
    "charge",
    "discharge",
    "cheer",
    "attack_d",
    "attack_dc",
    "deform",
    "sing",
    "shoot_d",
    "shoot_dc",
    "pre_battle_6",
    "standing",
    "walking",
    "small_step_right",
    "large_step_right",
    "small_step_left",
    "large_step_left",
    "talk_1",
    "talk_2",
    "nodding",
    "shaking_head",
    "looking_back",
    "looking_forward",
    "looking_down",
    "falling_into_valley",
    "falling_down",
    "looking_around",
    "jumping_down",
    "landing",
    "grand_gesture",
    "worrying",
    "surprised",
    "retreating",
    "standing_up",
    "arguing",
    "looking_up",
    "bathing_1",
    "sit_down_on_chair",
    "sleeping",
    "sitting_and_talking_1",
    "tiring_1",
    "tiring_2",
    "tiring_3",
    "blown_away",
    "peering_1",
    "peering_2",
    "peering_3",
    "sitting_down_on_chair",
    "standing_from_chair",
    "rising_from_the_dead",
    "rising_from_sleep",
    "sleeping_to_sitting",
    "lying_down_to_vertical_back",
    "talking_with_vertical_back",
    "while_corrin_is_touching_face_1",
    "standing_after_corrin_touches_face",
    "collapsing",
    "flustered",
    "flustered_2",
    "武器を突き出す",
    "武器を戻す",
    "半身起き悩み",
    "半身起き驚き",
    "半身起き驚き留まる",
    "深呼吸",
    "回転",
    "呪文を唱える",
    "跪く",
    "背中を叩く",
    "叩かれてのけぞる",
    "庇う",
    "馬移動1",
    "座って話す2",
    "慌て留まる",
    "ベッドに座る1",
    "よろける",
    "構える",
    "ベッドに座る2",
    "腕を掲げる",
    "慌て上半身上げ2待機",
    "慌て上半身上げ2",
    "よつんばい",
    "よつんばい→見渡す",
    "怯える",
    "顔を撫でる1",
    "慌て上半身上げ",
    "構え振り向き",
    "歌う1",
    "歌う3",
    "死亡→跪く",
    "歌う2",
    "泣く",
    "叩かれてのけぞる2",
    "ベッドに寝る",
    "入浴苦しむ1",
    "入浴苦しむ2",
    "喜ぶ2",
    "跪くうつむく",
    "首横振り武器持ち",
    "手振り武器持ち",
    "話す武器持ち",
    "掲げ戻す",
    "構えよろける",
    "へたりこむ",
    "喜ぶ",
    "手を持つ",
    "飛ぶ1",
    "飛ぶ受け取る",
    "構え嘆く",
    "戴冠式1",
    "戴冠式2",
    "上昇",
    "倒れる",
    "驚く2",
    "扉につく1",
    "死亡1",
    "庇う2",
    "構え見回す1",
    "戦闘態勢のまま下を向く",
    "構えよろける2",
    "腕を胸に当てる",
    "クラスチェンジ体勢1",
    "クラスチェンジ体勢2",
    "寝返り",
    "横たわる",
    "抱きしめる",
    "強く抱きしめる",
    "雷を受けよろける",
    "谷に落ちる2",
    "顔を撫でる2",
    "跪く頷く",
    "よろけ頷く",
    "上半身起き→倒れ",
    "膝立ち待機",
    "膝立ち叫び",
    "膝立ち叫び2",
    "膝立ち叫び3",
    "横たわる死",
    "横たわる死_待機",
    "横たわる死_待機2",
    "脅され待機",
    "風神弓を前に出す",
    "自刃1",
    "自刃2",
    "自刃3",
    "自刃4",
    "膝立ち待機沈む",
    "聞き耳1",
    "聞き耳2",
    "跪く→立つ",
    "よつんばい→首振り",
    "よつんばい→立つ",
    "よつんばい前を見る",
    "none1",
    "よつんばい立ち待機",
    "ショップ用立ち",
    "思い出す",
    "手を持つ2",
    "手を持つ3",
    "手を持つ4",
    "手を持つ5",
    "谷底を覗く",
    "馬と谷に落ちる",
    "攻撃1",
    "攻撃2",
    "跪く待機",
    "リリスに乗る",
    "跪く話す1",
    "跪く話す2",
    "跪く首振り",
    "抱きしめる2",
    "抱きしめる3",
    "抱きしめる4",
    "かがむ",
    "ベッドに座って話す1",
    "手を持つ6",
    "お辞儀",
    "飛び込む",
    "かがむ戻り",
    "ベッドに座って話す2",
    "部分竜化1",
    "部分竜化2",
    "部分竜化3",
    "入浴飛び込む",
    "神託受ける",
    "吹雪の中を歩く",
    "武器を抜く1",
    "武器を抜く2",
    "捉える1",
    "捉える2",
    "囚われる",
    "武器破壊1",
    "武器破壊2",
    "片手を前に出す",
    "片手を前に出して待機",
    "風神弓を掲げる",
    "木に寄りかかり座る",
    "木に寄りかかり座る→待機",
    "木に寄りかかり座る→立つ",
    "立ち_エンディング用",
    "頷く_エンディング用",
    "話す_エンディング用",
    "跪いて抱きかかえる",
    "跪いて抱きかかえる—泣く",
    "切腹死",
    "イベント用吹っ飛びダメージ",
    "イベント用攻撃モーション",
    "威嚇",
    "剣を調べる",
    "花をつける",
    "剣寸止め",
    "剣寸止め待機",
    "剣寸止め戻し",
    "リリス水に潜る1",
    "リリス水に潜る2",
    "リリス気付く",
    "ダメージ落下1",
    "ダメージ落下2",
    "落下中魔法攻撃",
    "座って話す3",
    "武器持ち待機",
    "武器持ち会話1",
    "武器持ち会話2",
    "あたりを見回す2",
    "リリス食事",
    "リリス喜ぶ",
    "店番_いらっしゃい",
    "店番_待機",
    "店番_ありがとう",
    "温泉_会話A1",
    "温泉_会話A2",
    "温泉_会話B1",
    "温泉_会話B2",
    "ポーズ1",
    "none2",
];

pub const FE15_ANIMATION_NAMES: &[&str] = &[
    "label",
    "IdleNormal",
    "IdleDying",
    "Attack1a",
    "Attack1b",
    "Attack2a",
    "Attack3a",
    "AttackT",
    "AttackC",
    "AttackF",
    "CounterN",
    "CounterE",
    "Backstep",
    "Charge",
    "Thanks",
    "Die",
    "Discharge",
    "DmgHig1",
    "DmgHig2",
    "DmgMid1",
    "DmgMid2",
    "DmgNon",
    "Repelled",
    "Run",
    "EvasionB",
    "EvasionL",
    "EvasionR",
    "TurnL",
    "TurnR",
    "Shoot",
    "ShootC",
    "Start",
    "Win",
    "Special1",
    "Final",
    "予備3",
    "予備4",
    "予備5",
    "TriangleA",
    "TriangleB",
    "TriangleC",
    "IdleNormalD",
    "WalkD",
    "RunD",
    "DashD",
    "TackleD",
    "StopD",
    "FindD",
    "Attack1D",
    "Attack2D",
    "Attack3D",
    "Attack4D",
    "Attack5D",
    "IdleStartD",
    "ClassChange",
    "EnterD",
    "Jump1D",
    "Jump2D",
    "Jump3D",
    "Unused1",
    "Unused2",
    "Unused3",
    "Unused4",
    "Unused5",
    "Unused6",
    "S01_OP_A",
    "S01_OP_B",
    "Unused7",
    "S02_EV01_A",
    "Unused8",
    "S03_BT01_A",
    "S03_BT01_B",
    "Unused9",
    "S03_BT02_A",
    "S03_BT02_B",
    "Unused10",
    "S05_EV_A",
    "S05_EV_B",
    "S05_EV_C",
    "Unused11",
    "S05_EV02_A",
    "S05_EV02_B",
    "Unused12",
    "S05_EV03_A",
    "S05_EV03_B",
    "S05_EV03_C",
    "Unused13",
    "S05_EV04_A",
    "Unused14",
    "T01_A",
    "T01_B",
    "Unused15",
    "T02_A",
    "T02_B",
    "Unused16",
    "S00_END_A",
    "S00_END_B",
];

pub struct ASetFile {
    pub meta: Option<String>,
    pub anim_clip_table: Vec<Option<String>>,
    pub sets: Vec<Vec<Option<String>>>,
}

impl ASetFile {
    pub fn new(meta: Option<String>) -> Self {
        ASetFile {
            meta,
            anim_clip_table: Vec::new(),
            sets: Vec::new(),
        }
    }

    pub fn from_archive(archive: &BinArchive) -> Result<Self> {
        let mut reader = BinArchiveReader::new(archive, 0);
        reader.skip(4);

        let anim_clip_table_address =
            archive
                .find_label_address("AnimClipNameTable")
                .ok_or_else(|| {
                    ArchiveError::OtherError("Bad aset file: no AnimClipTable label.".to_string())
                })?;

        let mut aset = ASetFile::new(reader.read_string()?);

        // Read anim clip table
        reader.seek(anim_clip_table_address);
        for _ in 0..257 {
            aset.anim_clip_table.push(reader.read_string()?);
        }

        // Read animation sets.
        while reader.tell() < archive.size() {
            let mut set = Vec::new();
            set.push(reader.read_label(0)?);
            let main_flags = reader.read_u32()?;
            for i in 0..8 {
                // Is this section present?
                if (main_flags & (1 << i)) != 0 {
                    let flags = reader.read_u32()?;
                    for bit in 0..32 {
                        if (flags & (1 << bit)) != 0 {
                            set.push(reader.read_string()?);
                        } else {
                            set.push(None);
                        }
                    }
                } else {
                    for _ in 0..32 {
                        set.push(None);
                    }
                }
            }
            aset.sets.push(set);
        }

        Ok(aset)
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        // Write the header.
        let mut archive = BinArchive::new(Endian::Little);
        archive.allocate_at_end(12);
        archive.write_u32(0, 4)?;
        archive.write_string(4, self.meta.as_deref())?;
        archive.write_u32(8, 0x100)?;

        // Write the anim clip table.
        archive.allocate_at_end(self.anim_clip_table.len() * 4);
        let mut writer = BinArchiveWriter::new(&mut archive, 12);
        writer.write_label("AnimClipNameTable")?;
        for name in &self.anim_clip_table {
            writer.write_string(name.as_deref())?;
        }

        // Write the animation sets.
        for set in &self.sets {
            // Generate the flags.
            let mut main_flags = 0;
            let mut flags_to_write = 0;
            let mut strings_to_write = 0;
            let mut compiled_flags = Vec::new();
            for flag_set in 0..8 {
                let mut set_flags = 0;
                for bit in 0..32 {
                    let index = flag_set * 32 + bit + 1;
                    let present = set
                        .get(index)
                        .map(|entry| entry.is_some())
                        .unwrap_or_default();
                    if present {
                        set_flags |= 1 << bit;
                        strings_to_write += 1;
                    }
                }
                compiled_flags.push(set_flags);
                if set_flags != 0 {
                    main_flags |= 1 << flag_set;
                    flags_to_write += 1;
                }
            }

            // Allocate space and write.
            writer.allocate_at_end((flags_to_write + strings_to_write + 1) * 4);
            if let Some(label) = &set[0] {
                writer.write_label(label)?;
            }
            writer.write_u32(main_flags)?;
            for (i, flag) in compiled_flags.iter().enumerate().take(8) {
                if *flag != 0 {
                    writer.write_u32(*flag)?;
                    for j in 0..32 {
                        let index = i * 32 + j + 1;
                        if let Some(v) = set.get(index).and_then(|entry| entry.as_deref()) {
                            writer.write_string(Some(v))?;
                        }
                    }
                }
            }
        }
        archive.serialize()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{utils::load_test_file, Endian};

    #[test]
    fn round_trip() {
        let file = load_test_file("FE14Aset_Test.bin");
        let archive = BinArchive::from_bytes(&file, Endian::Little).unwrap();
        let aset = ASetFile::from_archive(&archive);
        assert!(aset.is_ok());
        let aset = aset.unwrap();
        let bytes = aset.serialize().unwrap();
        assert_eq!(file, bytes);
    }
}
