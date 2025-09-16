//
// rust wrapper for espeak-sys low level bindings
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
extern crate espeak_sys as bindings;
extern crate lazy_static;
extern crate libc;
extern crate log;
extern crate regex;

pub struct ESpeak {
    datadir: String,
    dict: HashMap<String, String>,
    /// additional cleanup characters
    cleanup: Vec<char>,
}

// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::collections::HashMap;

use libc::{c_char, c_int};
use log::{debug, info};
use std::ffi::{CStr, CString};
use std::ptr;

use bindings::{
    espeakCHARS_UTF8, espeakINITIALIZE_PHONEME_EVENTS, espeakINITIALIZE_PHONEME_IPA,
    espeak_AUDIO_OUTPUT, espeak_ERROR,
};

use lazy_static::lazy_static;
// ----------------------------------------------------------------------------
impl ESpeak {
    // ------------------------------------------------------------------------
    pub fn new(datadir: &str) -> ESpeak {
        ESpeak {
            datadir: datadir.to_owned(),
            dict: HashMap::default(),
            cleanup: Vec::default(),
        }
    }
    // ------------------------------------------------------------------------
    pub fn init(&self) -> Result<(), String> {
        let data_dir = CString::new(self.datadir.clone()).unwrap();

        let res = unsafe {
            bindings::espeak_Initialize(espeak_AUDIO_OUTPUT::AUDIO_OUTPUT_SYNCHRONOUS,
                0,  // Buffer length. 0 == 200ms
                // eSpeak-data dir
                data_dir.as_ptr(),
                // Options.
                espeakINITIALIZE_PHONEME_EVENTS + espeakINITIALIZE_PHONEME_IPA,
            )
        };

        if res == espeak_ERROR::EE_INTERNAL_ERROR as i32 {
            Err("eSpeak: failed to initialize.".to_owned())
        } else {
            unsafe {
                // TODO make input parameter
                // value=0  No phoneme output (default)
                // value=1  Output the translated phoneme symbols for the text
                // value=2  as (1), but also output a trace of how the translation
                //          was done (matching rules and list entries)
                // value=3  as (1), but produces IPA rather than ascii phoneme names
                bindings::espeak_SetPhonemeTrace(3, ptr::null_mut());
            }
            Ok(())
        }
    }
    // ------------------------------------------------------------------------
    fn process_replacement_word(input: &str) -> String {
        let mut result = String::new();

        let mut s = input.chars().peekable();
        if input.contains('-') {
            result = input.replace('-', "_");
        } else {
            while let Some(c) = s.next() {
                result.push(c);
                if !(c == 'ˈ' || c == '"' || c == 'ˌ' || s.peek() == Some(&'ː')) {
                    result.push('_');
                }
            }
        }
        result.trim_matches('_').to_string()
    }
    // ------------------------------------------------------------------------
    pub fn set_language(&mut self, language: &str, dictfile: Option<String>) -> Result<(), String> {

        let mut cleanup = vec!['-', '…', '*', '/', '+', '\\'];

        // load optional custom dictionary
        if let Some(dictfile) = dictfile {
            if let Ok(data) = std::fs::read_to_string(format!("{}/{dictfile}", self.datadir)) {
                info!("espeak: found custom dictionary {dictfile}");
                for line in data.lines().filter(|line| !line.starts_with(';')) {
                    if let Some((key, value)) = line.split_once('=') {
                        if key.trim().to_lowercase().as_str() == "cleanup" {
                            cleanup.extend(value.trim().chars());
                            cleanup.sort();
                            cleanup.dedup();

                            info!("eSpeak: loaded cleanup characters: {}", cleanup.iter().collect::<String>());
                            continue;
                        } else {
                            return Err(format!("eSpeak: failed to parse dictionary line [{line}]"));
                        }
                    }

                    if let Some((key, replacer)) = line.split_once(' ') {
                        let key = key.trim().to_string();
                        let processed_replacer = Self::process_replacement_word(replacer.trim());
                        debug!("> espeak replacer: {key} -> {}", processed_replacer.replace('_', "-"));
                        self.dict.insert(key, processed_replacer);
                    }
                }
                info!(
                    "eSpeak: loaded {} custom dictionary string(s).",
                    self.dict.len()
                );
            }
        }

        self.cleanup = cleanup;

        // workaround to match some input language ids to espeak language codes
        let mapping = vec![
                ("br", "pt"),
                ("cz", "cs"),
                ("esmx", "es-la"),
                ("kr", "ko")
            ]
            .drain(..)
            .collect::<HashMap<_, _>>();

        let voice = match mapping.get(language) {
            Some(remapped) => {
                debug!("> espeak: remapping language {language} to {remapped}");
                CString::new(*remapped).unwrap()
            }
            None => CString::new(language).unwrap(),
        };


        let result = unsafe { bindings::espeak_SetVoiceByName(voice.as_ptr()) };

        match result {
            espeak_ERROR::EE_OK => Ok(()),
            _ => Err(format!("eSpeak: failed to set language [{}]", language)),
        }
    }
    // ------------------------------------------------------------------------
    pub fn convert_to_phonemes(&self, text: &str, use_separator: bool) -> Result<String, String> {
        let input = text
            .to_lowercase()
            .replace(self.cleanup.as_slice(), " ");

        let s = CString::new(input).unwrap();

        #[cfg(windows)]
        let phonememode = {
            // previous versions (precompiled windows lib)
            // phonememode bits0-3
            //      0= just phonemes.
            //      1= include ties (U+361) for phoneme names of more than one letter.
            //      2= include zero-width-joiner for phoneme names of more than one letter.
            //      3= separate phonemes with underscore characters.
            //      4= eSpeak's ascii phoneme names.
            //      5= International Phonetic Alphabet (as UTF-8 characters).
            #[allow(clippy::match_bool)]
            match use_separator {
                true => 0b0001_1000 as c_int,
                false => 0b0001_0000 as c_int,
            }
        };

        #[cfg(not(windows))]
        let phonememode = {
            // newer espeak version (Revision 10 29.Aug.2014) has different parameter!
            // phoneme_mode
            //   bit 1:   0=eSpeak's ascii phoneme names, 1= International Phonetic Alphabet (as UTF-8 characters).
            //   bit 7:   use (bits 8-23) as a tie within multi-letter phonemes names
            //   bits 8-23:  separator character, between phoneme names
            if use_separator {
                0b0000_0010 | 0b1000_0000 | (0b0101_1111 << 8)
            } else {
                0b0000_0010
            }
        };
        unsafe {
            let mut text_ptr = s.as_ptr() as *const libc::c_void;
            let mut phonemes_result = String::new();

            let mut err = None;

            // text_ptr: The address of a pointer to the input text
            // which is terminated by a zero character. On return from
            // espeak_TextToPhonemes the pointer has been advanced past
            // the text which has been translated, or else set to NULL
            // to indicate that the end of the text has been reached.
            while !text_ptr.is_null() {
                // returns a pointer to a character string which contains
                // the phonemes for the text up to end of a sentence, or
                // comma, semicolon, colon, or similar punctuation.

                // ownership of phonemes result stays in lib!
                let phonemes: *const c_char = bindings::espeak_TextToPhonemes(
                    &mut text_ptr, // *mut *const c_void
                    espeakCHARS_UTF8 as c_int,
                    phonememode,
                );

                match CStr::from_ptr(phonemes).to_str() {
                    Ok(string) => phonemes_result.push_str(string),
                    Err(why) => {
                        err = Some(format!("eSpeak: failed to convert text to phonemes: {why}"));
                        break;
                    }
                }
            }

            #[cfg(not(windows))]
            {
                // workaround for different separators in latest espeak version
                let mut phoneme_char = phonemes_result.chars().peekable();
                let mut remapped_phonemes = String::default();

                while let Some(c) = phoneme_char.next() {
                    match c {
                        '_' => {}
                        ' ' | 'ˈ' | 'ˌ' => remapped_phonemes.push(c),
                        _ => match phoneme_char.peek() {
                            Some(m) if ['_', ' ', 'ː', '-', '"', '^'].contains(m) => {
                                remapped_phonemes.push(c)
                            }
                            _ => remapped_phonemes.push_str(&format!("{c}_")),
                        },
                    }
                }
                phonemes_result = remapped_phonemes;
            }
            // Quick hack: remove any mixed lang espeak tags like:
            // (<lang1>)...(<alng2>) ...
            let phonemes_result = &*REGEXP_LANG.replace_all(&phonemes_result, "");

            // merge some modifier into prev phoneme to make it easier to handle
            let mut filtered = String::with_capacity(phonemes_result.len());
            let mut phoneme_char = phonemes_result.chars().peekable();
            while let Some(c) = phoneme_char.next() {
                match (c, phoneme_char.peek()) {
                    ('_', Some(&'̃')) | ('_', Some(&'̩')) | ('_', Some(&'̝')) | ('_', Some(&'ʲ')) => {}
                    _ => {
                        filtered.push(c);
                    }
                }
            }

            match err {
                Some(err) => Err(err),
                None => Ok(filtered),
            }
        }
        .map(|translated| {
            // check if custom dictionary should replace a word
            if !self.dict.is_empty() {
                translated
                    .split(' ')
                    .map(|word| match self.dict.get(&word.replace('_', "")) {
                        Some(substitution) => substitution,
                        None => word,
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                translated
            }
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Drop for ESpeak {
    fn drop(&mut self) {
        unsafe {
            bindings::espeak_Terminate();
        }
    }
}
// ----------------------------------------------------------------------------
use regex::Regex;

lazy_static! {
    static ref REGEXP_LANG: Regex = Regex::new("_?\\([^()]+\\)_?").unwrap();
}
// ----------------------------------------------------------------------------
