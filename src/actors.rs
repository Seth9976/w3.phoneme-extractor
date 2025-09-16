// ----------------------------------------------------------------------------
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use indexmap::IndexMap;

#[derive(Default)]
pub struct ActorMapping {
    mappings: ActorMappingConfig,
    additional: ActorMappingConfig,
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct ActorMappingConfig {
    changed: bool,
    comments: Vec<String>,
    mappings: IndexMap<String, String>,
}
// ----------------------------------------------------------------------------
impl ActorMapping {
    // ------------------------------------------------------------------------
    pub fn load(datadir: &Path, additional: Option<&Path>) -> Result<Self, String> {
        let filepath = datadir.join("actor.captions.cfg");

        let mappings = if filepath.is_file() {
            info!("loading actor mappings file {}", filepath.display());
            let mappings = ActorMappingConfig::load_csv(&filepath)?;
            info!("loaded {} actor mappings", mappings.mappings.len());
            mappings
        } else {
            warn!("did not find actor mappings config: {}", filepath.display());
            ActorMappingConfig::default()
        };

        let additional = if let Some(additional) = additional {
            info!(
                "loading additional actor mappings file {}",
                additional.display()
            );
            let mappings = ActorMappingConfig::load_csv(additional)?;
            info!("loaded {} actor mappings", mappings.len());
            mappings
        } else {
            ActorMappingConfig::default()
        };

        Ok(Self {
            mappings,
            additional,
        })
    }
    // ------------------------------------------------------------------------
    pub fn changed(&self) -> bool {
        self.additional.changed
    }
    // ------------------------------------------------------------------------
    pub fn available(&self) -> impl Iterator<Item = (&String, &String)> {
        self.mappings.mappings.iter()
    }
    // ------------------------------------------------------------------------
    /// resolves mapping for provided actor id and collects the mapping in an
    /// extra list resembling mappings for all processed actors. function
    /// tries to match actor on cpation or id in the general config id-captions
    /// pairs. if matched a match is found its id is returned otherwise the
    /// original actor id is returned unchanged. in any case a mapping from
    /// original id to result is added to additional mapping list.
    /// the unique set of collected actor mappings can be stored afterwrads.
    /// Note: if additional config was provided its mappings are ALWAYS
    /// priorized over the general config (no actor caption mapping is attempted).
    pub fn resolve(&mut self, actor: &str) -> String {
        let actor_lc = actor.to_lowercase();
        // prioritize additional so it is possible to overwrite general config
        if let Some(value) = self.additional.get(&actor_lc) {
            value.to_string()
        } else {
            let result = if let Some(value) = self.mappings.match_fuzzy(&actor_lc) {
                value.to_string()
            } else {
                actor.to_string()
            };

            self.additional.add(actor, &result);

            result
        }
    }
    // ------------------------------------------------------------------------
    pub fn update(&mut self, actor: &str, mapping: &str) {
        self.additional.update(actor, mapping);
    }
    // ------------------------------------------------------------------------
    pub fn store_updated(&mut self, path: &Path) -> Result<(), String> {
        if self.additional.changed {
            self.additional.store_updated(path)
        } else {
            Ok(())
        }
    }
    // ------------------------------------------------------------------------
    pub fn create_new(path: &Path) -> Result<(), String> {
        ActorMappingConfig::default().store(path)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ActorMappingConfig {
    // ------------------------------------------------------------------------
    fn len(&self) -> usize {
        self.mappings.len()
    }
    // ------------------------------------------------------------------------
    fn get<'a>(&'a self, actor: &str) -> Option<&'a str> {
        self.mappings.get(actor).map(|x| x.as_str())
    }
    // ------------------------------------------------------------------------
    // tries to match on caption or id. returns id of the mapping
    fn match_fuzzy<'a>(&'a self, actor: &str) -> Option<&'a str> {
        self.mappings
            .iter()
            .find(|(id, caption)| caption.as_str() == actor || *id == actor)
            .map(|(id, _)| id.as_str())
    }
    // ------------------------------------------------------------------------
    fn add(&mut self, actor: &str, mapped_to: &str) {
        self.mappings
            .insert(actor.to_lowercase(), mapped_to.to_lowercase());
        self.changed = true;
    }
    // ------------------------------------------------------------------------
    fn update(&mut self, actor: &str, mapped_to: &str) {
        if let Some(value) = self.mappings.get_mut(actor) {
            if value != mapped_to {
                self.changed = true;
                *value = mapped_to.to_string();
            }
        } else {
            self.changed = true;
            self.mappings
                .insert(actor.to_string(), mapped_to.to_string());
        }
    }
    // ------------------------------------------------------------------------
    fn load_csv(path: &Path) -> Result<Self, String> {
        let filepath = path
            .to_str()
            .ok_or_else(|| String::from("path to string conversion failed"))?;

        let file = File::open(filepath)
            .map(BufReader::new)
            .map_err(|e| format!("couldn't open {}: {}", filepath, e))?;

        let mut comments = Vec::default();
        let mut mappings = IndexMap::default();

        for (i, line) in file.lines().enumerate() {
            let original_line =
                line.map_err(|err| format!("failed to read line {}: {err}", i + 1))?;
            let line = original_line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with(';') {
                comments.push(line.to_string());
            } else {
                let cols: Vec<&str> = line.split(':').map(|col| col.trim()).collect();

                if cols.len() != 2 {
                    return Err(format!(
                        "actor mappings line {}: expected 2 columns. found {}",
                        i + 1,
                        cols.len()
                    ));
                }

                let key = cols[0].trim().to_lowercase();
                let value = cols[1].trim().to_lowercase();

                if mappings.insert(key.clone(), value.clone()).is_some() {
                    warn!("found duplicate mapping for {}", cols[0]);
                }
            }
        }
        Ok(Self {
            changed: false,
            comments,
            mappings,
        })
    }
    // ------------------------------------------------------------------------
    fn store_updated(&mut self, path: &Path) -> Result<(), String> {
        if self.changed {
            self.store(path)
        } else {
            Ok(())
        }
    }
    // ------------------------------------------------------------------------
    fn store(&mut self, path: &Path) -> Result<(), String> {
        use std::io::{BufWriter, Write};

        let mut file = BufWriter::new(File::create(path).map_err(|err| format!("{err}"))?);

        if self.comments.is_empty() {
            self.comments
                .push("; mapping for actor names/ids\n;".to_string());
        }

        for comment in &self.comments {
            writeln!(file, "{comment}").map_err(|err| format!("{err}"))?;
        }

        let mut result = self
            .mappings
            .iter()
            .map(|(k, v)| format!("{k}:{v}"))
            .collect::<Vec<_>>();
        result.sort();

        for line in result {
            writeln!(file, "{line}").map_err(|err| format!("{err}"))?;
        }
        self.changed = false;
        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
