//
// text provider
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub trait StringsProvider {
    // ------------------------------------------------------------------------
    fn get_lang(&self) -> &String;
    // ------------------------------------------------------------------------
    fn get_line(&self, id: u32) -> Result<&String, String>;
    // ------------------------------------------------------------------------
    fn get_actor(&self, id: u32) -> Option<&String>;
    // ------------------------------------------------------------------------
    fn get_all_lines(&self) -> &BTreeMap<u32, (String, Option<String>)>;
    // ------------------------------------------------------------------------
    fn get_all_lines_lowercased(&self) -> &Vec<(u32, String)>;
    // ------------------------------------------------------------------------
    fn line_count(&self) -> usize;
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
pub trait CsvLoader<T> {
    // ------------------------------------------------------------------------
    fn load(file: &Path) -> Result<T, String>;
    // ------------------------------------------------------------------------
    fn create_reader(filepath: &Path) -> Result<BufReader<File>, String> {
        debug!("opening {}...", filepath.display());

        let filepath = filepath
            .to_str()
            .ok_or_else(|| String::from("path to string conversion failed"))?;

        File::open(filepath)
            .map(BufReader::new)
            .map_err(|e| format!("couldn't open {}: {}", filepath, e))
    }
    // ------------------------------------------------------------------------
    fn parse_meta(line: &str) -> Result<(&str, &str), String> {
        if line.starts_with(";meta[") && line.ends_with(']') {
            let s = &line[6..line.len() - 1];
            match s.find('=') {
                Some(pos) => Ok((&s[0..pos], &s[pos + 1..])),
                None => Err(String::from("invalid meta format")),
            }
        } else {
            Err(String::from("line does not contain any meta data."))
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
pub trait CsvStringsLoader<T>: CsvLoader<T> {
    // ------------------------------------------------------------------------
    fn load_with_language(file: &Path, lang: Option<&str>) -> Result<T, String>;
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
pub trait CsvWriter {
    // ------------------------------------------------------------------------
    fn writeln(&mut self, line: &str);
    // ------------------------------------------------------------------------
    fn write_meta(&mut self, key: &str, value: &str) {
        self.writeln(&format!(";meta[{}={}]", key, value));
    }
    // ------------------------------------------------------------------------
    fn write_header(&mut self, line: &str) {
        self.writeln(&format!(";{}", line));
    }
    // ------------------------------------------------------------------------
    fn write_comment(&mut self, line: &str) {
        self.writeln(&format!(";{}", line));
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
pub struct CsvStringsData {
    lang: String,
    lines: BTreeMap<u32, (String, Option<String>)>,
    lines_lowercased: Vec<(u32, String)>,
}
// ----------------------------------------------------------------------------
pub struct SimpleCsvWriter {
    file: BufWriter<File>,
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

// ----------------------------------------------------------------------------
impl StringsProvider for CsvStringsData {
    // ------------------------------------------------------------------------
    fn get_lang(&self) -> &String {
        &self.lang
    }
    // ------------------------------------------------------------------------
    fn get_line(&self, id: u32) -> Result<&String, String> {
        match self.lines.get(&id) {
            Some((text, _actor)) => Ok(text),
            None => Err(format!("string for id {} not found!", id)),
        }
    }
    // ------------------------------------------------------------------------
    fn get_actor(&self, id: u32) -> Option<&String> {
        self.lines.get(&id).and_then(|(_, actor)| actor.as_ref())
    }
    // ------------------------------------------------------------------------
    fn get_all_lines(&self) -> &BTreeMap<u32, (String, Option<String>)> {
        &self.lines
    }
    // ------------------------------------------------------------------------
    fn get_all_lines_lowercased(&self) -> &Vec<(u32, String)> {
        &self.lines_lowercased
    }
    // ------------------------------------------------------------------------
    fn line_count(&self) -> usize {
        self.lines.len()
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl CsvStringsData {
    // ------------------------------------------------------------------------
    pub fn preprocess_lowercased(&mut self) {
        self.lines_lowercased = Vec::with_capacity(self.lines.len());

        for (id, (text, _actor)) in &self.lines {
            self.lines_lowercased.push((*id, text.to_lowercase()));
        }
    }
    // ------------------------------------------------------------------------
    fn extract_textline(
        column_separator: char,
        id_col: usize,
        text_col: usize,
        actor_col: Option<usize>,
        textline: &str,
    ) -> Result<(u32, String, Option<String>), String> {
        let mut new_textline = String::new();
        let cols: Vec<&str> = if column_separator == ';' {
            // the ; character may be present in the textline. make sure to split
            // the columnes correctly
            let mut iter = textline.chars().peekable();
            let mut quoted_col = false;
            while let Some(c) = iter.next() {
                if quoted_col {
                    new_textline.push(c);
                    if c == '"' && iter.peek() == Some(&';') {
                        quoted_col = false;
                    }
                } else if c == ';' {
                    new_textline.push('|');

                    if iter.peek() == Some(&'"') {
                        quoted_col = true;
                    }
                } else {
                    new_textline.push(c);
                }
            }
            new_textline.split('|').collect()
        } else {
            textline.split(column_separator).collect()
        };

        let min_cols = id_col.max(text_col) + 1;

        if cols.len() < min_cols {
            return Err(format!(
                "at least {min_cols} columns required. found: {}",
                cols.len()
            ));
        }

        // id col is <=10 digit u32
        let id: u32 = match cols[id_col].trim().parse() {
            Ok(id) => id,
            Err(why) => return Err(format!("could not parse id [{}]: {}", cols[id_col], why)),
        };
        // column is text
        let text = cols[text_col]
            .replace("\"\"", "|")
            .trim_matches(|c: char| c.is_whitespace() || c == '"')
            .replace('|', "\"");

        fn filter_voiceover(voiceover: &str) -> String {
            // assumed format: <actor>_<something>_<id>
            voiceover
                .rsplitn(3, '_')
                .collect::<Vec<_>>()
                .last()
                .map(|v| v.trim().to_uppercase())
                .unwrap_or_default()
        }

        let actor = actor_col
            .and_then(|actor| cols.get(actor))
            .map(|actor| if column_separator == ';' {
                filter_voiceover(actor)
            } else {
                actor.trim().to_uppercase()
            })
            .filter(|actor| !actor.is_empty());

        Ok((id, text, actor))
    }
    // ------------------------------------------------------------------------
    fn extract_language(textline: &str) -> Option<String> {
        let prefix = ";meta[language=";

        if textline.starts_with(prefix) && textline.ends_with(']') {
            let value = &textline[prefix.len()..textline.len() - 1];
            Some(String::from(value))
        } else {
            None
        }
    }
    // ------------------------------------------------------------------------
    fn extract_columns(textline: &str) -> Result<(char, usize, usize, Option<usize>), String> {
        let col_separator = '|';
        let cols: Vec<_> = if let Some(textline) = textline.strip_prefix(';') {
            textline.split(col_separator).map(|s| s.trim()).collect()
        } else {
            textline.split(col_separator).map(|s| s.trim()).collect()
        };

        let id_col = cols.iter().enumerate().find(|(_, col)| **col == "id").map(|(i, _)| i)
            .ok_or_else(|| String::from("failed to find 'id' column"))?;

        let text_col = cols.iter().enumerate().find(|(_, col)| **col == "text").map(|(i, _)| i)
            .ok_or_else(|| String::from("failed to find 'text' column"))?;

        let actor_col = cols.iter().enumerate().find(|(_, col)| **col == "actor").map(|(i, _)| i);

        Ok((col_separator, id_col, text_col, actor_col))
    }
    // ------------------------------------------------------------------------
    fn extract_redkit_columns(lang: &str, textline: &str) -> Result<(char, usize, usize, Option<usize>), String> {
        let col_separator = ';';
        let cols: Vec<_> = textline.split(col_separator).map(|s| s.trim().to_lowercase()).collect();

        let id_col = cols.iter().enumerate().find(|(_, col)| **col == "id").map(|(i, _)| i)
            .ok_or_else(|| String::from("failed to find 'id' column"))?;

        let text_col = cols.iter().enumerate().find(|(_, col)| **col == lang).map(|(i, _)| i)
            .ok_or_else(|| format!("failed to find '{lang}' column for text extraction"))?;

        let actor_col = cols.iter().enumerate().find(|(_, col)| **col == "voiceover").map(|(i, _)| i);

        Ok((col_separator, id_col, text_col, actor_col))
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl CsvStringsLoader<CsvStringsData> for CsvStringsData {
    // ------------------------------------------------------------------------
    fn load_with_language(filepath: &Path, language: Option<&str>) -> Result<Self, String> {
        let reader = Self::create_reader(filepath)?;

        let mut data = CsvStringsData {
            lang: "".to_owned(),
            lines: BTreeMap::new(),
            lines_lowercased: Vec::new(),
        };

        let mut lines = reader.lines().enumerate();

        let first_line = lines.next().map(|(_, line)| line)
            .ok_or_else(|| String::from("failed to read line 1"))?
            .map_err(|why| format!("failed to read line 1: {why}"))?;

        let (column_separator, id_col, text_col, actor_col) = match Self::extract_language(&first_line) {
            Some(lang) => {
                if let Some(expected_lang) = language {
                    if expected_lang != lang {
                        return Err(format!("expected languange [{expected_lang}] in file. found: {lang}"));
                    }
                }
                data.lang = lang;
                let col_line = lines.next().map(|(_, line)| line)
                    .ok_or_else(|| String::from("failed to read columns from line 2"))?
                    .map_err(|why| format!("failed to read line 2: {why}"))?;

                Self::extract_columns(&col_line)?
            }
            None => {
                // provided language is used as column name
                data.lang = language.unwrap_or("en").to_lowercase();

                Self::extract_redkit_columns(&data.lang, &first_line)?
            }
        };

        for (line, text) in lines {
            let (id, text, actor) = match text {
                Ok(text) if text.starts_with(';') => continue,

                Ok(text) => Self::extract_textline(column_separator, id_col, text_col, actor_col, &text)
                    .map_err(|why| format!("error reading line {}: {why}", line + 1))?,

                // match Self::extract_textline(id_col, text_col, actor_col, &text) {
                //     Ok(extracted_data) => extracted_data,
                //     Err(why) => return Err(format!("error reading line {}: {}", line + 1, &why)),
                // },

                Err(why) => {
                    return Err(format!("error reading line {}: {why}", line + 1))
                }
            };
            data.lines.insert(id, (text, actor));
        }

        info!("loaded {} strings", data.lines.len());

        Ok(data)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl CsvLoader<CsvStringsData> for CsvStringsData {
    // ------------------------------------------------------------------------
    fn load(filepath: &Path) -> Result<CsvStringsData, String> {
        CsvStringsData::load_with_language(filepath, None)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl SimpleCsvWriter {
    // ------------------------------------------------------------------------
    pub fn create(path: &PathBuf) -> Result<SimpleCsvWriter, String> {
        trace!("creating {}...", path.display());

        let file = match File::create(path) {
            Ok(file) => file,
            Err(why) => return Err(format!("{}", &why)),
        };

        Ok(SimpleCsvWriter {
            file: BufWriter::new(file),
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl CsvWriter for SimpleCsvWriter {
    // ------------------------------------------------------------------------
    fn writeln(&mut self, line: &str) {
        self.file.write_all(line.as_bytes()).unwrap();
        self.file.write_all(b"\n").unwrap();
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
