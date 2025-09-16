//
// phoneme extractor
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
extern crate byteorder;
extern crate pocketsphinx;

pub struct PocketSphinx {
    decoder: pocketsphinx::Decoder,
    translator: Translator,
}
// ----------------------------------------------------------------------------
struct Translator {
    pocketsphinx: pocketsphinx::Decoder,
    custom_dict: HashMap<String, String>,
    character_rules: IndexMap<char, String>,
    grammer_template: String,
    cleanup_chars: Vec<char>,
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
struct Config {
    model_dir: String,
    /// phoneme set used
    phoneme_set: Vec<String>,
    /// additional cleanup characters
    cleanup: Vec<char>,
    /// mappings from IPA phonemes to a set of alternative pocketsphinx phonemes.
    /// used to translate unknown words letter by letter
    mapping: IndexMap<char, String>,

    /// filename of language dependent phoneme model
    phoneme_model: String,
    /// filename of language dependent generated phoneme set dictionary
    phoneme_dictionary: String,
    /// filename of language dependent dictionary
    language_dictionary: String,
    /// filename of language dependent custom dictionary
    custom_dictionary: String,

    /// path to language dependent phoneme model
    phoneme_model_path: String,
    /// path to language dependent generated phoneme set dictionary
    phoneme_dictionary_path: String,
    /// path to language dependent dictionary
    language_dictionary_path: String,
    /// path to language dependent noise dictionary
    noise_dictionary_path: String,
    /// path to language dependent custom dictionary with
    custom_dictionary_path: String,
}
// ----------------------------------------------------------------------------
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use indexmap::IndexMap;
use logger::LevelFilter;

use super::WordPhonetizer;

use phonemes::{PhonemeResult, PhonemeSegment};
use pocketsphinx::pocketsphinx::{Config as PocketSphinxConfig, SegFrames};
// ----------------------------------------------------------------------------
#[cfg(target_os = "linux")]
fn get_null_logfile() -> &'static str {
    "/dev/null"
}
#[cfg(target_os = "windows")]
fn get_null_logfile() -> &'static str {
    "NUL"
}
// ----------------------------------------------------------------------------
impl Config {
    // ------------------------------------------------------------------------
    fn load(cfg_dir: &str, language: &str) -> Result<Self, String> {
        use std::fs;

        let cfg_name = format!("{language}.pocketsphinx.cfg");
        let cfg_file = format!("{cfg_dir}/{cfg_name}");
        let cfg_path = check_file(&cfg_file)?;

        let mut mapping = IndexMap::new();
        let mut phonemes = Vec::new();
        let mut cleanup = vec!['?', '!', ';', ',', '.', ':', '-', '"', '…', '*', '/', '+'];
        let mut model_dictionary = None;
        let mut phoneme_model = None;
        let mut noise_dictionary = "noisedict".to_string();

        let conf = fs::read_to_string(cfg_path)
            .map_err(|err| format!("failed to load {cfg_name}: {err}"))?;

        for (i, line) in conf
            .lines()
            .enumerate()
            .filter(|(_i, line)| !line.is_empty() && !line.starts_with(';'))
        {
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("{cfg_name}:{} failed to parse: {line}", i + 1))?;

            match key.trim().to_lowercase().as_str() {
                "phonemes" => {
                    phonemes = value
                        .trim()
                        .split(' ')
                        .map(|a| a.trim().to_string())
                        .filter(|a| !a.is_empty())
                        .collect::<Vec<_>>();
                    phonemes.sort();
                    phonemes.dedup();
                }
                "cleanup" => {
                    cleanup.extend(value.trim().chars());
                    cleanup.sort();
                    cleanup.dedup();
                }
                "phoneme-model" => {
                    phoneme_model = Some(value.trim().to_string());
                }
                "dictionary" => {
                    model_dictionary = Some(value.trim().to_string());
                }
                "noise-dictionary" => {
                    noise_dictionary = value.trim().to_string();
                }
                character if character.chars().count() == 1 => {
                    let alternatives = value
                        .trim()
                        .split(' ')
                        .map(|a| a.trim())
                        .filter(|a| !a.is_empty())
                        .collect::<Vec<_>>();
                    let value = if alternatives.is_empty() {
                        return Err(format!("{cfg_name}:{} expected at least one alternative for character {character}", i + 1));
                    } else if alternatives.len() > 1 {
                        format!("({})", alternatives.join("|"))
                    } else {
                        alternatives.first().unwrap().to_string()
                    };

                    mapping.insert(character.chars().next().unwrap(), value);
                }
                _ => {
                    return Err(format!(
                        "{cfg_name}:{} expected exactly one character as mapping key. found {key}",
                        i + 1
                    ))
                }
            }
        }

