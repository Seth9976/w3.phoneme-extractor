//
// gui: audio player
//
extern crate rodio;

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(super) struct Player {
    _stream: rodio::OutputStream,
    sink: rodio::Sink,
    samplerate: u32,
    data: AudioData,
}
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub enum PlaybackState {
    Idle,
    Playing,
}
// ----------------------------------------------------------------------------
pub(super) fn init() -> Result<Player, String> {
    let (SampleRate(samplerate), _stream, sink) = init_playback_device()?;

    let playerdata = Player {
        _stream,
        sink,
        samplerate,
        data: AudioData::new(),
    };

    Ok(playerdata)
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use self::rodio::cpal::traits::{DeviceTrait, HostTrait};
use self::rodio::cpal::SampleRate;
use self::rodio::{source::SeekError, OutputStream, Sink, Source};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
// ----------------------------------------------------------------------------
static AUDIO_PLAY_POS: AtomicUsize = AtomicUsize::new(0);
// ----------------------------------------------------------------------------
#[derive(Clone)]
struct AudioData {
    audio: Vec<i16>,
    sample_rate: u32,
    startpos: usize,
    endpos: usize,
    datapos: usize,
}
// ----------------------------------------------------------------------------
impl Iterator for AudioData {
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.datapos < self.endpos {
            self.datapos += 1;
            AUDIO_PLAY_POS.store(self.datapos, Ordering::SeqCst);
            self.audio.get(self.datapos).copied()
        } else {
            None
        }
    }
}
// ----------------------------------------------------------------------------
impl Source for AudioData {
    // ------------------------------------------------------------------------
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn channels(&self) -> u16 {
        1
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(
            self.audio.len() as f32 / self.sample_rate() as f32,
        ))
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn try_seek(&mut self, _: Duration) -> Result<(), SeekError> {
        // TODO?
        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl AudioData {
    // ------------------------------------------------------------------------
    fn new() -> AudioData {
        AUDIO_PLAY_POS.store(0, Ordering::SeqCst);

        AudioData {
            audio: Vec::default(),
            sample_rate: 1,
            startpos: 0,
            endpos: 0,
            datapos: 0,
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn playpos(&self) -> usize {
        AUDIO_PLAY_POS.load(Ordering::SeqCst)
    }
    // ------------------------------------------------------------------------
    fn rewind(&mut self) {
        self.datapos = self.startpos;
        // self.playpos.store(startpos, Ordering::SeqCst);
    }
    // ------------------------------------------------------------------------
    fn seek(&mut self, pos: usize) {
        self.startpos = pos;
        self.rewind();
    }
    // ------------------------------------------------------------------------
    fn clip_at(&mut self, pos: usize) {
        if self.startpos < pos {
            self.endpos = pos;
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn set(&mut self, data: Vec<i16>, sample_rate: u32) -> Result<(), String> {
        self.datapos = 0;
        self.startpos = 0;
        self.endpos = data.len();
        self.audio = data;
        self.sample_rate = sample_rate;
        AUDIO_PLAY_POS.store(0, Ordering::SeqCst);
        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Player {
    // ------------------------------------------------------------------------
    #[inline]
    pub fn playback_samplerate(&self) -> u32 {
        self.samplerate
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn playpos(&self) -> usize {
        self.data.playpos()
    }
    // ------------------------------------------------------------------------
    pub fn state(&self) -> Result<PlaybackState, String> {
        if self.sink.empty() {
            Ok(PlaybackState::Idle)
        } else {
            Ok(PlaybackState::Playing)
        }
    }
    // ------------------------------------------------------------------------
    pub fn set_data(&mut self, data: Vec<i16>) -> Result<(), String> {
        self.sink.clear();
        self.data.set(data, self.samplerate)
    }
    // ------------------------------------------------------------------------
    pub fn seek(&mut self, pos: usize) {
        self.data.seek(pos)
    }
    // ------------------------------------------------------------------------
    pub fn clip_at(&mut self, pos: usize) {
        self.data.clip_at(pos)
    }
    // ------------------------------------------------------------------------
    pub fn play(&mut self) -> Result<(), String> {
        self.sink.stop();
        self.data.rewind();
        self.sink.append(self.data.clone());
        self.sink.play();

        Ok(())
    }
    // ------------------------------------------------------------------------
    pub fn stop(&self) -> Result<(), String> {
        self.sink.stop();
        AUDIO_PLAY_POS.store(0, Ordering::SeqCst);
        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// init
// ----------------------------------------------------------------------------
fn init_playback_device() -> Result<(SampleRate, OutputStream, Sink), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| String::from("player: failed to find a default audio device"))?;

    // preferred playback format is 44.1khz 16bit mono
    // but fallback to anything >= 44.1kHz is usable, too
    let config = device
        .supported_output_configs()
        .map_err(|e| format!("player: failed to get supported formats audio device: {e}"))?
        .fold(None, select_best_format)
        .and_then(|conf| {
            conf.try_with_sample_rate(SampleRate(44100))
                .or_else(|| conf.try_with_sample_rate(SampleRate(48000)))
                .or_else(|| Some(conf.with_max_sample_rate()))
        })
        .ok_or_else(|| {
            String::from(
            "player: no audio device found for playback (required playback sample rate >=44.1kHz)")
        })?;

    let sample_rate = config.sample_rate();

    let (_stream, stream_handle) = rodio::OutputStream::try_from_device_config(&device, config)
        .map_err(|e| format!("player: failed to create output stream ({e})"))?;

    let sink = rodio::Sink::try_new(&stream_handle)
        .map_err(|e| format!("player: failed to create new audio sink ({e})"))?;

    Ok((sample_rate, _stream, sink))
}

// ----------------------------------------------------------------------------
use self::rodio::cpal::SupportedStreamConfigRange;

fn select_best_format(
    best_found: Option<SupportedStreamConfigRange>,
    format: SupportedStreamConfigRange,
) -> Option<SupportedStreamConfigRange> {
    let cpal::SampleRate(max_sample_rate) = format.max_sample_rate();
    let channels = format.channels();

    trace!(
        "> available playback format: {max_sample_rate} Hz {channels} channels {:?} bit",
        format.sample_format()
    );

    if max_sample_rate >= 44100 {
        if let Some(ref current) = best_found {
            // try to match 44100 16bit mono
            if format.max_sample_rate() < current.max_sample_rate()
                || format.channels() < current.channels()
            {
                trace!(">> selecting [better samplingrate/channel count]");
                return Some(format);
            } else if format.sample_format() == cpal::SampleFormat::I16
                && format.max_sample_rate() == current.max_sample_rate()
                && format.channels() == current.channels()
            {
                trace!(">> selecting [better datatype]");
                return Some(format);
            }
        } else {
            trace!(">> selecting.");
            return Some(format);
        }
    }
    best_found
}
// ----------------------------------------------------------------------------
