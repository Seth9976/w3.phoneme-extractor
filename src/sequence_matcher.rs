//
// phoneme sequence matcher
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub const WARN_MATCHING_SCORE_MIN: f32 = 0.0;

pub struct SequenceMatcher {
    similarity: SimilarityMatrix,
    /// highest score pairs from sim matrix (mapping ipa -> pocketsphnix phoneme)
    mapping: HashMap<String, String>,
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::f32;
use std::fmt;
use std::collections::HashMap;

use matrix::{DebugMatrix2D, Matrix2D};
use phonemes::{PhonemeResult, PhonemeSegment};
use similarity_matrix::SimilarityMatrix;
// ----------------------------------------------------------------------------

#[derive(PartialEq, Clone, Copy, Debug)]
enum AlignmentOperation {
    Match = 0,
    Delete = 1,
    MergeLeft = 2,
    //    MergeRight = 3,
    Insert = 4,
    SplitLeft = 5,
    //    SplitRight = 6,
}
// ----------------------------------------------------------------------------
#[derive(Clone, Copy)]
struct Score {
    score: f32,
    total: f32,
    op: Option<AlignmentOperation>,
    _debug: [f32; 7],
}
// ----------------------------------------------------------------------------
impl Default for Score {
    fn default() -> Score {
        Score {
            score: f32::MIN,
            total: f32::MIN,
            op: None,
            _debug: [f32::MIN; 7],
        }
    }
}
// ----------------------------------------------------------------------------
impl Score {
    fn set(&mut self, op: AlignmentOperation, score: f32, prev_total: f32) {
        if prev_total + score > self.total {
            self.total = prev_total + score;
            self.score = score;
            self.op = Some(op);
        }
        self._debug[op as usize] = score;
    }
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct SequenceAlignmentString {
    audio: Vec<String>,
    text: Vec<String>,
    ops: Vec<String>,
    score: Vec<String>,
    traceback: Vec<(String, AlignmentOperation, String)>,
}
impl SequenceAlignmentString {
    // ------------------------------------------------------------------------
    fn push(
        &mut self,
        a: Option<&PhonemeSegment>,
        t: Option<&PhonemeSegment>,
        alignment_info: &Score,
    ) {
        let gap = String::from("_");

        let text_phoneme = match t {
            Some(t) => t.phoneme.as_str(),
            None => gap.as_str(),
        };

        let (audio_phoneme, audio_timing) = match a {
            Some(a) => (
                a.phoneme.as_str(),
                format!("{: <3}[{: >5} -{: >5}]", a.phoneme, a.start, a.end),
            ),
            None => (gap.as_str(), String::from("")),
        };
        let op = alignment_info
            .op
            .unwrap_or_else(|| fatal!("got aligment info without op!"));

        self.audio.push(format!("{: ^4}", audio_phoneme));
        self.text.push(format!("{: ^4}", text_phoneme));
        self.ops.push(format!(" {}  ", op));
        self.score.push(format!("{:4.1}", alignment_info.total));

        self.traceback
            .push((text_phoneme.to_owned(), op, audio_timing));
    }
    // ------------------------------------------------------------------------
    fn get_last_traceback(&self) -> String {
        match self.traceback.last() {
            Some(info) => match info.1 {
                AlignmentOperation::MergeLeft => {
                    let prev = &self.traceback[self.traceback.len() - 2];
                    format!(
                        " {: <2} {}{} {} + {}",
                        prev.0, prev.1, info.1, prev.2, info.2
                    )
                }
                op => format!(" {: <2} {}  {}", info.0, op, info.2),
            },
            None => "".to_owned(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl fmt::Display for SequenceAlignmentString {
    // ------------------------------------------------------------------------
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\naudio: {}\ntext:  {}\nop:    {}\nscore: {}",
            self.audio.join("|"),
            self.text.join("|"),
            self.ops.join("|"),
            self.score.join("|")
        )
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
const MAX_PHONEME_ALTERNATIVES: usize = 25;
// ----------------------------------------------------------------------------
impl SequenceMatcher {
    // ------------------------------------------------------------------------
    pub fn new(similarity_matrix: SimilarityMatrix) -> SequenceMatcher {
        let mapping = similarity_matrix
            .sorted_scores()
            .drain(..)
            .map(|(key, mut scores)| {
                if scores.len() > 1 {
                    let alternatives = scores
                        .drain(..)
                        .take(MAX_PHONEME_ALTERNATIVES)
                        .collect::<Vec<_>>()
                        .join("|");
                    (key, format!("({alternatives})"))
                } else {
                    (key, scores.join(""))
                }
            })
            .collect();

        SequenceMatcher {
            similarity: similarity_matrix,
            mapping,
        }
    }
    // ------------------------------------------------------------------------
    fn calculate_score_matrix(
        &self,
        audio: &[PhonemeSegment],
        text: &[PhonemeSegment],
    ) -> Matrix2D<Score> {
        // reserve space (include additional first row & col)
        let a_len = audio.len();
        let t_len = text.len();
        let width = a_len + 1;
        let height = t_len + 1;
        let mut matrix: Matrix2D<Score> = Matrix2D::new(width, height);
        // println!("a.len {}", a_len);
        // for a in audio {
        // println!("AUDIO PHONEMES {:?}", &a.phoneme);
        // }
        // small sequences, performance not that important
        // top left score must be zero
        matrix[(0, 0)].score = 0.0;
        matrix[(0, 0)].total = 0.0;

        // special case: first row + col
        for a in 1..width {
            let prev_idx = (a - 1, 0);
            let prev_total = matrix[prev_idx].total;
            let p_a = &audio[a - 1].phoneme;

            let current = &mut matrix[(a, 0)];
            current.set(
                AlignmentOperation::Delete,
                self.similarity.get_delete_score(p_a),
                prev_total,
            );
        }

        for t in 1..height {
            let prev_idx = (0, t - 1);
            let prev_total = matrix[prev_idx].total;
            let p_t = &text[t - 1].phoneme;

            let current = &mut matrix[(0, t)];
            current.set(
                AlignmentOperation::Insert,
                self.similarity.get_insert_score(p_t),
                prev_total,
            );
        }

        // -- addittional alignment constraints if audio gaps (silence) are detected

        // as this is only for detecting gaps within words ignore starting gap
        let mut last_end = audio[0].end as i32;
        let preceding_audio_gaps = audio
            .iter()
            .map(|p| {
                let is_gap = p.start as i32 > last_end;
                last_end = p.end as i32;
                is_gap
            })
            .collect::<Vec<_>>();

        for t in 1..height {
            for a in 1..width {
                let mut score = Score::default();

                // zero based index
                let text_idx = t - 1;
                let audio_idx = a - 1;

                // currently tested audio phoneme
                let p_a = &audio[audio_idx].phoneme;
                // currently tested text phoneme
                let p_t = &text[text_idx].phoneme;

                // ------ some precalculated audio-gap alignment flags ----------------------------

                // is there a gap before currently tested audio phonene (-> word start)?
                let flag_preceding_gap = preceding_audio_gaps[audio_idx];

                // is the next detected audio phoneme after a silence (-> word ends/next word)
                let flag_audio_next_phoneme_after_gap = preceding_audio_gaps
                    .get(audio_idx + 1)
                    .copied()
                    .unwrap_or(false);
                let flag_audio_word_end_gap_follows = flag_audio_next_phoneme_after_gap;

                // is the next tested text phoneme a word start (-> current text phoneme is word end)?
                let flag_next_text_phoneme_is_word_start =
                    text.get(text_idx + 1).map(|p| p.word_start).unwrap_or(true);

                let flag_text_word_does_not_end_yet = !flag_next_text_phoneme_is_word_start;

                // audio phoneme follows a silence (-> most probably a new word)
                // but text phoneme is not a word start (-> phoneme is within a word)
                let flag_audio_gap_but_still_within_word =
                    flag_preceding_gap && !text[text_idx].word_start;

                // do not apply any audio-gap constraints for single phoneme words
                let flag_text_single_phoneme_word =
                    text[text_idx].word_start && flag_next_text_phoneme_is_word_start;

                // starting text word contains at least 2 phonemes
                let flag_text_word_with_multiple_phonemes_start =
                    text[text_idx].word_start && flag_text_word_does_not_end_yet;

                // audio word ends with a gap but text word started
                let flag_audio_word_end_misaligned_with_text_word_start =
                    flag_audio_word_end_gap_follows && flag_text_word_with_multiple_phonemes_start;
                // --------------------------------------------------------------------------------

                // --- match
                let prev_idx = (a - 1, t - 1);

                // penalty for matches in the vicinity of detected audio gaps
                let op_score = if !flag_text_single_phoneme_word
                    && (flag_audio_gap_but_still_within_word
                        || flag_audio_word_end_misaligned_with_text_word_start)
                {
                    // e.g.:
                    //  a: A|CB
                    //  t: A.CB
                    self.similarity.get_gap_within_word_score(p_a, p_t)
                } else {
                    //  a: ACB
                    //  t: ACB
                    self.similarity.get_score(p_a, p_t)
                };
                score.set(AlignmentOperation::Match, op_score, matrix[prev_idx].total);

                // --- deletes
                //  a: ACB
                //  t: A_B
                let prev_idx = (a - 1, t);
                score.set(
                    AlignmentOperation::Delete,
                    self.similarity.get_delete_score(p_a),
                    matrix[prev_idx].total,
                );

                // instead of making gaps favor extension of previous text
                // phoneme - but only if previous is not already a gap/gap
                // filler
                if matrix[prev_idx].op == Some(AlignmentOperation::Match) {
                    // do not match + merge over audio gaps (phonemes most probably belong to
                    // different words with a silence in between)
                    let op_score = if flag_preceding_gap {
                        //  a: A|CB   A|CB
                        //  t: A_B -> A|_B
                        self.similarity.get_merge_left_score_over_gap(p_a, p_t)
                    } else {
                        //  a: ACB    ACB
                        //  t: A_B -> AaB
                        self.similarity.get_merge_left_score(p_a, p_t)
                    };

                    score.set(
                        AlignmentOperation::MergeLeft,
                        op_score,
                        matrix[prev_idx].total,
                    );
                }
                // FIXME: MergeRight needs a more sophisticated/different
                // apply_alignment to allow for prefetching and merging of
                // text phoneme. also how to test for gap extension? (multipass?).
                // Maybe later...
                //
                // if t < t_len {
                //     //  a: ACB    ACB
                //     //  t: A_B -> AbB
                //     let next_t = &text[t].phoneme;
                //     score.set(AlignmentOperation::MergeRight,
                //         self.similarity.get_merge_right_score(p_a, next_t),
                //         matrix[prev_idx].total);
                // }

                // --- inserts
                //  a: A_B
                //  t: ACB
                let prev_idx = (a, t - 1);
                score.set(
                    AlignmentOperation::Insert,
                    self.similarity.get_insert_score(p_t),
                    matrix[prev_idx].total,
                );

                // instead of making insert favor split of previous/next audio
                // phoneme - but only if previous is not already a gap/gap filler
                if matrix[prev_idx].op == Some(AlignmentOperation::Match) {

                    // do not split phonemes over audio gaps
                    let op_score = if flag_text_word_does_not_end_yet && flag_audio_next_phoneme_after_gap {
                        //  a: A_|B   A_|B
                        //  t: ACB -> ACB
                        self.similarity.get_split_left_gap_within_word_score(p_a, p_t)
                    } else {
                        //  a: A_B    aaB
                        //  t: ACB -> ACB
                        self.similarity.get_split_left_score(p_a, p_t)
                    };
                    score.set(
                        AlignmentOperation::SplitLeft,
                        op_score,
                        matrix[prev_idx].total,
                    );
                }

                //println!("{}x{} {:?}", a, t, &score);

                matrix[(a, t)] = score;
            }
        }
        trace!(
            "> ScoreMatrix: {}",
            DebugMatrix2D::new(&matrix, audio, text)
        );

        matrix
    }
    // ------------------------------------------------------------------------
    fn calculate_alignment(&self, score_matrix: Matrix2D<Score>) -> Result<Vec<Score>, String> {
        let mut ops = Vec::with_capacity(score_matrix.height);

        // backtrace (one!) best path from (width, height) to (0,0)
        let mut a = score_matrix.width - 1;
        let mut t = score_matrix.height - 1;
        while a > 0 || t > 0 {
            let score = score_matrix[(a, t)];
            //warn!("a,t: ({},{}) {:?}:{}", a, t, score.op.unwrap(), score.score);
            match score.op {
                Some(ref op) => match *op {
                    AlignmentOperation::Match => {
                        a -= 1;
                        t -= 1;
                    }
                    AlignmentOperation::Delete => {
                        a -= 1;
                    }
                    AlignmentOperation::MergeLeft => {
                        a -= 1;
                    }
                    //AlignmentOperation::MergeRight => { a -= 1; },
                    AlignmentOperation::Insert => {
                        t -= 1;
                    }
                    AlignmentOperation::SplitLeft => {
                        t -= 1;
                    }
                    //AlignmentOperation::SplitRight => { t -= 1; },
                },
                None => {
                    return Err(format!(
                        "found score matrix element without alignment \
                         op at ({}, {}).",
                        a, t
                    ))
                }
            }

            ops.push(score);
        }

        ops.reverse();
        Ok(ops)
    }
    // ------------------------------------------------------------------------
    fn apply_alignment(
        &self,
        audio: &[PhonemeSegment],
        align_ops: &[Score],
        text: &[PhonemeSegment],
    ) -> Result<(Vec<PhonemeSegment>, f32, bool, SequenceAlignmentString), String> {
        let mut result = Vec::new();
        let mut seq = SequenceAlignmentString::default();
        let mut a = 0;
        let mut t = 0;
        let mut has_gaps = false;
        let mut min_score = f32::MAX;

        for (i, align_info) in align_ops.iter().enumerate() {

            let mut score = align_info.score;

            let (phoneme, new_word_starting, weight, start, end, active) = match align_info.op {
                Some(op) => match op {
                    AlignmentOperation::Match => {
                        let p_a = &audio[a];
                        let p_t = &text[t];

                        seq.push(Some(p_a), Some(p_t), align_info);
                        a += 1;
                        t += 1;

                        (
                            p_t.phoneme.clone(),
                            p_t.word_start,
                            p_t.weight,
                            p_a.start,
                            p_a.end,
                            true,
                        )
                    }
                    AlignmentOperation::Delete => {
                        let p_a = &audio[a];

                        seq.push(Some(p_a), None, align_info);
                        a += 1;
                        // special cases:
                        // - SIL never matches output from translator but it is
                        //   important to enforce silence and prevent neighbor
                        //   phoneme "bleeding" (aka merges)
                        // - +NSN+ and +SPN+ are "noise phonemes" that won't match
                        //   anything from translator, too
                        match p_a.phoneme.to_lowercase().as_str() {
                            "sil" | "+nsn+" | "+spn+" | "<sil>" | "</s>" => continue,
                            _ => {
                                // no gap warning trigger for above special cases
                                has_gaps = true;

                                // word start can only be indicated from text phonemes
                                // inactive as those are unknown
                                ("_".to_owned(), false, 1.0, p_a.start, p_a.end, false)
                            }
                        }
                    }
                    AlignmentOperation::MergeLeft => {
                        assert!(
                            !result.is_empty(),
                            "MergeLeft alignment failed for \
                             empty result phoneme trail!"
                        );

                        let p_a = &audio[a];
                        // extend previous text phoneme to include this audio phoneme
                        let prev_t: PhonemeSegment = result.pop().unwrap();

                        seq.push(Some(p_a), Some(&prev_t), align_info);
                        a += 1;

                        // score is adjusted as it contains "two" scores
                        score = (prev_t.score + align_info.score) / 2.0;

                        (
                            prev_t.phoneme,
                            prev_t.word_start,
                            prev_t.weight,
                            prev_t.start,
                            p_a.end,
                            true,
                        )
                    }
                    // AlignmentOperation::MergeRight => {
                    //     seq.push(&p_a.phoneme, &p_t.phoneme, op, score);
                    //     a += 1;
                    //     // FIXME
                    //     unimplemented!();
                    // },
                    AlignmentOperation::Insert => {
                        let p_t = &text[t];
                        has_gaps = true;
                        seq.push(None, Some(p_t), align_info);
                        t += 1;
                        let time = match result.last() {
                            Some(prev) => prev.end,
                            None => 0,
                        };
                        // inactive since they have a duration of 0
                        (
                            p_t.phoneme.clone(),
                            p_t.word_start,
                            p_t.weight,
                            time,
                            time,
                            false,
                        )
                    }
                    AlignmentOperation::SplitLeft => {
                        let prev_audio = &audio[a - 1];
                        let p_t = &text[t];

                        seq.push(Some(prev_audio), Some(p_t), align_info);
                        t += 1;

                        // take half the timing slot of previous audio phoneme
                        // adjust previous phoneme end time
                        let duration = prev_audio.end - prev_audio.start;
                        let half_time = prev_audio.start + duration / 2;

                        if let Some(seg) = result.last_mut() {
                            seg.end = half_time;
                        }
                        (
                            p_t.phoneme.clone(),
                            p_t.word_start,
                            p_t.weight,
                            half_time,
                            prev_audio.end,
                            true,
                        )
                    }
                    // AlignmentOperation::SplitRight => {
                    //     let p_a = &audio[a];
                    //     let p_t = &text[t];

                    //     seq.push(Some(p_a), Some(p_t), align_info);
                    //     t += 1;
                    //     // take half the timing slot of "next" audio phoneme
                    //     //TODO adjust next textphoneme?
                    //     let duration = p_a.end - p_a.start;
                    //     (p_t.phoneme.clone(), p_t.weight,
                    //         p_a.start + duration / 2, p_a.end)
                    // },

                    //_ => unimplemented!(),

                },
                None => return Err(format!("found alignment op without op at {}!", i)),
            };

            // println!("traceback: #{} ({}, {}) [{:3.1}]: {}",
            //     i, a, t, score, &seq.get_last_traceback());

            result.push(PhonemeSegment {
                phoneme,
                word_start: new_word_starting,
                start,
                end,
                weight,
                score,
                matching_info: Some(seq.get_last_traceback()),
                traceback: None,
                active,
                warnings: Vec::default(),
            });
            // required for suggesting manual adjustments below a threshold
            min_score = min_score.min(score);
        }

        debug!("> Aligned sequences:{}", seq);

        Ok((result, min_score, has_gaps, seq))
    }
    // ------------------------------------------------------------------------
    pub fn phoneme_pairing_alternatives(&self) -> &HashMap<String, String> {
        &self.mapping
    }
    // ------------------------------------------------------------------------
    pub fn calculate_matching(
        &self,
        lineid: u32,
        audio: &PhonemeResult,
        text: &PhonemeResult,
    ) -> Result<PhonemeResult, String> {
        // "optimal" global alignment based on Needleman-Wunsch
        // https://en.wikipedia.org/wiki/Needleman%E2%80%93Wunsch_algorithm
        let score_matrix = self.calculate_score_matrix(&audio.phonemes, &text.phonemes);
        let ops = match self.calculate_alignment(score_matrix) {
            Ok(ops) => ops,
            Err(why) => return Err(why),
        };

        let (phonemes, min_score, has_gaps, _) =
            match self.apply_alignment(&audio.phonemes, &ops, &text.phonemes) {
                Ok(result) => result,
                Err(why) => return Err(why),
            };

        if has_gaps {
            warn!(
                "id {lineid:010}: automatic sequence alignment failed and created matching \
                 with gaps! manual adjustment in phoneme file is highly \
                 recommended!"
            );
        }
        if min_score < WARN_MATCHING_SCORE_MIN {
            warn!(
                "id {lineid:010}: automatic sequence alignment matched some phonemes with \
                 scores below {}. manual adjustment in phoneme file is highly \
                 recommended!",
                WARN_MATCHING_SCORE_MIN
            );
        }

        Ok(PhonemeResult {
            hypothesis: text.hypothesis.clone(),
            phonemes,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
impl fmt::Display for AlignmentOperation {
    // ------------------------------------------------------------------------
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                AlignmentOperation::Match => "~",
                AlignmentOperation::Delete => "d",
                AlignmentOperation::MergeLeft => ">",
                //AlignmentOperation::MergeRight => "<",
                AlignmentOperation::Insert => "i",
                AlignmentOperation::SplitLeft => "\\",
                //AlignmentOperation::SplitRight => "/",
            }
        )
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[cfg(test)]
#[allow(unused_must_use)]
fn fmt_debug_opscore(f: &mut fmt::Formatter, op: AlignmentOperation, score: f32) {
    if score > f32::MIN {
        write!(f, " {}:{:4.1} ", op, score);
    } else {
        write!(f, " {}:     ", op);
    }
}
#[cfg(test)]
#[allow(unused_must_use)]
impl fmt::Debug for Score {
    // ------------------------------------------------------------------------
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // all scores
        fmt_debug_opscore(f, AlignmentOperation::Match, self._debug[0]);
        fmt_debug_opscore(f, AlignmentOperation::Delete, self._debug[1]);
        fmt_debug_opscore(f, AlignmentOperation::MergeLeft, self._debug[2]);
        //fmt_debug_opscore(f, AlignmentOperation::MergeRight, self._debug[3]);
        fmt_debug_opscore(f, AlignmentOperation::Insert, self._debug[4]);
        fmt_debug_opscore(f, AlignmentOperation::SplitLeft, self._debug[5]);
        //fmt_debug_opscore(f, AlignmentOperation::SplitRight, self._debug[6]);

        write!(f, " {:?}", self.op);
        if self.total > f32::MIN {
            write!(f, " total: {:4.1}", self.total);
        } else {
            write!(f, " total:      ");
        }
        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl fmt::Display for Score {
    // ------------------------------------------------------------------------
    #[cfg(test)]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
    // ------------------------------------------------------------------------
    #[cfg(not(test))]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.total > -5.0 {
            write!(f, "{:4.1}", self.total)
        } else {
            write!(f, "    ")
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl fmt::Display for PhonemeSegment {
    // ------------------------------------------------------------------------
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{: ^4}", &self.phoneme)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::super::matrix::DebugMatrix2D;
    use super::super::similarity_matrix::SimilarityMatrix;
    use super::{PhonemeResult as PSeq, PhonemeSegment as P};

    use super::AlignmentOperation;
    use super::AlignmentOperation::*;
    use super::{Score, SequenceMatcher};

    impl PSeq {
        fn from_str(phonemes: &str) -> PSeq {
            let mut pos = 0;
            PSeq {
                hypothesis: None,
                phonemes: phonemes
                    .to_lowercase()
                    .split(';')
                    .map(|p| {
                        pos += 50;
                        if let Some(stripped) = p.strip_prefix('|') {
                            pos += 200;
                            (pos, true, stripped)
                        } else {
                            (pos, false, p)
                        }
                    })
                    .map(|(pos, word_start, p)| P::new_blank(pos, word_start, p))
                    .collect(),
            }
        }

        fn from_array(mut phonemes: Vec<(&str, u32, u32)>) -> PSeq {
            PSeq {
                hypothesis: None,
                phonemes: phonemes
                    .drain(..)
                    .map(|(p, start, end)| {
                        if let Some(stripped) = p.strip_prefix('|') {
                            (start, end, true, stripped.to_lowercase())
                        } else {
                            (start, end, false, p.to_lowercase())
                        }
                    })
                    .map(|(start, end, word_start, p)| P::new(start, end, word_start, &p))
                    .collect(),
            }
        }

        fn as_string(&self) -> String {
            let pseq: Vec<&str> = self.phonemes.iter().map(|p| p.phoneme.as_str()).collect();
            pseq.join(";")
        }
    }

    impl P {
        fn new_blank(start: u32, word_start: bool, phoneme: &str) -> P {
            P::new(start, start + 50, word_start, phoneme)
        }

        fn new(start: u32, end: u32, word_start: bool, phoneme: &str) -> P {
            P {
                phoneme: phoneme.to_owned(),
                word_start,
                start,
                end,
                weight: 0.0,
                score: 0.0,
                matching_info: None,
                traceback: None,
                active: false,
                warnings: Vec::default(),
            }
        }
    }

    fn scores_to_ops(input: Vec<Score>) -> Vec<AlignmentOperation> {
        input.iter().map(|o| o.op.unwrap()).collect()
    }

    fn to_human_friendly_sequence(result: &[P]) -> Vec<String> {
        result.iter().map(|p| {
            p.matching_info.as_deref().unwrap_or("").trim().to_string()
        })
        .collect()
    }

    fn to_human_friendly_sequence_with_scores(result: &[P]) -> Vec<String> {
        result.iter().map(|p| {
            format!("{} | {:.2}", p.matching_info.as_deref().unwrap_or("").trim(), p.score)
        })
        .collect()
    }

    fn generate_result_matching(
        matcher: &SequenceMatcher,
        a: &PSeq,
        alignment: &[Score],
        t: &PSeq,
        with_scores: bool,
    ) -> Vec<String> {
        let (result, _min_score, _has_gaps, _seq) = matcher
                .apply_alignment(&a.phonemes, alignment, &t.phonemes)
                .unwrap();

        let hf_sequence = if with_scores {
            to_human_friendly_sequence_with_scores(&result)
        } else {
            to_human_friendly_sequence(&result)
        };
        println!("result: {hf_sequence:#?}");
        hf_sequence
    }

    #[test]
    fn test_pseq_conversion() {
        let a = PSeq::from_str("a;b;c;a");
        let t = PSeq::from_str("a;a");
        assert_eq!("a;b;c;a", &a.as_string());
        assert_eq!("a;a", &t.as_string());
    }

    #[test]
    fn test_alignment_match() {
        let sim_matrix = SimilarityMatrix::init_from_str("a;b;c;d", "a;b;c;d", -1.0, 1.0);
        println!("> SimilarityMatrix: {}", &sim_matrix);
        let matcher = SequenceMatcher::new(sim_matrix);

        let a = PSeq::from_str("a;d");
        let t = PSeq::from_str("a;d");
        let alignment = matcher
            .calculate_alignment(matcher.calculate_score_matrix(&a.phonemes, &t.phonemes))
            .unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [   50 -  100] | 1.00",
            "d  ~  d  [  100 -  150] | 1.00",
        ]);
        assert_eq!(scores_to_ops(alignment), [Match, Match]);
    }

    #[test]
    fn test_alignment_inserts_splits() {
        let mut sim_matrix =
            SimilarityMatrix::init_from_str("a;b;c;d;e;x", "a;b;c;d;e;x", -1.0, 1.0);
        // prefer t=d more than a mismatch
        sim_matrix
            .set_score("d", "e", 0.5)
            .set_score("a", "e", 0.5)
            .set_score("d", "c", 0.5)
            .set_score("a", "x", 0.5);

        let matcher = SequenceMatcher::new(sim_matrix);

        let a = PSeq::from_str("a;d");
        let t = PSeq::from_str("x;a;d;e");

        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        println!(
            "> ScoreMatrix: {}",
            DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes)
        );
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        println!(
            "Seq: {}",
            matcher
                .apply_alignment(&a.phonemes, &alignment, &t.phonemes)
                .unwrap()
                .3
        );

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "x  ~  a  [   50 -  100] | 0.50",
            "a  \\  a  [   50 -  100] | 0.99",
            "d  ~  d  [  100 -  150] | 1.00",
            "e  \\  d  [  100 -  150] | 0.49",
        ]);
        assert_eq!(
            scores_to_ops(alignment),
            [Match, SplitLeft, Match, SplitLeft]
        );


        let a = PSeq::from_str("a;d");
        let t = PSeq::from_str("a;e;d");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        println!(
            "> ScoreMatrix: {}",
            DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes)
        );
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        println!(
            "Seq: {}",
            matcher
                .apply_alignment(&a.phonemes, &alignment, &t.phonemes)
                .unwrap()
                .3
        );

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [   50 -  100] | 1.00",
            "e  \\  a  [   50 -  100] | 0.49",
            "d  ~  d  [  100 -  150] | 1.00",
        ]);
        assert_eq!(scores_to_ops(alignment), [Match, SplitLeft, Match]);


        let a = PSeq::from_str("a;d");
        let t = PSeq::from_str("a;e;b;c;d");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        println!(
            "> ScoreMatrix: {}",
            DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes)
        );
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        println!(
            "Seq: {}",
            matcher
                .apply_alignment(&a.phonemes, &alignment, &t.phonemes)
                .unwrap()
                .3
        );

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [   50 -  100] | 1.00",
            "e  \\  a  [   50 -  100] | 0.49",
            "b  i | -1.00",
            "c  ~  d  [  100 -  150] | 0.50",
            "d  \\  d  [  100 -  150] | 0.99",
        ]);

