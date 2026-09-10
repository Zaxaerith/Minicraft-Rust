//! Embedded Java sound effect IDs and WAVE validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SoundEffect {
    BossDeath,
    Confirm,
    Craft,
    Death,
    Explode,
    Fuse,
    MonsterHurt,
    Pickup,
    PlayerHurt,
    Select,
}

impl SoundEffect {
    #[allow(dead_code)]
    pub const ALL: [Self; 10] = [
        Self::BossDeath,
        Self::Confirm,
        Self::Craft,
        Self::Death,
        Self::Explode,
        Self::Fuse,
        Self::MonsterHurt,
        Self::Pickup,
        Self::PlayerHurt,
        Self::Select,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::BossDeath => "bossdeath",
            Self::Confirm => "confirm",
            Self::Craft => "craft",
            Self::Death => "death",
            Self::Explode => "explode",
            Self::Fuse => "fuse",
            Self::MonsterHurt => "monsterhurt",
            Self::Pickup => "pickup",
            Self::PlayerHurt => "playerhurt",
            Self::Select => "select",
        }
    }

    pub fn bytes(self) -> &'static [u8] {
        macro_rules! wav {
            ($name:literal) => {
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../assets/assets/sound/",
                    $name,
                    ".wav"
                ))
            };
        }
        match self {
            Self::BossDeath => wav!("bossdeath"),
            Self::Confirm => wav!("confirm"),
            Self::Craft => wav!("craft"),
            Self::Death => wav!("death"),
            Self::Explode => wav!("explode"),
            Self::Fuse => wav!("fuse"),
            Self::MonsterHurt => wav!("monsterhurt"),
            Self::Pickup => wav!("pickup"),
            Self::PlayerHurt => wav!("playerhurt"),
            Self::Select => wav!("select"),
        }
    }
}

pub fn validate_embedded_sounds() -> Result<(), String> {
    for effect in SoundEffect::ALL {
        let bytes = effect.bytes();
        if bytes.len() <= 44 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
            return Err(format!(
                "embedded sound {} is not a valid WAVE image",
                effect.name()
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_embedded_sounds;

    #[test]
    fn every_embedded_sound_is_a_wave_file() {
        validate_embedded_sounds().unwrap();
    }
}
