//
// gui::help
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(in gui) type HelpText = ImString;
// ----------------------------------------------------------------------------
#[derive(Hash, Eq, PartialEq, Debug)]
pub(in gui) enum HelpTopic {
    General(ImString),
}
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(in gui) struct HelpSystem {
    help: IndexMap<HelpTopic, HelpText>,
}
// ----------------------------------------------------------------------------
use indexmap::IndexMap;

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use std::convert::TryFrom;

use imgui::ImString;
// ----------------------------------------------------------------------------
impl HelpSystem {
    // ------------------------------------------------------------------------
    pub fn get(&self, topic: &HelpTopic) -> Option<&HelpText> {
        self.help.get(topic)
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn topics(&self) -> impl Iterator<Item = &HelpTopic> {
        self.help.keys()
    }
    // ------------------------------------------------------------------------
    pub fn load(&mut self, path: &Path) -> Result<(), String> {
        let file = File::open(path)
            .map_err(|err| format!("helpsystem: error reading help file: {}", err))?;

        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();

        buf_reader
            .read_to_string(&mut contents)
            .map_err(|err| format!("helpsystem: error reading help file: {}", err))?;

        self.help = Self::parse_help(&contents)?;

        Ok(())
    }
    // ------------------------------------------------------------------------
    fn parse_help(contents: &str) -> Result<IndexMap<HelpTopic, HelpText>, String> {
        let mut help = IndexMap::new();

        // most simple parser (ignore first empty string in front of first token)
        for part in contents.split("##").skip(1) {
            let (topic, content) = part.split_at(part.find('\n').unwrap_or(part.len()));

            match HelpTopic::try_from(topic) {
                Ok(topic_id) => {
                    let content = content.trim();
                    if content.is_empty() {
                        warn!(
                            "helpsystem: found empty content for topic [{}]. skipping..",
                            topic
                        );
                    } else {
                        help.insert(topic_id, ImString::new(content.to_owned()));
                    }
                }
                Err(err) => {
                    warn!("helpsystem: topic-id parse-error for [{}]: {}", topic, err);
                }
            }
        }
        Ok(help)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TryFrom<&str> for HelpTopic {
    type Error = String;
    // ------------------------------------------------------------------------
    fn try_from(id: &str) -> Result<Self, Self::Error> {
        use self::HelpTopic::*;

        match id.to_lowercase().as_str() {
            s if s.starts_with("general#") => Ok(General(ImString::new(id.split_at(8).1))),
            _ => Err(String::from("unknown topic")),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
