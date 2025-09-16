// ----------------------------------------------------------------------------
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct ActorMapping {
    path: Option<PathBuf>,
    mapping: Mutex<crate::actors::ActorMapping>,
}
// ----------------------------------------------------------------------------
impl ActorMapping {
    // ------------------------------------------------------------------------
    pub fn load(datadir: &Path, additional: Option<&Path>) -> Result<Self, String> {
        crate::actors::ActorMapping::load(datadir, additional).map(|mapping| Self {
            path: additional.map(|p| p.to_path_buf()),
            mapping: mapping.into(),
        })
    }
    // ------------------------------------------------------------------------
    pub fn changed(&self) -> bool {
        self.path.is_some()
            && match self.mapping.lock() {
                Ok(mapping) => mapping.changed(),
                Err(e) => {
                    error!("could not acquire lock on actor mapping: {e}");
                    // better save then sorry
                    true
                }
            }
    }
    // ------------------------------------------------------------------------
    pub fn available(&self) -> Vec<(String, String)> {
        match self.mapping.lock() {
            Ok(mapping) => mapping
                .available()
                .map(|(id, caption)| (id.to_owned(), capitalize(caption)))
                .collect::<Vec<_>>(),
            Err(e) => {
                error!("could not acquire lock on actor mapping: {e}");
                Vec::default()
            }
        }
    }
    // ------------------------------------------------------------------------
    pub fn resolve(&self, actor: &str) -> String {
        match self.mapping.lock() {
            Ok(mut mapping) => mapping.resolve(actor),
            Err(e) => {
                error!("could not acquire lock on actor mapping: {e}");
                actor.to_string()
            }
        }
    }
    // ------------------------------------------------------------------------
    pub fn update(&self, actor: &str, mapped_to: &str) {
        match self.mapping.lock() {
            Ok(mut mapping) => mapping.update(actor, mapped_to),
            Err(e) => {
                error!("could not acquire lock on actor mapping: {e}");
            }
        }
    }
    // ------------------------------------------------------------------------
    pub fn store_updated(&self) -> Result<(), String> {
        if let Some(path) = self.path.as_ref() {
            match self.mapping.lock() {
                Ok(mut mapping) => mapping.store_updated(path),
                Err(e) => {
                    error!("could not acquire lock on actor mapping: {e}");
                    Err(format!("could not acquire lock on actor mapping: {e}"))
                }
            }
        } else {
            Ok(())
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
pub(super) fn capitalize(s: &str) -> String {
    let result = s
        .trim()
        .split(' ')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    result
}
// ----------------------------------------------------------------------------
