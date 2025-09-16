//
// phonemes datastructures and file encoding/decoding
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub trait PhonemeSegmentInterface {
    fn phoneme(&self) -> &str;
    fn is_active(&self) -> bool;
    fn is_word_start(&self) -> bool;
    fn start(&self) -> u32;
    fn end(&self) -> u32;

    fn set_active(&mut self, state: bool);
    fn set_start(&mut self, value: u32);
    fn set_end(&mut self, value: u32);
}
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
pub enum QualityWarning {
    PhonemeGapInWord,
    UnusualDuration(u32),
    InactiveSegmentsInWord(usize),
    HighAmountOfLowScoreSegments(u32, f32),
}
// ----------------------------------------------------------------------------
const LOW_SCORE_THRESHOLD: f32 = 0.15;
const MAX_LOW_SCORE_PERCENTAGE: u32 = 20;
// ----------------------------------------------------------------------------
impl QualityWarning {
    // ------------------------------------------------------------------------
    pub fn short(&self) -> String {
        use self::QualityWarning::*;

        match self {
            PhonemeGapInWord => "phoneme gap within a word?".to_string(),
            UnusualDuration(d) => format!("suspicious duration: {d}ms"),
            InactiveSegmentsInWord(_n) => "inactive phoneme within a word".to_string(),
            HighAmountOfLowScoreSegments(_n, f) => format!("low confidence-score: {f:.2}"),
        }
    }
    // ------------------------------------------------------------------------
    pub fn long(&self) -> String {
        use self::QualityWarning::*;

        match self {
            PhonemeGapInWord => "detected possible phoneme gap within a word".to_string(),
            UnusualDuration(d) => {
                format!("unusually short/long segment duration {d} ms for a phoneme within a word")
            }
            InactiveSegmentsInWord(n) => {
                format!("inactive segments within a word (found {n})")
            }
            HighAmountOfLowScoreSegments(n, _f) => {
                format!(
                    "> {MAX_LOW_SCORE_PERCENTAGE}% of active segments have low confidence-score \
                    (found {n}%, see phoneme data table)",
                )
            }
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[derive(Clone, Default, Debug)]
pub struct PhonemeSegment {
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
// ----------------------------------------------------------------------------
pub struct PhonemeResult {
    pub hypothesis: Option<String>,
    pub phonemes: Vec<PhonemeSegment>,
}
// ----------------------------------------------------------------------------
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum QualityAssessment {
    #[default]
    Unknown,
    Ok,
    NeedsCheckWarn,
    NeedsCheckError,
    EditedOk,
    EditedWithErrors,
}
// ----------------------------------------------------------------------------
#[derive(Default, Clone, Eq, PartialEq)]
pub struct PhonemeTrack<T> {
    id: u32,
    version: u16,
    language: String,
    input_text: String,
    translation: String,
    audio_hypothesis: Option<String>,
    actor: Option<String>,
    phonemes: Vec<T>,
    quality: QualityAssessment,
}
// ----------------------------------------------------------------------------
pub fn store(outputpath: &PathBuf, data: PhonemeTrack<PhonemeSegment>) -> Result<String, String> {
    let mut path = PathBuf::from(outputpath);
    path.push(format!("{:010}.phonemes", data.id));

    save_as_csv(&path, &data)?;

    Ok(path.to_string_lossy().into())
}
// ----------------------------------------------------------------------------
pub fn load<P: AsRef<Path>>(id: u32, path: P) -> Result<PhonemeTrack<PhonemeSegment>, String> {
    // overwrite id as it is not stored in the meta information
    PhonemeTrack::load(path.as_ref()).map(|mut track| {
        track.id = id;
        track.assess_quality();
        debug!(
            "read {} phoneme segments (assessed quality: {})",
            track.phonemes.len(),
            track.assessed_quality(),
        );
        track
    })
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::io::BufRead;
use std::path::{Path, PathBuf};
use text::{CsvLoader, CsvWriter, SimpleCsvWriter};

use sequence_matcher::WARN_MATCHING_SCORE_MIN;
// ----------------------------------------------------------------------------
impl<T> PhonemeTrack<T> {
    // ------------------------------------------------------------------------
    pub fn new(
        id: u32,
        language: &str,
        text: &str,
        translation: &str,
        audio_hypothesis: Option<String>,
        actor: Option<String>,
        phonemes: Vec<T>,
    ) -> PhonemeTrack<T> {
        PhonemeTrack {
            id,
            version: 1,
            language: language.to_owned(),
            input_text: text.to_owned(),
            translation: translation.to_owned(),
            audio_hypothesis,
            actor,
            phonemes,
            quality: QualityAssessment::Unknown,
        }
    }
    // ------------------------------------------------------------------------
    pub fn id(&self) -> u32 {
        self.id
    }
    // ------------------------------------------------------------------------
    pub fn version(&self) -> u16 {
        self.version
    }
    // ------------------------------------------------------------------------
    pub fn language(&self) -> &str {
        &self.language
    }
    // ------------------------------------------------------------------------
    pub fn input_text(&self) -> &str {
        &self.input_text
    }
    // ------------------------------------------------------------------------
    pub fn translation(&self) -> &str {
        &self.translation
    }
    // ------------------------------------------------------------------------
    pub fn audio_hypothesis(&self) -> &Option<String> {
        &self.audio_hypothesis
    }
    // ------------------------------------------------------------------------
    pub fn actor(&self) -> Option<&String> {
        self.actor.as_ref()
    }
    // ------------------------------------------------------------------------
    pub fn phonemes(&self) -> &Vec<T> {
        &self.phonemes
    }
    // ------------------------------------------------------------------------
    pub fn phonemes_mut(&mut self) -> &mut Vec<T> {
        &mut self.phonemes
    }
    // ------------------------------------------------------------------------
    pub fn set_version(&mut self, version: u16) {
        self.version = version
    }
    // ------------------------------------------------------------------------
    pub fn set_actor(&mut self, actor: &str) {
        self.actor = Some(actor.to_string())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl PhonemeTrack<PhonemeSegment> {
    // ------------------------------------------------------------------------
    pub fn assessed_quality(&self) -> QualityAssessment {
        self.quality
    }
    // ------------------------------------------------------------------------
    pub fn assess_quality(&mut self) -> QualityAssessment {
        use self::QualityAssessment::*;
        use self::QualityWarning::*;

        // check multiple indicators:
        // - any gaps within a word -> serious problem
        // - any translation word inactive -> serious problem
        // - unusually long phoneme timing -> probably a problem
        // - very low scores -> probably a problem

        let mut words = Vec::new();
        let mut word = Vec::new();
        for segment in &mut self.phonemes {
            if segment.word_start && !word.is_empty() {
                words.push(word);
                word = Vec::new();
            }
            word.push(segment);
        }
        if !word.is_empty() {
            words.push(word);
        }

        let mut new_assessment = if self.version > 1 {
            QualityAssessment::EditedOk
        } else {
            QualityAssessment::Ok
        };
        let mut low_scores = 0;
        let mut low_scores_segments = Vec::default();
        let mut active_segments = 0;
        let lineid = self.id;
        for word in &mut words {
            let mut pos = 0;
            let mut slot = 0;
            let mut inactive_segments = Vec::default();
            for segment in word {
                segment.warnings.clear();

                if segment.phoneme != "_" {
                    slot += 1;
                }

                if segment.word_start {
                    pos = segment.start;
                }

                let duration = segment.end.saturating_sub(segment.start);

                if segment.active {
                    if !(15..=500).contains(&duration) {
                        // very short or very long (> 0.5s) segment
                        debug!(
                            "id {lineid:010}: > detected unusually short or long segment within \
                            a word [{}: {} length {}]. suggested checking...",
                            segment.start, segment.phoneme, duration,
                        );
                        segment.warnings.push(UnusualDuration(duration));
                        new_assessment.update(NeedsCheckWarn);
                    }

                    active_segments += 1;

                    if segment.start.saturating_sub(pos) > 0 {
                        // a gap within a word is almost always an error
                        debug!(
                            "id {lineid:010}: > detected phoneme gap within a word [{}: {}]. \
                                requires checking...",
                            segment.start, segment.phoneme,
                        );
                        segment.warnings.push(PhonemeGapInWord);
                        new_assessment.update(NeedsCheckError);
                    }
                    pos = segment.end;

                    if segment.score < LOW_SCORE_THRESHOLD {
                        low_scores += 1;
                        low_scores_segments.push(segment);
                    }
                } else if segment.phoneme != "_" {
                    inactive_segments.push((segment, slot));
                }
            }
            if !inactive_segments.is_empty() {
                // a translation segment that should be active but is not
                // this should also cover complete inactive words
                let gaps = inactive_segments.len();
                debug!(
                    "id {lineid:010}: > detected #{gaps} inactive phoneme segments \
                    in phoneme track. requires checking...",
                );
                let mut inactive_within_word = false;
                for (segment, segment_slot) in inactive_segments.drain(..) {
                    segment.warnings.push(InactiveSegmentsInWord(gaps));
                    inactive_within_word |= segment_slot > 1 && segment_slot < slot;
                }

                // warning if the inactive slots are word boundaries. but error
                // if it is within a word
                if inactive_within_word {
                    new_assessment.update(NeedsCheckError);
                } else {
                    new_assessment.update(NeedsCheckWarn);
                }
            }
        }
        let low_scores_percentage = (low_scores as f32 / active_segments as f32) * 100.0;
        if low_scores_percentage.round() as u32 > MAX_LOW_SCORE_PERCENTAGE {
            debug!(
                "id {lineid:010}: > detected {low_scores_percentage:.2}% low-score segment \
                mappings. suggested checking...",
            );
            new_assessment.update(NeedsCheckWarn);

            for segment in low_scores_segments.drain(..) {
                segment.warnings.push(HighAmountOfLowScoreSegments(
                    low_scores_percentage.round() as u32,
                    segment.score,
                ));
            }
        }

        self.quality = new_assessment;
        self.quality
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl QualityAssessment {
    // ------------------------------------------------------------------------
    fn update(&mut self, new_assessment: Self) {
        use self::QualityAssessment::*;

        *self = match (&self, new_assessment) {
            (Unknown, _)
            | (Ok, NeedsCheckWarn)
            | (Ok, NeedsCheckError)
            | (Ok, EditedOk)
            | (NeedsCheckWarn, NeedsCheckError)
            | (NeedsCheckWarn, EditedOk)
            | (NeedsCheckError, EditedOk) => new_assessment,

            (EditedOk, NeedsCheckWarn) | (EditedOk, NeedsCheckError) => EditedWithErrors,
            (_, _) => *self,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<T> PhonemeTrack<T> {
    // ------------------------------------------------------------------------
    fn is_headerline(line: &str) -> bool {
        let cols = line
            .split('|')
            .map(|c| c.trim().to_lowercase())
            .collect::<Vec<_>>();

        cols.len() > 3
            && cols[0] == ";phoneme"
            && cols[1] == "start"
            && cols[2] == "end"
            && cols[3] == "weight"
        //&& cols[4] == "score"
    }
    // ------------------------------------------------------------------------
    fn parse_segment(input_line: &str, new_word: bool) -> Result<PhonemeSegment, String> {
        let (active, line) = if let Some(input_line) = input_line.strip_prefix(';') {
            (false, input_line)
        } else {
            (true, input_line)
        };
        let data: Vec<_> = line.split('|').map(str::trim).collect();

        if data.len() > 3 {
            let mut segment = PhonemeSegment {
                phoneme: data[0].to_string(),
                word_start: new_word,
                start: data[1]
                    .parse::<u32>()
                    .map_err(|e| format!("col #1: {}", e))?,
                end: data[2]
                    .parse::<u32>()
                    .map_err(|e| format!("col #2: {}", e))?,
                weight: data[3]
                    .parse::<f32>()
                    .map_err(|e| format!("col #3: {}", e))?,
                score: 0.0,

                matching_info: None,
                traceback: Some(input_line.to_owned()),
                active,
                warnings: Vec::default(),
            };

            if data.len() > 4 {
                segment.score = data[4]
                    .parse::<f32>()
                    .map_err(|e| format!("col #4: {}", e))?;
            }
            // ignore col 5 with status
            if data.len() > 6 {
                segment.matching_info = Some(data[6].to_owned());
            }
            Ok(segment)
        } else {
            Err(format!(
                "data line must contain at least 4 columns (phoneme, start, end, \
                 weight, [score]). found: {}",
                data.len()
            ))
        }
    }
    // ------------------------------------------------------------------------
    fn legacy_parse(line: &str) -> Result<String, String> {
        match line.find(':') {
            Some(pos) => Ok(line[pos + 1..].trim_matches('"').to_owned()),
            None => Err(String::from(
                "could not parse legacy format (missing separator).",
            )),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
use std::str::FromStr;
// ----------------------------------------------------------------------------
impl CsvLoader<PhonemeTrack<PhonemeSegment>> for PhonemeTrack<PhonemeSegment> {
    // ------------------------------------------------------------------------
    fn load(filepath: &Path) -> Result<PhonemeTrack<PhonemeSegment>, String> {
        let reader = Self::create_reader(filepath)?;

        let mut track = PhonemeTrack::default();
        let mut header_found = false;
        let mut new_word_starting = false;

        for (pos, line) in reader.lines().enumerate() {
            let err_format = |e: &str| -> String {
                format!("phonemes loader: error reading line {}: {}", pos + 1, e)
            };

            let line = line.map_err(|e| err_format(&e.to_string()))?;

            if !header_found {
                match line.as_str() {
                    l if l.starts_with(";meta") => {
                        match Self::parse_meta(l).map_err(|e| err_format(&e))? {
                            ("language", value) => track.language = value.to_string(),
                            ("text", value) => track.input_text = value.to_string(),
                            ("translation", value) => track.translation = value.to_string(),
                            ("audio-hypothesis", value) => {
                                track.audio_hypothesis = Some(value.to_owned())
                            }
                            ("version", value) => {
                                track.version =
                                    u16::from_str(value).map_err(|e| err_format(&e.to_string()))?
                            }
                            ("actor", value) => {
                                track.actor = Some(value.trim().to_lowercase());
                            }
                            (key, _) => {
                                return Err(err_format(&format!("found unsupported meta key [{key}]")))
                            }
                        }
                    }

                    l if Self::is_headerline(l) => header_found = true,

                    // --- legacy format data extraction (without meta)
                    l if l.starts_with(";provided source text") => {
                        track.input_text = Self::legacy_parse(l)?
                    }
                    l if l.starts_with(";phoneme translation") => {
                        track.translation = Self::legacy_parse(l)?
                    }
                    l if l.starts_with(";audio hypothesis") => {
                        track.audio_hypothesis = Some(Self::legacy_parse(l)?)
                    }
                    // --- legacy format data end

                    l if l.starts_with(';') => continue,
                    _ => return Err(err_format(
                        "expected header line with column definition \
                         (phoneme, start, end, weight, [score]) before start of data block.",
                    )),
                }
            } else {
                match line.as_str() {
                    l if l.starts_with("---") => {
                        new_word_starting = true;
                        continue;
                    }
                    l => track
                        .phonemes
                        .push(Self::parse_segment(l, new_word_starting)?),
                }
                new_word_starting = false;
            }
        }

        Ok(track)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
fn save_as_csv(filepath: &PathBuf, data: &PhonemeTrack<PhonemeSegment>) -> Result<(), String> {
    let line_length;
    let mut writer = SimpleCsvWriter::create(filepath)?;

    trace!("> writing csv header...");
    writer.write_meta("language", &data.language);
    writer.write_meta("version", &format!("{}", data.version));
    if let Some(actor) = data.actor.as_deref() {
        writer.write_meta("actor", actor);
    }
    writer.write_meta("text", &data.input_text);
    writer.write_meta("translation", &data.translation);

    // available audio hypothesis means audio + translation data available
    if let Some(ref audio_hypothesis) = data.audio_hypothesis {
        writer.write_meta("audio-hypothesis", audio_hypothesis);
        writer.write_comment("");
        writer.write_comment(
            "auto-matched phoneme translation (eSpeak) with timings (pocketsphinx):",
        );
        writer.write_comment("");
        writer.write_header(
            "phoneme|start|  end|weight| score| status     | match + pocketsphinx timing",
        );
        line_length = 72;
    } else {
        writer.write_comment("");
        writer.write_header("phoneme|start|  end|weight| score| status");
        line_length = 48;
    }

    let empty_str = &String::from("");
    let word_seperator = &format!("{:-<1$}", "-", line_length);

    // write timings
    debug!("storing #{} phoneme timings", data.phonemes.len());
    for segment in &data.phonemes {
        let status = if segment.score < WARN_MATCHING_SCORE_MIN {
            "<- VERIFY!"
        } else {
            "ok"
        };
        let active = if segment.active { "" } else { ";" };
        if segment.word_start {
            writer.writeln(word_seperator);
        }
        let line = format!(
            "{}{:<8}|{:>5}|{:>5}|{:>6.2}|{:>6.2}| {:<11}| {}",
            active,
            segment.phoneme,
            segment.start,
            segment.end,
            segment.weight,
            segment.score,
            status,
            segment
                .matching_info
                .as_ref()
                .map(|s| s.trim())
                .unwrap_or(empty_str)
        );

        writer.writeln(&line);
    }
    Ok(())
}
// ----------------------------------------------------------------------------
#[rustfmt::skip]
impl PhonemeSegmentInterface for PhonemeSegment {
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
// commands
// ----------------------------------------------------------------------------
pub fn auto_close_gaps<P: PhonemeSegmentInterface>(
    duration: u32,
    track: &mut PhonemeTrack<P>,
) -> usize {
    let mut flipped = Vec::new();
    let segments = track.phonemes_mut();

    for (i, segment) in segments.iter_mut().enumerate() {
        if !segment.is_active() && segment.phoneme() == "_" {
            segment.set_active(true);
            flipped.push(i);
        }
    }
    // workaround for borrow checker
    for slot in &flipped {
        segments
            .iter_mut()
            .skip(*slot)
            .take(1)
            .for_each(|segment| segment.set_active(false));
        update_timings_on_activation(duration, segments, *slot, false);
    }

    flipped.len()
}
// ----------------------------------------------------------------------------
/// phoneme gaps closing on de/activation of phoneme segment
pub fn update_timings_on_activation<P: PhonemeSegmentInterface>(
    max_position: u32,
    phonemes: &mut [P],
    slot: usize,
    activated: bool,
) {
    // find active neighboring phoneme segments that do not cross word boundary
    let (pred, succ) = phonemes.split_at_mut(slot);
    let (segment, succ) = succ.split_at_mut(1);

    let segment = &mut segment[0];
    segment.set_active(activated);

    let mut predecessor = pred
        .iter()
        .enumerate()
        .rev()
        .find(|&(_, segment)| segment.is_active() || segment.is_word_start())
        .map(|(i, segment)| (i, segment.is_active()));

    let mut successor = None;
    for (i, current) in succ.iter().enumerate() {
        if current.is_word_start() {
            break;
        }
        successor = Some((i, current.is_active()));

        if current.is_active() {
            break;
        }
    }

    if segment.is_word_start() {
        predecessor = None;
    }

    if activated {
        let duration = segment.end().saturating_sub(segment.start());
        let segment_mid = segment.start() as f32 + duration as f32 * 0.5;
        // default duration := 50
        let duration = u32::max(50, duration) as f32;

        // clip at 0 and audio clip length
        let mut start = clamp_ms(segment_mid - duration * 0.5, 5.0, 0, segment.end());
        let mut end = clamp_ms(segment_mid + duration * 0.5, 5.0, start, max_position);

        if let Some((slot, _)) = predecessor {
            pred.iter_mut().skip(slot).for_each(|p| {
                if p.is_active() {
                    p.set_end(clamp_ms(
                        p.start() as f32 + (p.end() as f32 - p.start() as f32) * 0.75,
                        5.0,
                        p.start(),
                        p.end(),
                    ));
                } else {
                    p.set_start(start);
                    p.set_end(start);
                }
                start = p.end();
            });
        }
        if let Some((slot, _)) = successor {
            succ.iter_mut()
                .enumerate()
                .rev()
                .skip_while(|&(i, _)| i > slot)
                .for_each(|(_, s)| {
                    if s.is_active() {
                        s.set_start(clamp_ms(
                            s.start() as f32 + (s.end() as f32 - s.start() as f32) / 3.0,
                            // hardcoded 5 ms granularity
                            5.0,
                            s.start(),
                            s.end(),
                        ));
                    } else {
                        s.set_start(end);
                        s.set_end(end);
                    }
                    end = s.start();
                });
        }
        segment.set_start(start);
        segment.set_end(end);
    } else {
        let active_predecessor = predecessor.map_or(false, |(_, active)| active);
        let active_successor = successor.map_or(false, |(_, active)| active);

        let segment_mid = segment.start() + segment.end().saturating_sub(segment.start()) / 2;

        #[allow(clippy::match_bool)]
        let new_pos = match active_predecessor {
            true if active_successor => segment_mid,
            true if !active_successor => segment.end(),
            false if active_successor => segment.start(),
            false if !active_successor => segment_mid,
            _ => unreachable!(),
        };

        if let Some((slot, _)) = predecessor {
            pred.iter_mut().skip(slot).for_each(|p| {
                if !p.is_active() {
                    p.set_start(new_pos);
                }
                p.set_end(new_pos);
            });
        }

        segment.set_start(new_pos);
        segment.set_end(new_pos);

        if let Some((slot, _)) = successor {
            succ.iter_mut().take(slot + 1).for_each(|s| {
                if !s.is_active() {
                    s.set_end(new_pos);
                }
                s.set_start(new_pos);
            });
        }
    }
}
// ----------------------------------------------------------------------------
// helper
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
