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
    #[allow(dead_code)] // Used by the non-Windows backend and cross-platform asset tests.
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

    const fn bytes(self) -> &'static [u8] {
        match self {
            Self::BossDeath => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/sound/bossdeath.wav"
            )),
            Self::Confirm => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/sound/confirm.wav"
            )),
            Self::Craft => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/sound/craft.wav"
            )),
            Self::Death => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/sound/death.wav"
            )),
            Self::Explode => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/sound/explode.wav"
            )),
            Self::Fuse => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/sound/fuse.wav"
            )),
            Self::MonsterHurt => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/sound/monsterhurt.wav"
            )),
            Self::Pickup => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/sound/pickup.wav"
            )),
            Self::PlayerHurt => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/sound/playerhurt.wav"
            )),
            Self::Select => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/assets/sound/select.wav"
            )),
        }
    }
}

pub fn validate_embedded_assets() -> Result<(), String> {
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

pub struct Audio {
    backend: platform::Backend,
}

impl Default for Audio {
    fn default() -> Self {
        Self {
            backend: platform::Backend::new(),
        }
    }
}

impl Audio {
    pub fn play(&self, effect: SoundEffect, enabled: bool) {
        if !enabled {
            return;
        }
        self.backend.play(effect);
    }
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;

    use super::SoundEffect;

    const SND_ASYNC: u32 = 0x0001;
    const SND_NODEFAULT: u32 = 0x0002;
    const SND_MEMORY: u32 = 0x0004;

    #[link(name = "winmm")]
    unsafe extern "system" {
        fn PlaySoundW(sound: *const u16, module: *mut c_void, flags: u32) -> i32;
    }

    pub struct Backend;

    impl Backend {
        pub const fn new() -> Self {
            Self
        }

        pub fn play(&self, effect: SoundEffect) {
            // The bytes come from `include_bytes!`, so the memory remains valid for asynchronous
            // playback. PlaySound accepts a RIFF image when SND_MEMORY is supplied.
            unsafe {
                PlaySoundW(
                    effect.bytes().as_ptr().cast(),
                    std::ptr::null_mut(),
                    SND_ASYNC | SND_NODEFAULT | SND_MEMORY,
                );
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use sdl2::{
        Sdl,
        audio::{AudioCVT, AudioQueue, AudioSpecDesired, AudioSpecWAV},
        rwops::RWops,
    };

    use super::SoundEffect;

    pub struct Backend(Option<SdlAudio>);

    struct SdlAudio {
        _sdl: Sdl,
        queue: AudioQueue<i16>,
        clips: [Vec<i16>; 10],
    }

    impl Backend {
        pub fn new() -> Self {
            Self(SdlAudio::new())
        }

        pub fn play(&self, effect: SoundEffect) {
            let Some(audio) = &self.0 else {
                return;
            };
            let _ = audio.queue.queue_audio(&audio.clips[effect as usize]);
        }
    }

    impl SdlAudio {
        fn new() -> Option<Self> {
            let sdl = sdl2::init().ok()?;
            let subsystem = sdl.audio().ok()?;
            let queue = subsystem
                .open_queue::<i16, _>(
                    None,
                    &AudioSpecDesired {
                        freq: Some(44_100),
                        channels: Some(2),
                        samples: Some(1_024),
                    },
                )
                .ok()?;
            let clips = std::array::from_fn(|index| {
                decode(SoundEffect::ALL[index], queue.spec()).unwrap_or_default()
            });
            queue.resume();
            Some(Self {
                _sdl: sdl,
                queue,
                clips,
            })
        }
    }

    fn decode(effect: SoundEffect, destination: &sdl2::audio::AudioSpec) -> Option<Vec<i16>> {
        let mut source = RWops::from_bytes(effect.bytes()).ok()?;
        let wave = AudioSpecWAV::load_wav_rw(&mut source).ok()?;
        let converter = AudioCVT::new(
            wave.format,
            wave.channels,
            wave.freq,
            destination.format,
            destination.channels,
            destination.freq,
        )
        .ok()?;
        let bytes = converter.convert(wave.buffer().to_vec());
        Some(
            bytes
                .chunks_exact(2)
                .map(|sample| i16::from_ne_bytes([sample[0], sample[1]]))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::SoundEffect;

    #[test]
    fn all_ten_copied_sound_assets_are_valid_wave_images() {
        assert_eq!(SoundEffect::ALL.len(), 10);
        super::validate_embedded_assets().unwrap();
    }
}
