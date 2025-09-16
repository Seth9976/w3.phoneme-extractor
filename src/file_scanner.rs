//
// simple file scanner
//
extern crate glob;

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub struct FilesScanner {
    input_wildcard: String,
}
// ----------------------------------------------------------------------------
pub enum FileInfo {
    UnlinkedAudio(String),
    Audio(u32, String, Option<f32>),
    Phonemes(u32, String),
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::path::PathBuf;
use std::str::FromStr;

use self::glob::{glob, GlobResult};
// ----------------------------------------------------------------------------
impl FilesScanner {
    // ------------------------------------------------------------------------
    pub fn new(inputpath: PathBuf) -> Result<FilesScanner, String> {
        let input = inputpath
            .to_str()
            .ok_or("path to string conversion failed")?;

        let wildcard = if inputpath.is_dir() {
            format!("{}/*", input)
        } else {
            input.to_string()
        };

        Ok(FilesScanner {
            input_wildcard: wildcard,
        })
    }
    // ------------------------------------------------------------------------
    fn extract_path_components(path: GlobResult) -> Option<(String, String)> {
        match path {
            Ok(ref path) if path.is_file() => {
                let filepath = path.to_str().map(ToOwned::to_owned);
                let filename = path.file_name().and_then(|n| n.to_str().map(String::from));
                if let (Some(filename), Some(filepath)) = (filename, filepath) {
                    Some((filename, filepath))
                } else {
                    error!(
                        "scanner: failed to extract filename from: {}",
                        path.display()
                    );
                    None
                }
            }
            Ok(_) => None,
            Err(why) => {
                error!("scanner: {}", why);
                None
            }
        }
    }
    // ------------------------------------------------------------------------
    fn extract_duration_from_filename(filename: &str) -> Option<f32> {
        // duration is enclosed in []
        let duration = filename
            .chars()
            .skip_while(|c| c != &'[')
            // also skip the starting bracket [
            .skip(1)
            .take_while(|c| c != &']')
            .collect::<String>();

        if duration.is_empty() {
            None
        } else {
            f32::from_str(&duration).ok()
        }
    }
    // ------------------------------------------------------------------------
    fn extract_metainfo(filename: &str) -> Option<(u32, Option<f32>)> {
        if let Some((filename, _ext)) = filename.rsplit_once('.') {
            // first (max 10) chars must be the id digits

            let idprefix = filename
                .chars()
                .take(10)
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>();

            if !idprefix.is_empty() {
                if let Ok(id) = u32::from_str(&idprefix) {
                    return Some((id, Self::extract_duration_from_filename(filename)));
                }
            }

            // fallback (max 10) last chars are digits
            let reversed_idsuffix = filename
                .chars()
                .rev()
                .take(10)
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>();

            if !reversed_idsuffix.is_empty() {
                let idsuffix = reversed_idsuffix.chars().rev().collect::<String>();
                if let Ok(id) = u32::from_str(&idsuffix) {
                    return Some((id, Self::extract_duration_from_filename(filename)));
                }
            }

            if let Ok(id) = u32::from_str(filename) {
                return Some((id, Self::extract_duration_from_filename(filename)));
            }
        }
        None
    }
    // ------------------------------------------------------------------------
    pub fn scan(&mut self) -> Result<Vec<FileInfo>, String> {
        info!("scanning for files [{}]", &self.input_wildcard);

        let mut files = Vec::new();

        for entry in glob(&self.input_wildcard).map_err(|e| format!("{}", e))? {
            if let Some((filename, filepath)) = Self::extract_path_components(entry) {
                let is_audiofile = filename.ends_with(".wav") || filename.ends_with(".ogg");
                match Self::extract_metainfo(&filename) {
                    Some((id, duration)) => {
                        if is_audiofile {
                            debug!("found audio file: {} [id: {}]", filepath, id);

                            files.push(FileInfo::Audio(id, filepath, duration));
                        } else if filename.ends_with(".phonemes") {
                            debug!("found phoneme file: {} [id: {}]", filepath, id);

                            files.push(FileInfo::Phonemes(id, filepath));
                        } else {
                            #[cfg(debug_assertions)]
                            trace!(
                                "> scanner: ignoring file {} (unsupported extension)",
                                filepath
                            );
                        }
                    }
                    None => {
                        if is_audiofile {
                            debug!("found audio file without id: {}", filepath);

                            files.push(FileInfo::UnlinkedAudio(filepath));
                        } else {
                            #[cfg(debug_assertions)]
                            trace!(
                                "> scanner: ignoring file {} (unsupported extension)",
                                filepath
                            );
                        }
                    }
                }
            }
        }

        debug!("found #{} files.", files.len());
        Ok(files)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
