//
// interactive editing
//

// ----------------------------------------------------------------------------
// state
// ----------------------------------------------------------------------------
pub(super) struct EditableData {
    audio: AudioData,
    phonemetrack: PhonemeTrack<PhonemeSegment>,
    unmodified: PhonemeTrack<PhonemeSegment>,
    offset: usize,
    zoom: f32,
    start_marker: usize,
    end_marker: usize,
    dragging: Option<DragOperation>,
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct AudioData {
    samples: Vec<i16>,
    max: i16,
    rate: u32,
    duration: f32,
}
// ----------------------------------------------------------------------------
enum DragOperation {
    Waveform(UiDragging<usize>, f32),
    PhonemeBlock(PhonemeDragging),
}
// ----------------------------------------------------------------------------
struct PhonemeDragging {
    dragging: UiDragging<f32>,

    drag_start_slot: usize,
    border_points: Vec<(Controlpoint, Controlpoint)>,

    valid_range: (u32, u32),
}
// ----------------------------------------------------------------------------
struct Controlpoint {
    value: f32,
    weight: f32,
}
// ----------------------------------------------------------------------------
// actions
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub(super) enum Action {
    SetDragMode(PhonemeDragMode),
    SetActor(String),
    AutoCloseGaps,
    ActivatePhonemeSegment(usize, bool),
    SetPhonemeSegmentPos(usize, f32, f32),
    SetPhonemeSegmentWeight(usize, f32),
    Timeline(TimelineAction),
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub(super) enum TimelineAction {
    Zoom(f32),
    ZoomChange(f32, usize),
    DataOffset(usize),
    SetPlaybackStart(usize),
    SetPlaybackEnd(usize),
    WaveformDragStart(f32),
    PhonemeDragStart(usize, f32, f32),
    DragStop,
}
// ----------------------------------------------------------------------------
// view
// ----------------------------------------------------------------------------
pub(in gui) mod view;
// ----------------------------------------------------------------------------
// action processing
// ----------------------------------------------------------------------------
#[inline]
pub(super) fn running_actions_tick(
    settings: &Settings,
    state: &mut EditableData,
) -> Option<Action> {
    let mut drag_stopped = None;
    if let Some(ref mut active_dragging) = state.dragging {
        let drag_active = match *active_dragging {
            DragOperation::Waveform(ref mut drag, scaling) => {
                let samples = state.audio.samples.len();
                let poffset = &mut state.offset;
                let zoom = state.zoom;
                drag.update(|offset, delta| {
                    let max_offset = samples as f32 - samples as f32 / zoom;
                    *poffset = (*offset as f32 - delta.x * scaling)
                        .max(0.0)
                        .min(max_offset) as usize
                })
            }
            DragOperation::PhonemeBlock(ref mut drag) => {
                drag.update(state.phonemetrack.phonemes_mut(), settings.granularity_ms())
            }
        };
        if !drag_active {
            drag_stopped = Some(TimelineAction::DragStop.into());
        }
    }
    drag_stopped
}
// ----------------------------------------------------------------------------
#[inline]
pub(super) fn handle_action(
    action: Action,
    state: &mut EditableData,
    settings: &mut Settings,
    player: &mut player::Player,
) -> Option<::gui::Action> {
    update::handle_action(action, state, settings, player)
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui_widgets::UiDragging;

use super::player;

use super::phonemes::{PhonemeSegment, PhonemeTrack};
use super::settings::{PhonemeDragMode, Settings};
use super::support::Fonts;
use super::UiArea;
// ----------------------------------------------------------------------------
mod timeline;
mod update;
// ----------------------------------------------------------------------------
const MAX_ZOOM: f32 = 30.0;
const MIN_ZOOM: f32 = 1.0;
// ----------------------------------------------------------------------------
impl EditableData {
    // ------------------------------------------------------------------------
    pub fn new() -> EditableData {
        EditableData {
            audio: AudioData::default(),
            phonemetrack: PhonemeTrack::default(),
            unmodified: PhonemeTrack::default(),
            offset: 0,
            zoom: 1.0,
            start_marker: 0,
            end_marker: 0,
            dragging: None,
        }
    }
    // ------------------------------------------------------------------------
    pub fn reset(&mut self) {
        self.audio = AudioData::default();
        self.phonemetrack = PhonemeTrack::default();
        self.unmodified = PhonemeTrack::default();
        self.start_marker = 0;
        self.end_marker = 0;
        self.offset = 0;
        self.zoom = 1.0;
        self.dragging = None;
    }
    // ------------------------------------------------------------------------
    pub fn set_audio(&mut self, rawaudio: Vec<i16>, sample_rate: u32) {
        self.audio.set(rawaudio, sample_rate);
        self.start_marker = 0;
        self.end_marker = 0;
        self.offset = 0;
        self.zoom = 1.0;
    }
    // ------------------------------------------------------------------------
    pub fn set_phonemetrack(&mut self, track: PhonemeTrack<PhonemeSegment>) {
        self.unmodified = track.clone();
        self.phonemetrack = track;
    }
    // ------------------------------------------------------------------------
    pub fn set_offset(&mut self, offset: usize) {
        self.offset = usize::min(
            offset,
            self.audio.samples.len() - (self.audio.samples.len() as f32 / self.zoom) as usize,
        );
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }
    // ------------------------------------------------------------------------
    pub fn adjust_zoom_with_fixpoint(&mut self, amount: f32, fixture: usize) {
        let prev_zoom = self.zoom;
        let new_zoom = prev_zoom + prev_zoom * 0.25 * amount;
        self.set_zoom(new_zoom);
        // some math...
        let new_offset = self.offset as f32
            - fixture as f32 * ((prev_zoom - self.zoom) / (prev_zoom * self.zoom));

        self.set_offset(f32::max(0.0, new_offset) as usize);
    }
    // ------------------------------------------------------------------------
    pub fn set_playback_start_pos(&mut self, start: usize) {
        let start = usize::min(start, self.audio.samples.len());

        if self.start_marker >= self.end_marker {
            // reset endmarker to start if no region selected
            self.start_marker = start;
            self.end_marker = start;
        } else {
            self.start_marker = start;
        }
    }
    // ------------------------------------------------------------------------
    pub fn set_playback_end_pos(&mut self, end: usize) {
        let end = usize::min(end, self.audio.samples.len());

        if self.start_marker < end {
            self.end_marker = end;
        } else {
            // do not highlight in this case (it will be played up to the end)
            self.end_marker = self.start_marker;
        }
    }
    // ------------------------------------------------------------------------
    pub fn playback_start_pos(&self) -> usize {
        self.start_marker
    }
    // ------------------------------------------------------------------------
    pub fn playback_end_pos(&self) -> usize {
        if self.start_marker < self.end_marker {
            self.end_marker
        } else {
            // play to the end
            self.audio.samples.len()
        }
    }
    // ------------------------------------------------------------------------
    pub fn is_available(&self) -> bool {
        !self.audio.is_empty()
    }
    // ------------------------------------------------------------------------
    pub fn changed(&self) -> bool {
        self.unmodified != self.phonemetrack
    }
    // ------------------------------------------------------------------------
    pub fn phonemetrack(&self) -> &PhonemeTrack<PhonemeSegment> {
        &self.phonemetrack
    }
    // ------------------------------------------------------------------------
    pub fn unmodified(&self) -> &PhonemeTrack<PhonemeSegment> {
        &self.unmodified
    }
    // ------------------------------------------------------------------------
    pub fn set_as_saved(&mut self, new_version: u16) {
        self.phonemetrack.set_version(new_version);
        self.unmodified = self.phonemetrack.clone();
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl AudioData {
    // ------------------------------------------------------------------------
    #[inline]
    fn is_empty(&self) -> bool {
        self.samples.len() == 0
    }
    // ------------------------------------------------------------------------
    fn set(&mut self, rawaudio: Vec<i16>, rate: u32) {
        self.max = rawaudio
            .iter()
            .map(|i| i16::checked_abs(*i).unwrap_or(i16::MAX))
            .max()
            .unwrap_or(0);
        self.samples = rawaudio;
        self.rate = rate;
        self.duration = self.samples.len() as f32 / rate as f32;
    }
    // ------------------------------------------------------------------------
    fn duration_ms(&self) -> u32 {
        f32::trunc(self.duration * 1000.0) as u32
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl From<TimelineAction> for Action {
    fn from(action: TimelineAction) -> Action {
        Action::Timeline(action)
    }
}
// ----------------------------------------------------------------------------
// helper functions
// ----------------------------------------------------------------------------
#[inline]
fn clamp_ms(time: f32, granularity: f32, min: u32, max: u32) -> u32 {
    let x = (time / granularity).round() * granularity;
    if x < min as f32 {
        min
    } else if x > max as f32 {
        max
    } else {
        x as u32
    }
}
// ----------------------------------------------------------------------------
