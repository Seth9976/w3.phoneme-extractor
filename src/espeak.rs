//
// phoneme translator
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
extern crate espeak;

pub use self::espeak::ESpeak;
// ----------------------------------------------------------------------------
pub trait TextPhonemeTranslator {
    // ------------------------------------------------------------------------
    fn translate(&self, text: &str) -> Result<PhonemeResult, String>;
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use phonemes::{PhonemeResult, PhonemeSegment};
// ----------------------------------------------------------------------------
impl PhonemeSegment {
    // ------------------------------------------------------------------------
    fn new_from_generated(
        phoneme: &str,
        start: u32,
        word_position: usize,
        default_duration: u32,
    ) -> PhonemeSegment {
        // : indicates longer phoneme
        let (filtered, end) = if phoneme.contains('ː') {
            (phoneme.replace('ː', ""), start + default_duration * 2)
        } else if phoneme.contains('-') {
            // : indicates silent(?) phoneme
            (phoneme.replace('-', ""), start + default_duration / 4)
        } else {
            (phoneme.to_owned(), start + default_duration)
        };

        let weight = if filtered.contains('"') {
            // extra stress -> add some more weight
            1.15
        } else if filtered.contains('ˌ') {
            // secondary stress -> add less weight
            1.05
        } else if filtered.contains('ˈ') {
            // primary stress -> add some more weight
            1.1
        } else {
            // defaults
            1.0
        };
        // remove all stresses
        let filtered = filtered.replace(['ˈ', 'ˌ', '"', '^'], "");

        PhonemeSegment {
            phoneme: filtered,
            word_start: word_position == 0,
            start,
            end,
            weight,
            score: 1.0,
            matching_info: None,
            traceback: None,
            active: true,
            warnings: Vec::default(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TextPhonemeTranslator for ESpeak {
    // ------------------------------------------------------------------------
    fn translate(&self, text: &str) -> Result<PhonemeResult, String> {
        debug!("text to translate  : {}", text);

        let mut phonemes = Vec::new();

        // result will have blanks as word separator and _ as phoneme separator
        let result = match self.convert_to_phonemes(text, true) {
            Ok(string) => string,
            Err(why) => return Err(why),
        };
        let result = result.trim();

        // remove phoneme separator for debug output string
        let hypothesis = if !result.is_empty() {
            Some(result.replace('_', ""))
        } else {
            None
        };

        let words: Vec<&str> = result.split(' ').collect();

        // duration is saved ms (intervals more or less like pocketsphinx' frames)
        // use equidistant phoneme intervals starting with a silent one
        let (segment_duration, padding) = length_adapted_durations(result);
        let mut time = padding;
        for word in words {
            for (i, phoneme) in word
                .trim_matches('_')
                .split('_')
                .filter(|s| !s.is_empty())
                .enumerate()
            {
                // skip "empty" phonemes
                let p = PhonemeSegment::new_from_generated(phoneme, time, i, segment_duration);
                time = p.end;

                phonemes.push(p);
            }

            // add silence at end of every word
            time += segment_duration;
        }

        Ok(PhonemeResult {
            hypothesis,
            phonemes,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
fn length_adapted_durations(phoneme_track: &str) -> (u32, u32) {
    // strip of all stress markers
    let cleaned = phoneme_track.replace(['ˈ', 'ˌ', '"', 'ː'], "");

    // split into phonemes (note: there are multi char phonemes!)
    let phonemes = cleaned.replace('_', " ").split(' ').count() as u32;

    let default_duration = 75;

    let segment_duration = match phonemes {
        0..=20 => 100,
        21..=50 => 80,
        51..=75 => default_duration,
        _ => 73,
    };

    let duration = phonemes * segment_duration;

    // at least 1 second
    let padding = 1000u32.saturating_sub(duration).max(default_duration);

    (segment_duration, padding)
}
// ----------------------------------------------------------------------------
