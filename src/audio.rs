//
// audio loader
//
extern crate fon;
extern crate hound;
extern crate lewton;

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub struct AudioData {
    pub values: Vec<i16>,
    pub resampled: bool,
}
// ----------------------------------------------------------------------------
pub struct AudioLoader;
pub struct AudioResampler;
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
#[derive(Eq, PartialEq)]
struct FormatSpec {
    channels: u8,
    sample_rate: u32,
    bits: u16,
}
// ----------------------------------------------------------------------------
impl FormatSpec {
    // ------------------------------------------------------------------------
    fn new(channels: u8, sample_rate: u32, bits: u16) -> FormatSpec {
        FormatSpec {
            channels,
            sample_rate,
            bits,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl AudioData {
    // ------------------------------------------------------------------------
    fn new(values: Vec<i16>, resampled: bool) -> AudioData {
        AudioData { values, resampled }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl AudioLoader {
    // ------------------------------------------------------------------------
    pub fn load(file: &str, target_samplerate: u32) -> Result<AudioData, String> {
        let (format, mut values): (_, Vec<i16>) = match file {
            s if s.ends_with(".wav") => {
                let decoder = WavDecoder::new(file)?;
                (decoder.format(), decoder.collect())
            }
            s if s.ends_with(".ogg") => {
                let decoder = OggDecoder::new(file)?;
                (decoder.format(), decoder.collect())
            }
            _ => {
                return Err(String::from(
                    "audioloader: found unsupported audio format (supported: wav, ogg)",
                ))
            }
        };

        let resample = Self::needs_resampling(&format, target_samplerate)?;

        if resample {
            values =
                AudioResampler::resample(&values, format.sample_rate, target_samplerate, false)?;
        }
        Ok(AudioData::new(values, resample))
    }
    // ------------------------------------------------------------------------
    fn needs_resampling(spec: &FormatSpec, target_samplerate: u32) -> Result<bool, String> {
        if *spec == FormatSpec::new(1, target_samplerate, 16)
            || *spec == FormatSpec::new(2, target_samplerate, 16)
        {
            Ok(false)
        } else if spec.channels <= 2 && spec.sample_rate >= target_samplerate {
            Ok(true)
        } else {
            Err(format!(
                "expected format >= {} Hz, 16 bit, 1 or 2 channel. \
                 found {} Hz, {} bit, {} channel(s)",
                target_samplerate, spec.sample_rate, spec.bits, spec.channels
            ))
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl AudioResampler {
    // ------------------------------------------------------------------------
    pub fn resample(
        values: &[i16],
        from_hz: u32,
        to_hz: u32,
        normalize: bool,
    ) -> Result<Vec<i16>, String> {
        use self::fon::chan::Ch16;
        use self::fon::Audio;

        let audio = if normalize {
            // simple max peak scaling
            let max = values
                .iter()
                .copied()
                .map(|i16| i16.clamp(i16::MIN + 1, i16::MAX).abs())
                .max()
                .unwrap_or(i16::MAX);
            let scale = i16::MAX as f32 * 0.95 / (max as f32);
            let normalized = values
                .iter()
                .copied()
                .map(|v| (v as f32 * scale).round() as i16)
                .collect::<Vec<_>>();

            Audio::<Ch16, 1>::with_i16_buffer(from_hz, normalized)
        } else {
            Audio::<Ch16, 1>::with_i16_buffer(from_hz, values)
        };

        let mut audio = Audio::<Ch16, 1>::with_audio(to_hz, &audio);
        Ok(audio.as_i16_slice().to_vec())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// inspired by rodios decoder implementations
// ----------------------------------------------------------------------------
// Wav
// ----------------------------------------------------------------------------
use std::fs::File;
use std::io::BufReader;

struct WavDecoder {
    reader: hound::WavReader<BufReader<File>>,
    channels: u16,
}
// ----------------------------------------------------------------------------
impl WavDecoder {
    // ------------------------------------------------------------------------
    fn new(filename: &str) -> Result<WavDecoder, String> {
        let reader = hound::WavReader::open(filename).map_err(|e| format!("WavLoader: {}", e))?;
        let channels = reader.spec().channels;

        if channels > 1 {
            warn!("{filename} contains {channels} channels. decoding only channel #{channels}!");
        }

        Ok(WavDecoder { reader, channels })
    }
    // ------------------------------------------------------------------------
    fn format(&self) -> FormatSpec {
        self.reader.spec().into()
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Iterator for WavDecoder {
    type Item = i16;
    // ------------------------------------------------------------------------
    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.channels > 1 {
            self.reader
                .samples()
                .nth(self.channels.saturating_sub(1) as usize)
                .map(|value| value.unwrap_or(0))
        } else {
            self.reader.samples().next().map(|value| value.unwrap_or(0))
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<hound::WavSpec> for FormatSpec {
    // ------------------------------------------------------------------------
    fn from(spec: hound::WavSpec) -> FormatSpec {
        FormatSpec {
            channels: spec.channels as u8,
            sample_rate: spec.sample_rate,
            bits: spec.bits_per_sample,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// Ogg Vorbis
// ----------------------------------------------------------------------------
use self::lewton::inside_ogg::OggStreamReader;
use std::vec;

struct OggDecoder {
    reader: OggStreamReader<File>,
    packet_data: vec::IntoIter<i16>,
}
// ----------------------------------------------------------------------------
impl OggDecoder {
    // ------------------------------------------------------------------------
    fn new(filename: &str) -> Result<OggDecoder, String> {
        let f = File::open(filename).map_err(|e| format!("OggLoader: {}", e))?;
        let mut reader = OggStreamReader::new(f).map_err(|e| format!("OggLoader: {}", e))?;

        // The first packet is always empty => prefetch directly
        let data = reader
            .read_dec_packet_itl()
            .ok()
            .and_then(|v| v)
            .unwrap_or_default();

        let channels = reader.ident_hdr.audio_channels;
        if channels > 1 {
            warn!("{filename} contains {channels} channels. decoding only channel #{channels}!");
        }

        Ok(OggDecoder {
            reader,
            packet_data: data.into_iter(),
        })
    }
    // ------------------------------------------------------------------------
    fn format(&self) -> FormatSpec {
        FormatSpec {
            channels: self.reader.ident_hdr.audio_channels,
            sample_rate: self.reader.ident_hdr.audio_sample_rate,
            bits: 16,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Iterator for OggDecoder {
    type Item = i16;
    // ------------------------------------------------------------------------
    #[inline]
    fn next(&mut self) -> Option<i16> {
        if let Some(value) = self.packet_data.next() {
            Some(value)
        } else {
            // read next packet
            self.packet_data = match self.reader.read_dec_packet() {
                Err(msg) => {
                    error!("OggLoader: {}", msg);
                    return None;
                }
                Ok(packet) => match packet {
                    Some(mut data) => match data.drain(..).last() {
                        Some(channel) => channel.into_iter(),
                        None => return None,
                    },
                    None => return None,
                },
            };
            self.packet_data.next()
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
