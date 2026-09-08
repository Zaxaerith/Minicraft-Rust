/// Deterministic 2.2.4 `Sound` pointer mixer shared by the offline oracle and
/// the Windows device backend. Device sample conversion is outside this layer.
pub struct PcmMixer {
    clips: [Vec<i16>; 10],
    pointers: Vec<AudioPointer>,
}

#[derive(Debug, Clone, Copy)]
struct AudioPointer {
    clip: usize,
    offset: usize,
}

impl PcmMixer {
    pub fn new(clips: [Vec<i16>; 10]) -> Self {
        Self {
            clips,
            pointers: Vec::new(),
        }
    }

    pub fn play(&mut self, clip: usize) {
        self.pointers.push(AudioPointer { clip, offset: 0 });
    }

    pub fn mix(&mut self, sample_count: usize) -> Vec<i16> {
        let mut output = vec![0_i16; sample_count];
        self.mix_into(&mut output);
        output
    }

    /// Fills a caller-owned device buffer without allocating in the callback path.
    pub fn mix_into(&mut self, output: &mut [i16]) {
        output.fill(0);
        for sample in output {
            let mut count = 0_i32;
            let mut sum = 0_i32;
            let mut factor = 1.0_f64;
            self.pointers.retain_mut(|pointer| {
                let Some(value) = self.clips[pointer.clip].get(pointer.offset).copied() else {
                    return false;
                };
                pointer.offset += 1;
                let value = i32::from(value);
                let net = value.abs();
                if net < 8_192 {
                    factor *= (((8_192 - net) as f64 / 8_192.0).ln_1p() / std::f64::consts::LN_2
                        + 1.0)
                        .powf(1.0 / 3.0);
                }
                sum += value + 32_768;
                count += 1;
                true
            });
            if count == 0 {
                break;
            }
            let mut value = f64::from(sum) / f64::from(count) - 32_768.0;
            value *= factor.sqrt();
            let net = value.abs();
            const CUT: f64 = 32_768.0 * 0.9;
            if net > CUT {
                value =
                    (CUT + (net - CUT) / (net - CUT + 1.0) * (32_768.0 / 10.0)) * value.signum();
            }
            *sample = value as i16;
        }
    }

    pub fn offsets(&self) -> Vec<usize> {
        self.pointers.iter().map(|pointer| pointer.offset).collect()
    }
}
