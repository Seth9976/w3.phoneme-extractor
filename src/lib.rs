extern crate glium;
extern crate indexmap;
extern crate lazy_static;
extern crate regex;
#[macro_use]
extern crate imgui;
extern crate imgui_glium_renderer;
#[macro_use]
extern crate imgui_support;
extern crate imgui_controls;
extern crate imgui_widgets;
#[macro_use]
extern crate log;
#[macro_use]
extern crate logger;

mod audio;
mod queue;
mod utils;

mod actors;
mod espeak;
mod file_scanner;
mod matrix;
mod phonemes;
mod pocketsphinx;
mod sequence_matcher;
mod similarity_matrix;
mod text;

pub mod gui;
// ----------------------------------------------------------------------------
pub use espeak::{ESpeak as TextPhonemeConverter, TextPhonemeTranslator};
pub use pocketsphinx::PocketSphinx as AudioPhonemeExtractor;

pub use sequence_matcher::SequenceMatcher as PhonemeSequenceMatcher;
pub use similarity_matrix::SimilarityMatrix as PhonemeSimilarityMatrix;
pub use text::{
    CsvLoader, CsvStringsData, CsvStringsLoader, CsvWriter, SimpleCsvWriter, StringsProvider,
};

pub use actors::ActorMapping;
pub use phonemes::store as store_phonemes;

pub use phonemes::{PhonemeResult, PhonemeTrack};
pub use queue::ProcessingQueue;
pub struct Processor<S>
where
    S: StringsProvider,
{
    strings: S,
    translator: TextPhonemeConverter,
    extractor: AudioPhonemeExtractor,
    matcher: PhonemeSequenceMatcher,
}
// ----------------------------------------------------------------------------
pub struct Generator {
    language: String,
    translator: TextPhonemeConverter,
    outputdir: PathBuf,
}
// ----------------------------------------------------------------------------
use std::collections::{BTreeMap, HashMap};

use lazy_static::lazy_static;