        // verify that all mappings contain valid phonemes
        let set: HashSet<_> = phonemes.iter().map(|s| s.as_str()).collect();

        for (c, rule) in mapping.iter() {
            for p in rule
                .replace(['|', '(', ')'], " ")
                .split(' ')
                .filter(|p| !p.is_empty())
            {
                if !set.contains(p) {
                    return Err(format!(
                        "{cfg_name} character mapping for '{c}' contains invalid phoneme '{p}'"
                    ));
                }
            }
        }

        // expand info into filenames and full paths
        let phoneme_model = phoneme_model
            .ok_or_else(|| format!("{cfg_name} pocketsphinx phoneme model definition not found"))?;
        let language_dictionary = model_dictionary.ok_or_else(|| {
            format!("{cfg_name} pocketsphinx language dictionary definition not found")
        })?;
        let model_dir = format!("{cfg_dir}/pocketsphinx/{language}");

        let phoneme_dictionary = format!("{language}-phoneme.dict");
        let custom_dictionary = format!("{language}.pocketsphinx.custom.dict");

        let phoneme_model_path = format!("{model_dir}/{phoneme_model}");
        let phoneme_dictionary_path = format!("{model_dir}/{phoneme_dictionary}");
        let language_dictionary_path = format!("{model_dir}/{language_dictionary}");
        let noise_dictionary_path = format!("{model_dir}/{noise_dictionary}");
        let custom_dictionary_path = format!("{cfg_dir}/{custom_dictionary}");

        // check if files exist
        let model_dir = check_dir(&model_dir)?;
        let phoneme_model_path = check_file(&phoneme_model_path)?;
        let language_dictionary_path = check_file(&language_dictionary_path)?;
        let noise_dictionary_path = check_file(&noise_dictionary_path)?;
        // let phoneme_dictionary_path = check_file(&phoneme_dictionary_path)?;
        let custom_dictionary_path = check_file(&custom_dictionary_path)?;

