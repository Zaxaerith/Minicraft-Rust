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

/// Asset adapter; mixing arithmetic and device scheduling remain unchanged.
pub(crate) struct JavaPcmMixer(minicraft_client::io::audio::PcmMixer);
impl JavaPcmMixer {
    pub(crate) fn new() -> Result<Self, String> {
        let clips = SoundEffect::ALL
            .into_iter()
            .map(decode_pcm)
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|clips: Vec<Vec<i16>>| {
                format!("expected 10 decoded clips, got {}", clips.len())
            })?;
        Ok(Self(minicraft_client::io::audio::PcmMixer::new(clips)))
    }
    pub(crate) fn play(&mut self, effect: SoundEffect) {
        self.0.play(effect as usize);
    }
    pub(crate) fn mix(&mut self, count: usize) -> Vec<i16> {
        self.0.mix(count)
    }
    pub(crate) fn mix_into(&mut self, output: &mut [i16]) {
        self.0.mix_into(output);
    }
    pub(crate) fn offsets(&self) -> Vec<usize> {
        self.0.offsets()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PcmCommand {
    Play(SoundEffect),
    Close,
}

/// One ordered command queue and one mixer, shared by tests and WASAPI.
/// Device-owned PCM frames are requested only after polling this queue.
pub(crate) struct PcmStream {
    mixer: JavaPcmMixer,
    receiver: std::sync::mpsc::Receiver<PcmCommand>,
    closed: bool,
    pub(crate) accepted_plays: u64,
}

impl PcmStream {
    pub(crate) fn channel() -> Result<(std::sync::mpsc::Sender<PcmCommand>, Self), String> {
        let (sender, receiver) = std::sync::mpsc::channel();
        Ok((
            sender,
            Self {
                mixer: JavaPcmMixer::new()?,
                receiver,
                closed: false,
                accepted_plays: 0,
            },
        ))
    }

    /// False means shutdown: no further device buffer should be acquired.
    pub(crate) fn poll_commands(&mut self) -> bool {
        if self.closed {
            return false;
        }
        loop {
            match self.receiver.try_recv() {
                Ok(PcmCommand::Play(effect)) => {
                    self.mixer.play(effect);
                    self.accepted_plays += 1;
                }
                Ok(PcmCommand::Close) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.closed = true;
                    return false;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return true,
            }
        }
    }

    pub(crate) fn fill(&mut self, output: &mut [i16]) {
        if self.closed {
            output.fill(0);
        } else {
            self.mixer.mix_into(output);
        }
    }
}

/// The engine's capacity minus its actual unread frames, not an inferred
/// application-side callback slot count. Invalid/fully occupied queues yield no write.
pub(crate) fn device_write_frames(capacity: u32, padding: u32) -> u32 {
    capacity.saturating_sub(padding)
}

pub fn validate_pcm_stream() -> Result<(), String> {
    let (sender, mut stream) = PcmStream::channel()?;
    sender
        .send(PcmCommand::Play(SoundEffect::Select))
        .map_err(|error| error.to_string())?;
    if !stream.poll_commands() {
        return Err("PCM stream closed before playback".to_owned());
    }
    let mut samples = [0_i16; 882];
    stream.fill(&mut samples);
    if stream.accepted_plays != 1
        || samples.iter().all(|sample| *sample == 0)
        || device_write_frames(1036, 595) != 441
    {
        return Err("PCM stream did not consume its first sound exactly once".to_owned());
    }
    sender
        .send(PcmCommand::Close)
        .map_err(|error| error.to_string())?;
    if stream.poll_commands() {
        return Err("PCM stream ignored close".to_owned());
    }
    stream.fill(&mut samples);
    if samples.iter().any(|sample| *sample != 0) {
        return Err("closed PCM stream produced audio".to_owned());
    }
    Ok(())
}

fn decode_pcm(effect: SoundEffect) -> Result<Vec<i16>, String> {
    let bytes = effect.bytes();
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(format!("{} is not a RIFF/WAVE image", effect.name()));
    }
    let mut channels = 0_u16;
    let mut rate = 0_u32;
    let mut bits = 0_u16;
    let mut data = None;
    let mut offset = 12_usize;
    while offset + 8 <= bytes.len() {
        let length = u32::from_le_bytes(
            bytes[offset + 4..offset + 8]
                .try_into()
                .map_err(|_| "invalid WAVE chunk length".to_owned())?,
        ) as usize;
        let start = offset + 8;
        let end = start
            .checked_add(length)
            .ok_or_else(|| "WAVE chunk length overflow".to_owned())?;
        if end > bytes.len() {
            return Err(format!("{} has a truncated WAVE chunk", effect.name()));
        }
        match &bytes[offset..offset + 4] {
            b"fmt " if length >= 16 => {
                let format = u16::from_le_bytes([bytes[start], bytes[start + 1]]);
                if format != 1 {
                    return Err(format!("{} is not PCM", effect.name()));
                }
                channels = u16::from_le_bytes([bytes[start + 2], bytes[start + 3]]);
                rate = u32::from_le_bytes(
                    bytes[start + 4..start + 8]
                        .try_into()
                        .map_err(|_| "invalid WAVE sample rate".to_owned())?,
                );
                bits = u16::from_le_bytes([bytes[start + 14], bytes[start + 15]]);
            }
            b"data" => data = Some(&bytes[start..end]),
            _ => {}
        }
        offset = end + (length & 1);
    }
    if rate != 44_100 || bits != 16 || !matches!(channels, 1 | 2) {
        return Err(format!(
            "{} is not 44.1 kHz, 16-bit mono/stereo PCM",
            effect.name()
        ));
    }
    let data = data.ok_or_else(|| format!("{} has no data chunk", effect.name()))?;
    let samples: Vec<i16> = data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|sample| i16::from_le_bytes(*sample))
        .collect();
    Ok(if channels == 1 {
        samples
            .into_iter()
            .flat_map(|sample| [sample, sample])
            .collect()
    } else {
        samples
    })
}