use queue::TaskData;
use queue::TaskResult;
// ----------------------------------------------------------------------------
impl<S> Processor<S>
where
    S: StringsProvider,
{
    // ------------------------------------------------------------------------
    pub fn new(
        strings: S,
        translator: TextPhonemeConverter,
        extractor: AudioPhonemeExtractor,
        matcher: PhonemeSequenceMatcher,
    ) -> Processor<S> {
        Processor {
            strings,
            translator,
            extractor,
            matcher,
        }
    }
    // ------------------------------------------------------------------------
    pub fn strings(&self) -> &dyn StringsProvider {
        &self.strings
    }
    // ------------------------------------------------------------------------
    fn extract_phonemes(&mut self, task: &TaskData) -> Result<String, String> {
        let original_text = self.strings.get_line(task.lineid())?;
        let actor = self.strings.get_actor(task.lineid());

        // remove all non-spoken textual hints framed by *
        let text = &*REGEXP_CLEANUP.replace_all(original_text, "");

        let text_phonemetrail = self.translator.translate(text)?;
        let lineid = task.lineid();

        let translation = text_phonemetrail
            .hypothesis
            .as_ref()
            .ok_or_else(|| String::from("text to phoneme translator returned empty string."))?;
        debug!("id {lineid:10}: phoneme translation: {translation}");

        let mut dataprovider = DataProvider::new_from_taskdata(task);
        dataprovider.load()?;

        // pocketsphinx requires the audiodata to be 16khz
        // in addition normalize audio to improve detection for low volume audio
        let audiodata = dataprovider.get_rawaudio(16000, true)?;

        info!("id {lineid:010}: extracting phonemes from audio... (this may take a while)");
        let phonetizer = WordPhonetizer::new(
            self.matcher.phoneme_pairing_alternatives(),
            &self.translator
        );

        let audio_phonemetrail =
            self.extractor
                .extract_phonemes(lineid, &audiodata, text, &phonetizer)?;

        let recognized = audio_phonemetrail
            .hypothesis
            .as_ref()
            .ok_or_else(|| String::from("audio phoneme extractor returned empty result."))?;
        debug!("id {lineid:010}: phonemes from audio: {recognized}");

        let phonemetrail = self.matcher.calculate_matching(
            task.lineid(),
            &audio_phonemetrail,
            &text_phonemetrail,
        )?;
        let phonemecount = phonemetrail.phonemes.len();

        let mut phonemetrack = PhonemeTrack::new(
            task.lineid(),
            self.strings.get_lang(),
            original_text,
            translation,
            Some(recognized.to_owned()),
            actor.cloned(),
            phonemetrail.phonemes,
        );

        let duration_in_ms = f32::trunc(dataprovider.get_audio_duration() * 1000.0) as u32;
        let gaps_closed = phonemes::auto_close_gaps(duration_in_ms, &mut phonemetrack);
        if gaps_closed > 0 {
            warn!("id {lineid:010}: > auto-closed #{gaps_closed} phoneme timing gaps found within a word boundary.",);
        }
        phonemetrack.assess_quality();

        let mut outputdir = PathBuf::from(task.audiofile());
        outputdir.pop();

        let phonemefile = phonemes::store(&outputdir, phonemetrack)?;

        info!("id {lineid:010}: stored #{phonemecount} phoneme timings in [{phonemefile}]");

        Ok(phonemefile)
    }
    // ------------------------------------------------------------------------
    fn rename_audiofile(&mut self, task: &TaskData, full_rename: bool) -> Result<String, String> {
        use std::fs;

        let mut dataprovider = DataProvider::new_from_taskdata(task);
        dataprovider.load()?;

        debug!("extracting duration from audio...");
        let duration = dataprovider.get_audio_duration();

        let old_audiofile = PathBuf::from(task.audiofile());

        match old_audiofile.file_name() {
            Some(filename) => {
                let filename = if full_rename {
                    let line = self.strings.get_line(task.lineid())?;
                    format!(
                        "{}.{}",
                        escape_textline(line)?,
                        old_audiofile
                            .extension()
                            .unwrap_or_default()
                            .to_string_lossy()
                    )
                } else {
                    // remove max-10 digit prefix and use remaining filename for renaming
                    filename
                        .to_string_lossy()
                        .chars()
                        .skip_while(char::is_ascii_digit)
                        .collect::<String>()
                };
                let actor = self.strings.get_actor(task.lineid());
                let new_audiofile = format!(
                    "{:0>10}[{:.4}]{}{}",
                    task.lineid(),
                    duration,
                    ::escape_actor(&actor.map(|a| format!("-{a}-")).unwrap_or_default())?,
                    filename
                );

                info!("renaming audiofile to: {}", new_audiofile);
                let new_audiofile = old_audiofile.with_file_name(new_audiofile);

                fs::rename(&old_audiofile, &new_audiofile).map_err(|err| {
                    format!("failed to rename {}: {}", old_audiofile.display(), err)
                })?;

                Ok(new_audiofile.to_string_lossy().to_string())
            }
            _ => Err(
                "could not extract filename without id prefix for renaming operation.".to_string(),
            ),
        }
    }
    // ------------------------------------------------------------------------
    pub fn process(&mut self, task: TaskData) -> TaskResult {
        use queue::TaskOperation::*;

        match task.operation() {
            Extract => match self.extract_phonemes(&task) {
                Ok(phoneme_file) => task.set_phonemefile(phoneme_file),
                Err(why) => {
                    error!("{} skipping id [{:>10}]...", &why, task.lineid());
                    task.set_error(why)
                }
            },
            Rename(full_rename) => match self.rename_audiofile(&task, full_rename) {
                Ok(new_audiofile) => task.set_audiofile(new_audiofile),
                Err(why) => {
                    error!("{} skipping id [{:>10}]...", &why, task.lineid());
                    task.set_error(why)
                }
            },
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Generator {
    // ------------------------------------------------------------------------
    pub fn new(language: &str, translator: TextPhonemeConverter, outputdir: PathBuf) -> Generator {
        Generator {
            language: language.to_owned(),
            translator,
            outputdir,
        }
    }
    // ------------------------------------------------------------------------
    pub fn generate(&self, id: u32, actor: &str, text: &str) -> Result<String, String> {
        let phonemetrail = self.translator.translate(text)?;

        let translation = phonemetrail
            .hypothesis
            .as_ref()
            .ok_or_else(|| String::from("text to phoneme translator returned empty string."))?;
        debug!("id {id:10}: phoneme translation: {translation}");

        let phonemecount = phonemetrail.phonemes.len();

        // Note: quality assesment not needed as the track is instantly saved and
        // not used in gui
        let phonemefile = phonemes::store(
            &self.outputdir,
            PhonemeTrack::new(
                id,
                &self.language,
                text,
                translation,
                None,
                Some(actor.to_owned()),
                phonemetrail.phonemes,
            ),
        )?;

        info!(
            "id {:010}: stored #{} phoneme timings in [{}]",
            id, phonemecount, &phonemefile
        );

        Ok(phonemefile)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
pub struct WordPhonetizer<'translator, 'mapping> {
    translator: &'translator TextPhonemeConverter,
    /// highest score pairs from sim matrix (mapping ipa -> pocketsphnix phoneme)
    mapping: &'mapping HashMap<String, String>,
}
// ----------------------------------------------------------------------------
impl<'translator, 'mapping> WordPhonetizer<'translator, 'mapping> {
    // ------------------------------------------------------------------------
    fn new(
        scores: &'mapping HashMap<String, String>,
        translator: &'translator TextPhonemeConverter,
    ) -> Self {
        Self {
            translator,
            mapping: scores,
        }
    }
    // ------------------------------------------------------------------------
    fn phonetize(&self, word: &str) -> Option<String> {
        if let Ok(trail) = self.translator.translate(word.trim()) {
            let mut phonemes = Vec::with_capacity(trail.phonemes.len());
            for ipa in &trail.phonemes {
                if let Some(phoneme) = self.mapping.get(&ipa.phoneme) {
                    phonemes.push(phoneme.as_str());
                } else {
                    return None;
                }
            }
            Some(phonemes.join(" "))
        } else {
            None
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
pub struct DataProvider {
    audiofile: String,

    audio_modified: bool,
    audiodata: Vec<i16>,
}
// ----------------------------------------------------------------------------
const REQUIRED_SAMPLE_RATE: u32 = 44100;
// ----------------------------------------------------------------------------
impl DataProvider {
    // ------------------------------------------------------------------------
    pub fn new(audiofile: &str) -> DataProvider {
        DataProvider {
            audiofile: audiofile.to_owned(),
            audio_modified: false,
            audiodata: Vec::default(),
        }
    }
    // ------------------------------------------------------------------------
    pub fn new_from_taskdata(taskinfo: &TaskData) -> DataProvider {
        DataProvider::new(taskinfo.audiofile())
    }
    // ------------------------------------------------------------------------
    pub fn load(&mut self) -> Result<(), String> {
        debug!("reading audio data from {}", self.audiofile);

        let data = audio::AudioLoader::load(&self.audiofile, REQUIRED_SAMPLE_RATE)?;
        self.audiodata = data.values;
        self.audio_modified = data.resampled;

        Ok(())
    }
    // ------------------------------------------------------------------------
    pub fn get_audio_duration(&self) -> f32 {
        self.audiodata.len() as f32 / REQUIRED_SAMPLE_RATE as f32
    }
    // ------------------------------------------------------------------------
    pub fn get_rawaudio(&self, sample_rate: u32, normalized: bool) -> Result<Vec<i16>, String> {
        if sample_rate == REQUIRED_SAMPLE_RATE {
            Ok(self.audiodata.clone())
        } else {
            trace!("> resampling audio to {}Hz", sample_rate);

            Ok(audio::AudioResampler::resample(
                &self.audiodata,
                REQUIRED_SAMPLE_RATE,
                sample_rate,
                normalized,
            )?)
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// utility functions
// ----------------------------------------------------------------------------

use logger::LevelFilter;
use std::path::{Path, PathBuf};

pub fn init_phoneme_extraction(
    language: &str,
    stringsfile: &Path,
    datadir: &Path,
    loglevel: LevelFilter,
) -> Result<Processor<CsvStringsData>, String> {
    let similarity_file = datadir.join(format!("{language}.phoneme.similarity.csv"));

    info!("loading strings file {}", stringsfile.display());
    let strings_provider = CsvStringsData::load_with_language(stringsfile, Some(language))
        .map_err(|e| {
            format!(
                "could not create string provider from \"{}\": {}.",
                stringsfile.display(),
                e
            )
        })?;

    info!("initializing text to phoneme translator (eSpeak)");
    let mut translator = TextPhonemeConverter::new(&datadir.to_string_lossy());

    translator
        .init()
        .and(translator.set_language(language, Some(format!("{language}.espeak.custom.dict"))))?;

    info!("initializing audio phoneme extractor (pocketsphinx)");
    let extractor = AudioPhonemeExtractor::new(&datadir.to_string_lossy(), language, loglevel)?;

    info!(
        "loading phoneme similarity matrix for phoneme sequence matcher {}",
        similarity_file.display()
    );

    let similarity_matrix = PhonemeSimilarityMatrix::load(&similarity_file)?;
    let matcher = PhonemeSequenceMatcher::new(similarity_matrix);

    Ok(Processor::new(
        strings_provider,
        translator,
        extractor,
        matcher,
    ))
}
// ----------------------------------------------------------------------------
pub fn init_phoneme_generation(
    language: &str,
    stringsfile: &Path,
    datadir: &Path,
    outputdir: &Path,
) -> Result<(Generator, Box<dyn StringsProvider>), String> {
    info!("loading strings file {}", stringsfile.display());
    let strings_provider = CsvStringsData::load_with_language(stringsfile, Some(language))
        .map_err(|e| format!("could not create string provider: {}", e))?;

    info!("initializing text to phoneme translator (eSpeak)");
    let mut translator = TextPhonemeConverter::new(&datadir.to_string_lossy());

    translator
        .init()
        .and(translator.set_language(language, Some(format!("{language}.espeak.custom.dict"))))?;

    Ok((
        Generator::new(language, translator, outputdir.to_owned()),
        Box::new(strings_provider),
    ))
}
// ----------------------------------------------------------------------------
pub fn find_missing_audio(
    datadir: PathBuf,
    stringsfile: PathBuf,
    language: Option<&str>,
) -> Result<BTreeMap<u32, (String, Option<String>)>, String> {
    use file_scanner::FileInfo;

    info!("loading strings file {}", stringsfile.display());
    let strings_provider =
        CsvStringsData::load_with_language(&stringsfile, language).map_err(|e| {
            format!(
                "could not create string provider from \"{}\": {}.",
                stringsfile.display(),
                e
            )
        })?;

    let mut scanner = file_scanner::FilesScanner::new(datadir)?;

    let mut audio = HashMap::new();
    let mut unassigned = Vec::new();

    for file in scanner.scan()? {
        #[allow(clippy::map_entry)]
        match file {
            FileInfo::UnlinkedAudio(ref filepath) => {
                info!("found unlinked audiofile: {}", filepath);
                unassigned.push(filepath.to_owned());
            }
            FileInfo::Audio(id, ref filepath, duration) => {
                if audio.contains_key(&id) {
                    warn!(
                        "found duplicate audiofile for id [{}]: {}. skipping... ",
                        id, filepath
                    );
                } else {
                    audio.insert(id, (filepath.to_owned(), duration.is_some()));
                }
            }
            FileInfo::Phonemes(_, _) => {}
        }
    }
    info!("found {} audio files", audio.len());

    let mut missing_audio = BTreeMap::new();

    for (lineid, data) in strings_provider.get_all_lines() {
        if !audio.contains_key(lineid) {
            missing_audio.insert(*lineid, data.clone());
        }
    }

    info!(
        "> {} / {} textlines without audio file ({:.3} %).",
        missing_audio.len(),
        strings_provider.line_count(),
        100.0 * (missing_audio.len() as f32 / strings_provider.line_count() as f32)
    );

    Ok(missing_audio)
}
// ----------------------------------------------------------------------------
pub fn extract_language_info(audiodir: &Path) -> Option<String> {
    if let Some(last_dir) = audiodir.components().last() {
        use std::path::Component;

        if let Component::Normal(last_dir) = last_dir {
            let last_dir = last_dir.to_string_lossy().to_lowercase();

            // check for these cases
            //  - <lang>.*
            //  - speech.<lang>.*

            let last_dir = last_dir
                .strip_prefix("speech.")
                .unwrap_or_else(|| &last_dir);
            if let Some((lang, _)) = last_dir.split_once('.') {
                if lang.len() == 2 {
                    info!("detected language in {}: {lang}", audiodir.display());
                    return Some(lang.to_owned());
                }
            }
        }
    }
    None
}
// ----------------------------------------------------------------------------
/// searches for a csv or strings-csv file in dir or its parent dir
pub fn search_strings_file(dir: &PathBuf) -> Result<PathBuf, String> {
    use std::io;

    let result = || -> io::Result<Option<PathBuf>> {
        let mut parentdir = dir.clone();
        parentdir.pop();

        let is_csv = |f: &str| f.ends_with(".csv") || f.ends_with(".strings-csv");

        for dir in &[dir, &parentdir] {
            for entry in dir.read_dir()? {
                let entry = entry?;

                if entry.file_type()?.is_file() && entry.file_name().to_str().map_or(false, is_csv)
                {
                    return Ok(Some(entry.path()));
                }
            }
        }
        Ok(None)
    }()
    .map_err(|e| e.to_string())?;

    result.ok_or_else(|| {
        format!(
            "no strings-file found in: {} (or its parent)",
            dir.display()
        )
    })
}
// ----------------------------------------------------------------------------
/// searches for a cfg file based on stringsfile location. if none is found an
/// empty default mapping is created
pub fn search_actor_mappings_file(stringsfile: &Path) -> Result<PathBuf, String> {
    let mut mapping_file = stringsfile.to_path_buf();
    mapping_file.set_file_name("actor_mapping.cfg");

    if !mapping_file.is_file() {
        info!(
            "no actor-mappings-file found. creating new in: {}",
            mapping_file.display()
        );
        ActorMapping::create_new(&mapping_file)?;
    }
    Ok(mapping_file)
}
// ----------------------------------------------------------------------------
// some helper
// ----------------------------------------------------------------------------
const TEXTHINT_CHARS_MAX: usize = 50;
// ----------------------------------------------------------------------------
fn escape_textline(line: &str) -> Result<String, String> {
    let replacer = regex::Regex::new("[/\\?%*:|<>.$…, \"]")
        .map_err(|err| format!("failed to initialize filename escape regex: {}", err))?;

    Ok(replacer
        .replace_all(line, "_")
        .chars()
        .take(TEXTHINT_CHARS_MAX)
        .collect())
}
// ----------------------------------------------------------------------------
fn escape_actor(line: &str) -> Result<String, String> {
    let replacer = regex::Regex::new("[/\\?%*:|<>.$…, \"]")
        .map_err(|err| format!("failed to initialize filename escape regex: {}", err))?;

    Ok(replacer
        .replace_all(line, "_")
        .chars()
        .take(TEXTHINT_CHARS_MAX)
        .collect())
}
// ----------------------------------------------------------------------------
// ----------------------------------------------------------------------------
use regex::Regex;

lazy_static! {
    static ref REGEXP_CLEANUP: Regex = Regex::new("\\*[^*]+\\*").unwrap();
}
// ----------------------------------------------------------------------------
