//
// gui: extended phonemes datastructures for interactive adjustments
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
#[derive(Default, Clone)]
pub(super) struct PhonemeSegment {
    pub phoneme: String,
    pub word_start: bool,
    pub start: u32,
    pub end: u32,
    pub weight: f32,
    pub score: f32,
    pub matching_info: Option<String>,
    pub traceback: Option<String>,
    pub active: bool,
    pub warnings: Vec<QualityWarning>,
}
pub(super) use phonemes::PhonemeTrack;
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use phonemes::{PhonemeSegment as RawPhonemeSegment, QualityWarning};
// ----------------------------------------------------------------------------
impl From<&PhonemeTrack<RawPhonemeSegment>> for PhonemeTrack<PhonemeSegment> {
    // ------------------------------------------------------------------------
    fn from(track: &PhonemeTrack<RawPhonemeSegment>) -> PhonemeTrack<PhonemeSegment> {
        let phonemes = track.phonemes().iter().map(|p| p.into()).collect();
        let mut t = PhonemeTrack::new(
            track.id(),
            track.language(),
            track.input_text(),
            track.translation(),
            track.audio_hypothesis().clone(),
            track.actor().cloned(),
            phonemes,
        );
        t.set_version(track.version());
        t
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<&PhonemeTrack<PhonemeSegment>> for PhonemeTrack<RawPhonemeSegment> {
    // ------------------------------------------------------------------------
    fn from(track: &PhonemeTrack<PhonemeSegment>) -> PhonemeTrack<RawPhonemeSegment> {
        let phonemes = track.phonemes().iter().map(|p| p.into()).collect();
        let mut t = PhonemeTrack::new(
            track.id(),
            track.language(),
            track.input_text(),
            track.translation(),
            track.audio_hypothesis().clone(),
            track.actor().cloned(),
            phonemes,
        );
        t.set_version(track.version());
        t.assess_quality();
        t
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<&RawPhonemeSegment> for PhonemeSegment {
    // ------------------------------------------------------------------------
    fn from(raw: &RawPhonemeSegment) -> PhonemeSegment {
        PhonemeSegment {
            phoneme: raw.phoneme.clone(),
            word_start: raw.word_start,
            start: raw.start,
            end: raw.end,
            weight: raw.weight,
            score: raw.score,
            matching_info: raw.matching_info.clone(),
            traceback: raw.traceback.clone(),
            active: raw.active,
            warnings: raw.warnings.clone(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl From<&PhonemeSegment> for RawPhonemeSegment {
    // ------------------------------------------------------------------------
    fn from(seg: &PhonemeSegment) -> RawPhonemeSegment {
        RawPhonemeSegment {
            phoneme: seg.phoneme.clone(),
            word_start: seg.word_start,
            start: seg.start,
            end: seg.end,
            weight: seg.weight,
            score: seg.score,
            matching_info: seg.matching_info.clone(),
            traceback: seg.traceback.clone(),
            active: seg.active,
            warnings: seg.warnings.clone(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl PartialEq for PhonemeSegment {
    // ------------------------------------------------------------------------
    fn eq(&self, other: &PhonemeSegment) -> bool {
        // only the modifyable properties are required (and id)
        // self.word_start == other.word_start &&
        self.phoneme == other.phoneme &&
        self.start == other.start &&
        self.end == other.end &&
        self.weight == other.weight &&
        // self.score == other.score &&
        // self.matching_info == other.matching_info &&
        // self.traceback == other.traceback &&
        self.active == other.active
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
impl ::phonemes::PhonemeSegmentInterface for PhonemeSegment {
    #[inline(always)] fn phoneme(&self) -> &str { &self.phoneme }
    #[inline(always)] fn is_active(&self) -> bool { self.active }
    #[inline(always)] fn is_word_start(&self) -> bool { self.word_start }
    #[inline(always)] fn start(&self) -> u32 { self.start }
    #[inline(always)] fn end(&self) -> u32 { self.end }

    #[inline(always)] fn set_active(&mut self, state: bool) { self.active = state; }
    #[inline(always)] fn set_start(&mut self, value: u32) { self.start = value; }
    #[inline(always)] fn set_end(&mut self, value: u32) { self.end = value; }
}
// ----------------------------------------------------------------------------
