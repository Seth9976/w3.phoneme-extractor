//
// gui::editor::view
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub mod misc;
pub mod table;
pub mod timeline;
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------

// actions
use super::{Action as EditorAction, TimelineAction};

// state
use super::{EditableData, PhonemeDragMode, Settings};

// misc
use super::{PhonemeSegment, PhonemeTrack};
use super::{MAX_ZOOM, MIN_ZOOM};

// util
use super::{Fonts, UiArea};
// ----------------------------------------------------------------------------
impl PhonemeSegment {
    // ------------------------------------------------------------------------
    fn color(&self, highlight: bool) -> (f32, f32, f32, f32) {
        match (self.warnings.is_empty(), highlight) {
            (true, true) => (1.0, 1.0, 0.0, 1.0),
            (true, false) => (1.0, 1.0, 1.0, 1.0),
            (false, true) => (1.0, 0.35, 1.0, 1.0),
            (false, false) => (1.0, 0.35, 0.35, 1.0),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