pub struct Audio {
    backend: platform::Backend,
}

/// Silent physical-device timing check; does not record microphones or system audio.
pub fn device_probe() -> Result<(), String> {
    platform::device_probe()
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
#[path = "audio/wasapi.rs"]
mod platform;

#[cfg(not(windows))]
mod platform {
    pub fn device_probe() -> Result<(), String> {
        Err("--audio-probe currently requires Windows WASAPI".to_owned())
    }

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
    use super::{JavaPcmMixer, PcmCommand, PcmStream, SoundEffect, device_write_frames};

    #[test]
    fn all_ten_copied_sound_assets_are_valid_wave_images() {
        assert_eq!(SoundEffect::ALL.len(), 10);
        super::validate_embedded_assets().unwrap();
    }

    #[test]
    fn java_pcm_mixer_keeps_independent_overlapping_pointers() {
        let mut mixer = JavaPcmMixer::new().unwrap();
        mixer.play(SoundEffect::Select);
        assert_eq!(mixer.offsets(), vec![0]);
        assert_eq!(mixer.mix(12).len(), 12);
        mixer.play(SoundEffect::Select);
        mixer.mix(32);
        assert_eq!(mixer.offsets(), vec![44, 32]);
        mixer.mix(4_096);
        assert!(mixer.offsets().is_empty());
    }

    #[test]
    fn caller_owned_mix_buffer_matches_the_allocating_parity_path() {
        let mut allocating = JavaPcmMixer::new().unwrap();
        let mut callback = JavaPcmMixer::new().unwrap();
        for effect in [
            SoundEffect::Select,
            SoundEffect::Confirm,
            SoundEffect::Craft,
        ] {
            allocating.play(effect);
            callback.play(effect);
        }
        let expected = allocating.mix(882);
        let mut actual = vec![i16::MAX; 882];
        callback.mix_into(&mut actual);
        assert_eq!(actual, expected);
        assert_eq!(callback.offsets(), allocating.offsets());
    }

    #[test]
    fn device_sized_blocks_do_not_repeat_or_skip_pcm_samples() {
        let mut reference = JavaPcmMixer::new().unwrap();
        let mut streaming = JavaPcmMixer::new().unwrap();
        for sound in SoundEffect::ALL {
            reference.play(sound);
            streaming.play(sound);
        }
        // Uneven device padding produces partial blocks; interleaved stereo
        // sample counts must preserve every sample, including clip exhaustion.
        let mut actual = Vec::new();
        for frames in [1036, 441, 440, 1, 882, 207, 441, 256, 65536, 65536] {
            let mut buffer = vec![i16::MAX; frames * 2];
            streaming.mix_into(&mut buffer);
            actual.extend(buffer);
        }
        assert_eq!(actual, reference.mix(actual.len()));
        assert_eq!(streaming.offsets(), reference.offsets());
    }

    #[test]
    fn padding_limits_writes_to_engine_owned_free_frames() {
        assert_eq!(device_write_frames(1036, 1036), 0);
        assert_eq!(device_write_frames(1036, 595), 441);
        assert_eq!(device_write_frames(1036, 0), 1036);
        assert_eq!(device_write_frames(1036, 2000), 0);
    }

    #[test]
    fn commands_are_consumed_once_before_device_refill() {
        let (sender, mut stream) = PcmStream::channel().unwrap();
        sender.send(PcmCommand::Play(SoundEffect::Confirm)).unwrap();
        assert!(stream.poll_commands());
        assert!(stream.poll_commands());
        let mut samples = [0; 882];
        stream.fill(&mut samples);
        assert_eq!(stream.accepted_plays, 1);
        assert_eq!(stream.mixer.offsets(), vec![882]);
        stream.fill(&mut samples);
        assert_eq!(stream.mixer.offsets(), vec![1764]);
    }

    #[test]
    fn pcm_command_channel_accepts_concurrent_independent_sounds() {
        let (sender, mut stream) = PcmStream::channel().unwrap();
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let sender = sender.clone();
                std::thread::spawn(move || {
                    sender.send(PcmCommand::Play(SoundEffect::Select)).unwrap()
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        assert!(stream.poll_commands());
        stream.fill(&mut [0; 882]);
        assert_eq!(stream.accepted_plays, 8);
        assert_eq!(stream.mixer.offsets(), vec![882; 8]);
    }

    #[test]
    fn close_prevents_later_commands_and_zeroes_reused_buffers() {
        let (sender, mut stream) = PcmStream::channel().unwrap();
        sender.send(PcmCommand::Close).unwrap();
        sender.send(PcmCommand::Play(SoundEffect::Select)).unwrap();
        assert!(!stream.poll_commands());
        assert!(!stream.poll_commands());
        let mut samples = [i16::MAX; 882];
        stream.fill(&mut samples);
        assert_eq!(samples, [0; 882]);
        assert_eq!(stream.accepted_plays, 0);
    }

    #[test]
    fn disconnected_pcm_command_channel_stops_device_writes() {
        let (sender, mut stream) = PcmStream::channel().unwrap();
        drop(sender);
        assert!(!stream.poll_commands());
    }
}