        assert_eq!(
            scores_to_ops(alignment),
            [Match, SplitLeft, Insert, Match, SplitLeft]
        );
    }

    #[test]
    fn test_alignment_deletes_merges() {
        let mut sim_matrix =
            SimilarityMatrix::init_from_str("e;a;b;c;d;x;t", "d;a;b;c;e;x", -1.0, 1.0);
        // prefer t=d more than a mismatch
        sim_matrix.set_score("t", "d", 0.5).set_score("e", "a", 0.5);

        //println!("> SimilarityMatrix: {}", &sim_matrix);

        let matcher = SequenceMatcher::new(sim_matrix);

        let a = PSeq::from_str("x;a;d;t");
        let t = PSeq::from_str("a;d");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        //println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "_  d  x  [   50 -  100] | -1.00",
            "a  ~  a  [  100 -  150] | 1.00",
            "d  ~> d  [  150 -  200] + t  [  200 -  250] | 0.75",
        ]);
        assert_eq!(scores_to_ops(alignment), [Delete, Match, Match, MergeLeft]);

        // NO MergeLeft as similarity x != a
        let a = PSeq::from_str("a;x;d");
        let t = PSeq::from_str("a;d");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        //println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [   50 -  100] | 1.00",
            "_  d  x  [  100 -  150] | -1.00",
            "d  ~  d  [  150 -  200] | 1.00",
        ]);
        assert_eq!(scores_to_ops(alignment), [Match, Delete, Match]);

        // MergeLeft as similarity e ~ a
        let a = PSeq::from_str("a;e;d");
        let t = PSeq::from_str("a;d");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        //println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        assert_eq!(scores_to_ops(alignment), [Match, MergeLeft, Match]);


        // prefer Match, MergeLeft over Match, Delete
        let a = PSeq::from_str("a;e;a;c;d");
        let t = PSeq::from_str("a;d");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        //println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~> a  [   50 -  100] + e  [  100 -  150] | 0.75",
            "_  d  a  [  150 -  200] | -1.00",
            "_  d  c  [  200 -  250] | -1.00",
            "d  ~  d  [  250 -  300] | 1.00",
        ]);

        assert_eq!(
            scores_to_ops(alignment),
            [Match, MergeLeft, Delete, Delete, Match]
        );
    }

    #[test]
    fn test_alignment_with_word_gaps() {
        let mut sim_matrix =
            SimilarityMatrix::init_from_str("A;B;C;D;E;F", "a;b;c;d;e;f", -1.0, 1.0);

        sim_matrix
            .set_score("E", "c", -2.1);

        println!("> SimilarityMatrix: {}", &sim_matrix);

        let matcher = SequenceMatcher::new(sim_matrix);

        // split over words without audio gap
        let a = PSeq::from_str("A;B;C;E;F");
        let t = PSeq::from_str("a;b;c;|c;e;f");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        //println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [   50 -  100] | 1.00",
            "b  ~  b  [  100 -  150] | 1.00",
            "c  ~  c  [  150 -  200] | 1.00",
            "c  \\  c  [  150 -  200] | 0.99",
            "e  ~  e  [  200 -  250] | 1.00",
            "f  ~  f  [  250 -  300] | 1.00",
        ], "split over words without audio gap");
        assert_eq!(scores_to_ops(alignment), [Match, Match, Match, SplitLeft, Match, Match]);


        // matched gap + word start
        let a = PSeq::from_str("A;B;C;|C;E;F");
        let t = PSeq::from_str("a;b;c;|c;e;f");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        //println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [   50 -  100] | 1.00",
            "b  ~  b  [  100 -  150] | 1.00",
            "c  ~  c  [  150 -  200] | 1.00",
            "c  ~  c  [  400 -  450] | 1.00",
            "e  ~  e  [  450 -  500] | 1.00",
            "f  ~  f  [  500 -  550] | 1.00",
        ], "matched gap + word start");
        assert_eq!(scores_to_ops(alignment), [Match, Match, Match, Match, Match, Match]);
    }

    #[test]
    fn test_alignment_no_audio_gap_within_word() {
        let mut sim_matrix =
            SimilarityMatrix::init_from_str("A;B;C;D;E;F", "a;b;c;d;e;f", -1.0, 1.0);

        sim_matrix
            .set_score("E", "c", -2.1);

        println!("> SimilarityMatrix: {}", &sim_matrix);

        let matcher = SequenceMatcher::new(sim_matrix);
        // prevent audio gap within word on last phoneme
        let a = PSeq::from_str("A;B;|C;E;F");
        let t = PSeq::from_str("a;b;c;|c;e;f");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [   50 -  100] | 1.00",
            "b  ~  b  [  100 -  150] | 1.00",
            "c  i | -1.00",
            "c  ~  c  [  350 -  400] | 1.00",
            "e  ~  e  [  400 -  450] | 1.00",
            "f  ~  f  [  450 -  500] | 1.00",
        ], "prevent audio gap within word on last phoneme");
        assert_eq!(scores_to_ops(alignment), [Match, Match, Insert, Match, Match, Match]);
    }

    #[test]
    fn test_alignment_no_split_on_word_gaps() {
        let mut sim_matrix =
            SimilarityMatrix::init_from_str("A;B;C;D;E;F", "a;b;c;d;e;f", -1.0, 1.0);

        sim_matrix
            .set_score("E", "c", -2.1);

        println!("> SimilarityMatrix: {}", &sim_matrix);

        let matcher = SequenceMatcher::new(sim_matrix);

        let a = PSeq::from_str("A;B;C;|E;F");
        let t = PSeq::from_str("a;b;c;|c;e;f");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [   50 -  100] | 1.00",
            "b  ~  b  [  100 -  150] | 1.00",
            "c  ~  c  [  150 -  200] | 1.00",
            "c  i | -1.00",
            // difficult to increase confidence as audio gap precedes *and* text phoneme is inserted
            "e  ~  e  [  400 -  450] | -1.00",
            "f  ~  f  [  450 -  500] | 1.00",
        ], "no split on word gaps");
        assert_eq!(scores_to_ops(alignment), [Match, Match, Match, Insert, Match, Match]);
    }

    #[test]
    fn test_alignment_no_merge_on_word_gaps() {
        let mut sim_matrix =
            SimilarityMatrix::init_from_str("A;B;C;D;E;F", "a;b;c;d;e;f", -1.0, 1.0);

        sim_matrix
            .set_score("B", "c", 0.7);

        println!("> SimilarityMatrix: {}", &sim_matrix);

        let matcher = SequenceMatcher::new(sim_matrix);

        let a = PSeq::from_str("A;B;C;|B;E;F");
        let t = PSeq::from_str("a;b;c;|e;f");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [   50 -  100] | 1.00",
            "b  ~  b  [  100 -  150] | 1.00",
            "c  ~  c  [  150 -  200] | 1.00",
            "_  d  b  [  400 -  450] | -1.00",
            "e  ~  e  [  450 -  500] | 1.00",
            "f  ~  f  [  500 -  550] | 1.00",
        ], "no merge on word gaps");
        assert_eq!(scores_to_ops(alignment), [Match, Match, Match, Delete, Match, Match]);
    }

    #[test]
    fn test_alignment_problem() {
        let mut sim_matrix =
            SimilarityMatrix::init_from_str("A;B;C", "a;b;c", -1.0, 2.0);

        sim_matrix
            .set_score("A", "a", 1.5);

        println!("> SimilarityMatrix: {}", &sim_matrix);

        let matcher = SequenceMatcher::new(sim_matrix);

        let a = PSeq::from_array(
            vec![
                ("SIL",    0,   65),
                ("A",     70,  460),
                ("SIL",  470, 1480),
                ("SIL", 1490, 2225),
                ("B",   2230, 2325),
                ("C",   2325, 2415),
                ("B",   2415, 2575),
                ("C",   2575, 2810),
                ("SIL", 2820, 3360),
            ]
        );
        let t = PSeq::from_str("|a;|b;c;b;c");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [   70 -  460] | 1.50",
            "b  ~  b  [ 2230 - 2325] | 2.00",
            "c  ~  c  [ 2325 - 2415] | 2.00",
            "b  ~  b  [ 2415 - 2575] | 2.00",
            "c  ~  c  [ 2575 - 2810] | 2.00",
        ]);
        assert_eq!(scores_to_ops(alignment), [Delete, Match, Delete, Delete, Match, Match, Match, Match, Delete]);
    }

    #[test]
    fn test_alignment_split_on_last_missing_phoneme() {
        let mut sim_matrix =
            SimilarityMatrix::init_from_str("A;B;C;D;E", "a;b;c;d;e;x", -1.0, 2.0);

        sim_matrix
            .set_score("D", "x", 0.01);

        println!("> SimilarityMatrix: {}", &sim_matrix);

        let matcher = SequenceMatcher::new(sim_matrix);

        let a = PSeq::from_array(
            vec![
                ("A",    125,  155),
                ("B",    155,  590),
                ("SIL",  600, 1025),
                ("C",   1030, 1085),
                ("D",   1085, 1210),
                ("SIL", 1220, 1315),
                ("E",   1320, 1505),
                ("SIL", 1220, 1315),
            ]
        );
        let t = PSeq::from_str("|a;b;|c;d;x;|e");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [  125 -  155] | 2.00",
            "b  ~  b  [  155 -  590] | 2.00",
            "c  ~  c  [ 1030 - 1085] | 2.00",
            "d  ~  d  [ 1085 - 1210] | 2.00",
            "x  \\  d  [ 1085 - 1210] | 0.00",
            "e  ~  e  [ 1320 - 1505] | 2.00",
        ], "split on last missing phoneme");
        assert_eq!(scores_to_ops(alignment), [Match, Match, Delete, Match, Match, SplitLeft, Delete, Match, Delete]);
    }

    #[test]
    fn test_alignment_no_split_if_next_matches() {
        let mut sim_matrix =
            SimilarityMatrix::init_from_str("A;B;C;D;E;F", "a;b;c;d;e;f;x", -1.0, 2.0);

        sim_matrix
            .set_score("D", "x", 0.01);

        println!("> SimilarityMatrix: {}", &sim_matrix);

        let matcher = SequenceMatcher::new(sim_matrix);

        let a = PSeq::from_array(
            vec![
                ("A",    345,  445),
                ("B",    445,  525),
                ("C",    525,  685),
                ("D",    685,  860),
                ("SIL",  870, 1385),
                ("E",   1390, 1525),
                ("F",   1525, 1595),
                ("SIL", 3070, 3450),
            ]
        );
        let t = PSeq::from_str("|a;b;c;|d;x;|e;f");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [  345 -  445] | 2.00",
            "b  ~  b  [  445 -  525] | 2.00",
            "c  ~  c  [  525 -  685] | 2.00",
            "d  ~  d  [  685 -  860] | -0.50",  // difficult to increase confidence as audio gap follows
            "x  \\  d  [  685 -  860] | 0.00",
            "e  ~  e  [ 1390 - 1525] | 2.00",
            "f  ~  f  [ 1525 - 1595] | 2.00",
        ], "no split if next matches");
        assert_eq!(scores_to_ops(alignment), [Match, Match, Match, Match, SplitLeft, Delete, Match, Match, Delete]);
    }

    #[test]
    fn test_alignment_donot_ignore_last_single_phoneme_match() {
        let sim_matrix =
            SimilarityMatrix::init_from_str("A;B", "a;b;", -1.0, 2.0);

        println!("> SimilarityMatrix: {}", &sim_matrix);

        let matcher = SequenceMatcher::new(sim_matrix);

        let a = PSeq::from_array(
            vec![
                ("SIL",  964,  978),
                ("A",    979,  998),
                ("SIL",  999, 1005),
                ("A",   1006, 1029),
                ("SIL", 1030, 1040),
            ]
        );
        let t = PSeq::from_str("|a;|a");
        let matrix = matcher.calculate_score_matrix(&a.phonemes, &t.phonemes);
        println!("> ScoreMatrix: {}", DebugMatrix2D::new(&matrix, &a.phonemes, &t.phonemes));
        let alignment = matcher.calculate_alignment(matrix).unwrap();

        let result = generate_result_matching(&matcher, &a, &alignment, &t, true);

        assert_eq!(result, vec![
            "a  ~  a  [  979 -  998] | 2.00",
            "a  ~  a  [ 1006 - 1029] | 2.00",
        ], "do not ignore last single phoneme match");
        assert_eq!(scores_to_ops(alignment), [Delete, Match, Delete, Match, Delete]);
    }
}
