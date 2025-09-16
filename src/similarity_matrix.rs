//
// similarity matrix loader
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub struct SimilarityMatrix {
    scores: Matrix2D<f32>,
    gap_penalty: f32,
    audio_idx: HashMap<String, usize>,
    text_idx: HashMap<String, usize>,
    audio_names: Vec<String>,
    text_names: Vec<String>,
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;

use super::matrix::{DebugMatrix2D, Matrix2D};
use super::CsvLoader;
// ----------------------------------------------------------------------------
const SIMILARITY_SCORE_MAX: f32 = 2.1;
const SIMILARITY_SCORE_MIN: f32 = -2.1;
const SIMILARITY_SCORE_DEFAULT: f32 = -1.00;
const SIMILARITY_SCORE_GAP_PENALTY: f32 = SIMILARITY_SCORE_DEFAULT;
const SIMILARITY_ALTERNATIVES_MIN_SCORE: f32 = 0.1;
// ----------------------------------------------------------------------------
impl SimilarityMatrix {
    // ------------------------------------------------------------------------
    pub fn sorted_scores(&self) -> Vec<(String, Vec<String>)> {
        let mut mapping = Vec::with_capacity(self.scores.height);

        for row in 0..self.scores.height {
            let mut scores = self.scores
                .row(row)
                .copied()
                .enumerate()
                .filter(|(_i, v)| *v >= SIMILARITY_ALTERNATIVES_MIN_SCORE)
                .collect::<Vec<_>>();

            scores.sort_by(|(_, a), (_, b)| b.total_cmp(a));

            let text_phoneme = self.text_names[row].to_string();
            let audio_phonemes = scores
                .iter()
                .map(|(idx, _score)| self.audio_names[*idx].to_string())
                .collect::<Vec<_>>();

            mapping.push((text_phoneme, audio_phonemes));
        }

        mapping
    }
    // ------------------------------------------------------------------------
    pub fn get_score(&self, audio: &str, text: &str) -> f32 {
        let audio = &audio.to_lowercase();
        let text = &text.to_lowercase();

        // special cases:
        // - SIL never matches output from translator but it is important to
        //   enforce silence and prevent neighbor phoneme "bleeding" (aka merges)
        // - +NSN+ and +SPN+ are "noise phonemes" that won't match anything
        //   from translator, too
        match audio.as_str() {
            "sil" | "+nsn+" | "+spn+" => SIMILARITY_SCORE_MIN,

            _ => match self.audio_idx.get(audio) {
                Some(a_idx) => match self.text_idx.get(text) {
                    Some(t_idx) => self.scores[(*a_idx, *t_idx)],
                    None => {
                        warn!(
                            "missing similarity scores for text phoneme [{}]! \
                             using default gap score!",
                            text
                        );
                        self.gap_penalty
                    }
                },
                None => {
                    warn!(
                        "missing similarity scores for audio phoneme [{}]! \
                         using default gap score!",
                        audio
                    );
                    self.gap_penalty
                }
            },
        }
    }
    // ------------------------------------------------------------------------
    pub fn get_delete_score(&self, audio: &str) -> f32 {
        // special case: SIL does not match anything and must be removed
        // -> favor it
        if audio.to_lowercase() == "sil" {
            SIMILARITY_SCORE_MAX + 0.01
        } else {
            self.gap_penalty
        }
    }
    // ------------------------------------------------------------------------
    pub fn get_merge_left_score(&self, audio: &str, text: &str) -> f32 {
        // reduce similarity so a match is always preferred
        self.get_score(audio, text) - 0.01
    }
    // ------------------------------------------------------------------------
    pub fn get_merge_right_score(&self, audio: &str, text: &str) -> f32 {
        // reduce similarity so a match is always preferred
        self.get_score(audio, text) - 0.01
    }
    // ------------------------------------------------------------------------
    pub fn get_insert_score(&self, _text: &str) -> f32 {
        //TODO some extra modifying factor?
        self.gap_penalty
    }
    // ------------------------------------------------------------------------
    pub fn get_gap_within_word_score(&self, audio: &str, text: &str) -> f32 {
        self.gap_penalty * 1.5 + self.get_score(audio, text) * 0.5
        // self.word_gap_penalty
    }
    // ------------------------------------------------------------------------
    pub fn get_merge_left_score_over_gap(&self, audio: &str, text: &str) -> f32 {
        // drastically reduce similarity if it would span a gap
        self.gap_penalty * 1.5 + self.get_score(audio, text) * 0.5
    }
    // ------------------------------------------------------------------------
    pub fn get_split_left_gap_within_word_score(&self, audio: &str, text: &str) -> f32 {
        self.gap_penalty * 1.5 + self.get_score(audio, text) * 0.5
        // self.word_gap_penalty
    }
    // ------------------------------------------------------------------------
    pub fn get_split_left_score(&self, audio: &str, text: &str) -> f32 {
        // reduce similarity so a match is always preferred
        // prefer left split than match
        self.get_score(audio, text) - 0.01
    }
    // ------------------------------------------------------------------------
    pub fn get_split_right_score(&self, audio: &str, text: &str) -> f32 {
        // reduce similarity so a match is always preferred
        // prefer match then right split
        self.get_score(audio, text) - 0.01
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl SimilarityMatrix {
    // ------------------------------------------------------------------------
    fn extract_audiophonemes_line(textline: &str) -> Result<Vec<String>, String> {
        if textline.starts_with("[T\\A]") {
            let cols: Vec<&str> = textline.split('|').collect();

            if cols.len() < 2 {
                return Err(format!(
                    "at least one audio phoneme name required. \
                     found: {}",
                    cols.len() - 1
                ));
            }
            Ok(cols
                .iter()
                .skip(1)
                .map(|name| name.trim().to_string())
                .collect::<Vec<_>>())
        } else {
            Err(
                "first non comment line must start with [T\\A] and define audio \
                 phonemes"
                    .to_owned(),
            )
        }
    }
    // ------------------------------------------------------------------------
    fn extract_scores_line(textline: &str) -> Result<(String, Vec<f32>, usize), String> {
        if textline.starts_with("[T\\A]") {
            Err("only one audio phoneme names column definition line \
                 allowed. found another one."
                .to_owned())
        } else {
            let cols: Vec<&str> = textline.split('|').collect();

            if cols.len() < 2 {
                return Err(format!(
                    "at least similarity score required. \
                     found: {}",
                    cols.len() - 1
                ));
            }

            let text_phoneme = cols[0].trim().to_lowercase();
            let mut counter = 0;
            let mut col = 1;
            let scores: Vec<_> = cols
                .iter()
                .skip(1)
                .map(|element| {
                    col += 1;
                    if element.trim().is_empty() {
                        Ok(SIMILARITY_SCORE_DEFAULT)
                    } else {
                        match element.trim().parse() {
                            Ok(v) if (SIMILARITY_SCORE_MIN..=SIMILARITY_SCORE_MAX).contains(&v) => {
                                counter += 1;
                                Ok(v)
                            }
                            Ok(v) => Err(format!(
                                "col #{}: score out of range [{};{}]: {}",
                                col, SIMILARITY_SCORE_MIN, SIMILARITY_SCORE_MAX, v
                            )),
                            Err(why) => Err(format!("col #{}: {}", col, &why)),
                        }
                    }
                })
                .collect();

            // look for the first error
            let result: Result<Vec<_>, String> = scores.iter().cloned().collect();

            match result {
                Ok(scores) => Ok((text_phoneme, scores, counter)),
                Err(e) => Err(e),
            }
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl CsvLoader<SimilarityMatrix> for SimilarityMatrix {
    // ------------------------------------------------------------------------
    fn load(filepath: &Path) -> Result<SimilarityMatrix, String> {
        let reader = Self::create_reader(filepath)?;

        let mut parse_headerline = true;
        let mut audio_lut = HashMap::new();
        let mut audio_names = Vec::default();
        let mut text_lut = HashMap::new();
        let mut text_names = Vec::default();

        let mut score_matrix = Matrix2D {
            width: 0,
            height: 0,
            data: Vec::<f32>::with_capacity(100 * 100),
        };
        // counter for specified scores (without defaults)
        let mut score_count = 0;

        for (line, text) in reader.lines().enumerate() {
            let (text_phoneme, mut scores, count) = match text {
                // comment
                Ok(ref text) if text.starts_with(';') => continue,

                // audio phoneme names
                Ok(ref text) if parse_headerline => match Self::extract_audiophonemes_line(text) {
                    Ok(names) => {
                        score_matrix.width = names.len();
                        audio_lut = names
                            .iter()
                            .enumerate()
                            .map(|(i, p)| (p.to_lowercase(), i))
                            .collect();
                        audio_names = names;
                        parse_headerline = false;
                        continue;
                    }
                    Err(why) => return Err(format!("error reading line {}: {}", line + 1, &why)),
                },

                // text phoneme + all scores
                Ok(ref text) => match Self::extract_scores_line(text) {
                    Ok(result) => result,
                    Err(why) => return Err(format!("error reading line {}: {}", line + 1, &why)),
                },

                Err(why) => return Err(format!("error reading line {}: {}", line + 1, &why)),
            };
            // add phoneme id row number to lut and all scores to next row of matrix
            if text_lut.insert(text_phoneme.clone(), score_matrix.height).is_some() {
                return Err(format!(
                    "error reading line {}: found duplicate phoneme definition",
                    line + 1
                ));
            }
            text_names.push(text_phoneme);
            if let Err(why) = score_matrix.add_row(&mut scores) {
                return Err(format!("error reading line {}: {}", line + 1, &why));
            }
            score_count += count;
        }
        info!(
            "loaded {} similarity scores for {} audio ~ {} text phonemes",
            score_count,
            audio_lut.len(),
            text_lut.len()
        );

        let m = SimilarityMatrix {
            scores: score_matrix,
            gap_penalty: SIMILARITY_SCORE_GAP_PENALTY,
            audio_idx: audio_lut,
            text_idx: text_lut,
            audio_names,
            text_names,
        };
        trace!(">> SimilarityMatrix: {}", &m);

        Ok(m)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
use std::fmt;

impl fmt::Display for SimilarityMatrix {
    // ------------------------------------------------------------------------
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // get phoneme names sorted by idx:
        // transform to vec of (&String, &usize)
        let mut a: Vec<_> = self.audio_idx.iter().collect();
        let mut t: Vec<_> = self.text_idx.iter().collect();

        // sort by &usize
        a.sort_by_key(|a| a.1);
        t.sort_by_key(|t| t.1);

        // get only the &String
        let a: Vec<_> = a.iter().map(|kv| kv.0).collect();
        let t: Vec<_> = t.iter().map(|kv| kv.0).collect();

        write!(f, "{: ^3}", DebugMatrix2D::new(&self.scores, &a, &t))
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
#[cfg(test)]
impl SimilarityMatrix {
    pub fn init_from_str(
        audio_phonemes: &str,
        text_phonemes: &str,
        default_score: f32,
        identity_score: f32,
    ) -> SimilarityMatrix {
        let mut audio = HashMap::with_capacity(audio_phonemes.len());
        let mut text = HashMap::with_capacity(text_phonemes.len());

        let mut audio_names = Vec::with_capacity(audio_phonemes.len());
        let mut text_names = Vec::with_capacity(text_phonemes.len());

        for (i, a) in audio_phonemes.split(';').enumerate() {
            audio.insert(a.to_lowercase(), i);
            audio_names.push(a.to_lowercase());
        }

        for (i, t) in text_phonemes.split(';').enumerate() {
            text.insert(t.to_lowercase(), i);
            text_names.push(t.to_lowercase());
        }

        let mut m = Matrix2D::new_with_default(audio.len(), text.len(), default_score);

        for (phoneme, idx1) in &audio {
            match text.get(phoneme) {
                Some(idx2) => m[(*idx1, *idx2)] = identity_score,
                None => continue,
            }
        }

        SimilarityMatrix {
            scores: m,
            gap_penalty: default_score,
            audio_idx: audio,
            text_idx: text,
            audio_names,
            text_names,
        }
    }

    pub fn set_score(&mut self, audio: &str, text: &str, score: f32) -> &mut SimilarityMatrix {
        let a = *self.audio_idx.get(&audio.to_lowercase()).unwrap();
        let t = *self.text_idx.get(&text.to_lowercase()).unwrap();
        self.scores[(a, t)] = score;
        self
    }
}
// ----------------------------------------------------------------------------