        Ok(Self {
            model_dir: model_dir.to_string(),
            phoneme_set: phonemes,
            cleanup,
            mapping,

            phoneme_model,
            phoneme_dictionary,
            custom_dictionary,
            language_dictionary,

            phoneme_model_path,
            phoneme_dictionary_path,
            language_dictionary_path,
            noise_dictionary_path,
            custom_dictionary_path,
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl PocketSphinx {
    // ------------------------------------------------------------------------
    pub fn new(modeldir: &str, language: &str, loglevel: LevelFilter) -> Result<Self, String> {
        let config = Config::load(modeldir, language)?;

        let translator =
            Translator::new(&config, loglevel).map_err(|err| format!("pocketsphinx: {err}"))?;

        debug!("initializing pocketsphinx:");
        debug!("> model directory:    [{}]", config.model_dir);
        debug!("> phoneme model:      [{}]", config.phoneme_model);
        debug!("> phoneme dictionary: [{}]", config.phoneme_dictionary);

        // audio -> phoneme decoder
        // Create a config and set default acoustic model, dictionary, and language model
        let decoder = Self::create_config(
            "decoder",
            &[
                ("hmm", &config.model_dir),
                ("allphone", &config.phoneme_model_path),
                ("backtrace", "yes"),
                // see: https://github.com/cmusphinx/pocketsphinx/issues/318
                // "FSG recognition with -bestpath is often harmful"
                ("bestpath", "false"),
                ("remove_noise", "true"),
                ("fsgusefiller", "false"),
                ("fdict", &config.noise_dictionary_path),
                ("dict", &config.phoneme_dictionary_path),
            ],
            loglevel,
        )?
        .init_decoder()
        .map_err(|err| format!("failed to create pocketsphinx decoder: {err}"))?;

        Ok(PocketSphinx {
            decoder,
            translator,
        })
    }
    // ------------------------------------------------------------------------
    fn create_config(
        name: &str,
        options: &[(&str, &str)],
        loglevel: LevelFilter,
    ) -> Result<pocketsphinx::Config, String> {
        let mut config = PocketSphinxConfig::new()
            .map_err(|err| format!("pocketsphinx: failed to init new {name} config: {err}"))?;

        trace!(">> cli parameters:");
        for (setting, value) in options {
            trace!(">> -{setting} {value}");
            config.set_str(setting, value).map_err(|err| {
                format!("pocketsphinx: failed to set {name} config setting {setting}: {err}")
            })?;
        }

        // disable pocketsphinx logging
        if loglevel != LevelFilter::Trace {
            config.set_str("logfn", get_null_logfile()).map_err(|err| {
                format!("pocketsphinx: failed to set config setting logfn: {err}")
            })?;
        }

        Ok(config)
    }
    // ------------------------------------------------------------------------
    fn get_hypothesis(
        &mut self,
        search_id: &str,
        raw_audio_data: &[i16],
    ) -> Result<Option<(String, i32)>, String> {
        self.decoder
            .set_activate_search(search_id)
            .map_err(|err| format!("pocketsphinx: failed to activate search: {err}"))?;

        self.decoder
            .start_utt()
            .map_err(|err| format!("pocketsphinx: {err}"))?;

        // returns number of frames of data that was searched, or <0 for error
        let frames = self
            .decoder
            .process_raw(raw_audio_data, false, true)
            .map_err(|err| format!("pocketsphinx: {err}"))?;

        self.decoder
            .end_utt()
            .map_err(|err| format!("pocketsphinx: {err}"))?;

        trace!(">> {} frames of data searched", frames);

        self.decoder
            .get_hyp()
            .map_err(|err| format!("pocketsphinx: {err}"))
    }
    // ------------------------------------------------------------------------
    pub fn extract_phonemes(
        &mut self,
        lineid: u32,
        raw_audio_data: &[i16],
        text: &str,
        phonetizer: &WordPhonetizer,
    ) -> Result<PhonemeResult, String> {
        trace!("> pocketsphinx: extracting phonemes...");

        let grammer = self.translator.generate_grammer(text, phonetizer);
        self.decoder
            .add_jsgf_string("textline", &grammer)
            .map_err(|err| format!("{lineid:010}: pocketsphinx: failed to add grammer: {err}"))?;

        let hypothesis = match self.get_hypothesis("textline", raw_audio_data)? {
            Some((hypothesis, _score)) => hypothesis,
            None => {
                // TODO should this be cleaned up in any case?
                self.decoder
                    .remove_search("textline")
                    .map_err(|err| format!("pocketsphinx: failed to deactivate search: {err}"))?;

                // fallback
                // retry without grammar constraint
                error!("{lineid:010}: pocketsphinx: > failed to extract hypothesis from audio. retrying without constraints...");

                let Some((fallback_hypothesis, _score)) =
                    self.get_hypothesis("_default", raw_audio_data)?
                else {
                    return Err(format!(
                        "{lineid:010}: pocketsphinx: failed to extract hypothesis from audio."
                    ));
                };

                fallback_hypothesis
            }
        };

        let mut result = PhonemeResult {
            hypothesis: Some(hypothesis),
            phonemes: Vec::new(),
        };

        // WORKAROUND for duped segments
        let mut prev_start = -1;
        for segment in self
            .decoder
            .get_seg_iter()
            .ok_or_else(|| format!("{lineid:010}: pocketsphinx: failed to get segments"))?
        {
            // remap different noise/sil phonemes
            let phoneme = match segment.get_word().as_str() {
                "<s>" | "<sil>" | "</s>" | "(NULL)"  => "SIL".to_string(),
                p => p.to_string(),
            };

            let SegFrames { start, end } = segment.get_frames();

            // skip zero-duration sil segments
            if end.saturating_sub(start) <= 0 && &phoneme == "SIL" {
                continue;
            }
            if prev_start < start {
                prev_start = start;
                result.phonemes.push(PhonemeSegment {
                    phoneme: phoneme.to_owned(),
                    // no information about word boundaries
                    word_start: false,
                    // save timings as ms
                    start: start as u32 * 10,
                    end: end as u32 * 10,
                    weight: 1.0,
                    score: 0.0,
                    matching_info: None,
                    traceback: None,
                    active: true,
                    warnings: Vec::default(),
                });
            }
        }
        // postprocessing:
        // remove timing gaps between neighboring phonemes (except SIL) by
        // extending start & end
        let len = result.phonemes.len();
        if len > 2 {
            let mut is_prev_sil = true;

            for i in 0..len {
                is_prev_sil = if result.phonemes[i].phoneme != "SIL" {
                    if !is_prev_sil {
                        result.phonemes[i].start -= 5;
                    }
                    false
                } else {
                    true
                };

                if i + 1 < len && result.phonemes[i + 1].phoneme != "SIL" {
                    result.phonemes[i].end += 5;
                }
            }
        }

        Ok(result)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Translator {
    // ------------------------------------------------------------------------
    fn new(config: &Config, loglevel: LevelFilter) -> Result<Self, String> {
        let custom_dict = Self::load_custom_dictionary(config)?;

        Self::verify_phoneme_dictionary(config)?;

        debug!("initializing pocketsphinx translator:");
        debug!("> model directory:     [{}]", config.model_dir);
        debug!("> language dictionary: [{}]", config.language_dictionary);

        let pocketsphinx = PocketSphinx::create_config(
            "translator",
            &[
                ("hmm", &config.model_dir),
                ("dict", &config.language_dictionary_path),
                ("fdict", &config.noise_dictionary_path),
            ],
            loglevel,
        )?
        .init_decoder()
        .map_err(|err| format!("failed to create pocketsphinx translator: {err}"))?;

        Ok(Self {
            pocketsphinx,
            custom_dict,
            grammer_template: Self::prepare_grammer_template(&config.phoneme_set, &config.mapping),
            character_rules: config.mapping.to_owned(),
            cleanup_chars: config.cleanup.to_owned(),
        })
    }
    // ------------------------------------------------------------------------
    fn load_custom_dictionary(config: &Config) -> Result<HashMap<String, String>, String> {
        use std::fs;

        let filename = &config.custom_dictionary;
        let file = &config.custom_dictionary_path;

        // verify that all entries map to valid phonemes
        let set: HashSet<_> = config.phoneme_set.iter().map(|s| s.as_str()).collect();

        let mut dict = HashMap::default();

        let conf =
            fs::read_to_string(file).map_err(|err| format!("failed to load {filename}: {err}"))?;

        for (i, line) in conf
            .lines()
            .enumerate()
            .filter(|(_i, line)| !line.is_empty() && !line.starts_with(';'))
        {
            let (word, rule) = line
                .split_once(' ')
                .ok_or_else(|| format!("{filename}:{} failed to parse: {line}", i + 1))?;
            let rule = rule.trim();

            for p in rule
                .replace(['|', '(', ')', '[', ']'], " ")
                .split(' ')
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
            {
                if !set.contains(p) {
                    return Err(format!(
                        "{filename}:{} word mapping for '{word}' contains invalid phoneme '{p}'",
                        i + 1
                    ));
                }
            }

            dict.insert(word.to_lowercase(), rule.to_string());
        }
        Ok(dict)
    }
    // ------------------------------------------------------------------------
    fn verify_phoneme_dictionary(config: &Config) -> Result<(), String> {
        use std::fs::{self, File};
        use std::io::Write;

        let filename = &config.phoneme_dictionary;
        let dictionary = PathBuf::from(&config.phoneme_dictionary_path);

        let mut expected_set: HashSet<_> = config.phoneme_set.iter().map(|s| s.as_str()).collect();
        expected_set.insert("SIL");
        let mut found_set = HashSet::new();

        if dictionary.is_file() {
            // read and verify it contains all phonemes in phonemset
            let content = fs::read_to_string(&dictionary)
                .map_err(|err| format!("failed to load {filename}: {err}"))?;

            // extract phonemes
            for (i, line) in content
                .lines()
                .enumerate()
                .filter(|(_i, line)| !line.trim().is_empty())
            {
                let (phoneme, _col2) = line
                    .split_once(' ')
                    .ok_or_else(|| format!("{filename}:{} failed to parse: {line}", i + 1))?;
                found_set.insert(phoneme);
            }

            // check if all phonemes defined in phoneme-mapping are present. if
            // not the dict has to be recreated
            if expected_set == found_set {
                return Ok(());
            }
            warn!("contents of {filename} are out of sync with defined set of phonemes in phoneme mapping config.");
        }

        // generate new file
        info!("pocketsphinx: (re)creating phoneme dictionary {filename}");

        let mut output = File::create(dictionary)
            .map_err(|err| format!("{filename}: failed to create phoneme dictionary: {err}"))?;

        for phoneme in &config.phoneme_set {
            writeln!(output, "{phoneme} {phoneme}")
                .map_err(|err| format!("{filename}: failed to write to dictionary file: {err}"))?;
        }
        writeln!(output, "SIL SIL")
            .map_err(|err| format!("{filename}: failed to write to dictionary file: {err}"))?;

        Ok(())
    }
    // ------------------------------------------------------------------------
    fn prepare_grammer_template(
        phoneme_set: &[String],
        character_rules: &IndexMap<char, String>,
    ) -> String {
        let phoneme_set = phoneme_set.join(" | ");
        let phoneme_mappings = character_rules
            .iter()
            .map(|(character, alternatives)| format!("<{character}> = {};", alternatives))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "\
            #JSGF V1.0;
            grammar textline;

            public <textline> = [[TEXTLINE_GRAMMER]];
            {phoneme_mappings}
            <unknown> = [({phoneme_set})];\
        "
        )
    }
    // ------------------------------------------------------------------------
    fn generate_grammer(&self, text: &str, phonetizer: &WordPhonetizer) -> String {
        let input = text
            .to_lowercase()
            .replace(self.cleanup_chars.as_slice(), " ");

        let map_to_rule = |c: char| -> &str {
            match self.character_rules.get(&c) {
                Some(s) => s.as_str(),
                None => "<unknown>",
            }
        };
        let grammer = input
            .split(' ')
            .filter(|w| !w.trim().is_empty())
            // .inspect(|w| println!("word: {w}"))
            .map(|word| {
                let word = word.trim();
                self.custom_dict
                    .get(word)
                    .inspect(|w| {
                        info!("pocketsphinx: > using custom dictionary word: {word} -> {w}")
                    })
                    .map(|w| w.to_string())
                    .or_else(|| self.pocketsphinx.lookup_word(word).ok().flatten())
                    // translate word: text -> ipa -> pocketsphinx-phoneme
                    .or_else(|| {
                        phonetizer
                            .phonetize(word)
                            .inspect(|w| {
                                info!("pocketsphinx: > phonetizing IPA word translation: {word} -> {w}")
                            })
                    })
                    // last fallback: text chars -> pocketsphinx-phonemes
                    .unwrap_or_else(|| {
                        let mut chars = word.replace('\'', "").chars().collect::<Vec<_>>();
                        chars.dedup();
                        let grammer = chars.iter().copied().map(map_to_rule).collect::<Vec<_>>().join(" ");
                        info!("pocketsphinx: > using fallback character matching for: {word} -> {grammer}");
                        grammer
                    })
            })
            .collect::<Vec<_>>()
            .join(" [ SIL+ ] ");

        let grammer = format!("[SIL] {grammer} [SIL]");

        // info!("text: {text}\ngrammer: {grammer}");

        self.grammer_template
            .replace("[[TEXTLINE_GRAMMER]]", &grammer.replace("  ", " "))
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
fn check_dir(dir: &str) -> Result<String, String> {
    let dirpath = PathBuf::from(&dir);
    if !dirpath.exists() || !dirpath.is_dir() {
        Err(format!("directory [{}] does not exist", dirpath.display()))
    } else {
        Ok(dir.to_owned())
    }
}
// ----------------------------------------------------------------------------
fn check_file(file: &str) -> Result<String, String> {
    let filepath = PathBuf::from(&file);
    if !filepath.exists() || !filepath.is_file() {
        Err(format!("file [{}] does not exist", filepath.display()))
    } else {
        Ok(file.to_owned())
    }
}
// ----------------------------------------------------------------------------
