//
// gui: settings for editor
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
#[derive(PartialEq, Debug, Default)]
pub(super) enum PhonemeDragMode {
    None,
    Neighbour,
    #[default]
    Words,
}
// ----------------------------------------------------------------------------
pub(super) struct Settings {
    drag_mode: PhonemeDragMode,
    drag_damping: f32,
    granularity_ms: f32,
    language: Languages,
    actors: Actors,
}
// ----------------------------------------------------------------------------
pub(super) struct Languages {
    selected: String,
    available: Vec<String>,
}
// ----------------------------------------------------------------------------
pub(super) struct Actors {
    selected: usize,
    available: Vec<(imgui::ImString, String)>,
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
extern crate glob;

use std::path::Path;
use std::sync::Arc;

use super::actors::ActorMapping;
// ----------------------------------------------------------------------------
impl Default for Settings {
    fn default() -> Settings {
        Settings {
            drag_mode: PhonemeDragMode::default(),
            drag_damping: 0.5,
            granularity_ms: 5.0,
            language: Languages::default(),
            actors: Actors::default(),
        }
    }
}
// ----------------------------------------------------------------------------
impl Default for Languages {
    fn default() -> Self {
        Self {
            selected: "en".into(),
            available: vec!["en".into()],
        }
    }
}
// ----------------------------------------------------------------------------
impl Languages {
    // ------------------------------------------------------------------------
    pub fn init_from_path(path: &Path) -> Result<Self, String> {
        use self::glob::glob;

        let mut available = Vec::new();
        let scan_path = format!("{}/*.cfg", path.display());
        for entry in
            (glob(&scan_path).map_err(|e| format!("failed to search cfg files: {e}"))?).flatten()
        {
            if entry.is_file() {
                if let Some(file) = entry
                    .as_path()
                    .file_stem()
                    .map(|name| name.to_string_lossy())
                {
                    if let Some((lang, _)) = file.split_once('.') {
                        if lang.len() == 2 {
                            info!("found config for language {lang}");
                            available.push(lang.to_lowercase());
                        }
                    }
                }
            }
        }

        if available.is_empty() {
            Err(format!(
                "no supported language config found in datadir: {}!",
                path.display()
            ))
        } else {
            Ok(Self {
                selected: available.first().unwrap().to_owned(),
                available,
            })
        }
    }
    // ------------------------------------------------------------------------
    pub fn set(&mut self, lang: &str) -> &str {
        if self.available.iter().any(|l| l == lang) {
            self.selected = lang.into();
        } else {
            warn!(
                "ignoring change to unsupported lang {lang}. supported: {}",
                self.available.join(", ")
            );
        }
        &self.selected
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Default for Actors {
    fn default() -> Self {
        Self {
            selected: 0,
            available: vec![(imgui::ImString::new("default"), "default".into())],
        }
    }
}
// ----------------------------------------------------------------------------
impl Actors {
    // ------------------------------------------------------------------------
    pub fn init_from(&mut self, mapping: Arc<ActorMapping>) {
        self.available = vec![(imgui::ImString::new("default"), "default".into())];

        self.available.extend(
            mapping
                .available()
                .drain(..)
                .map(|(id, caption)| (imgui::ImString::new(caption), id)),
        );
        self.selected = 0;
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn available(&self) -> &[(imgui::ImString, String)] {
        &self.available
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn selected(&self) -> Option<&(imgui::ImString, String)> {
        self.available.get(self.selected)
    }
    // ------------------------------------------------------------------------
    pub fn set(&mut self, actor: &str) -> Option<&str> {
        // try also to match on caption as fallback
        let cap_actor = super::actors::capitalize(actor);

        if let Some((i, (_caption, _id))) = self
            .available
            .iter()
            .enumerate()
            .find(|(_i, (caption, id))| id == actor || caption.to_str() == cap_actor)
        {
            self.selected = i;
        } else {
            if !actor.is_empty() {
                warn!(
                    "ignoring change to unknown actor '{actor}'. setting to default. supported: {}",
                    self.available
                        .iter()
                        .map(|(caption, _)| caption.to_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            self.selected = 0;
        }
        self.available.get(self.selected).map(|(_, id)| id.as_str())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Settings {
    // ------------------------------------------------------------------------
    pub fn detect_language_support(&mut self, datadir: &Path) -> Result<(), String> {
        self.language = Languages::init_from_path(datadir)?;
        Ok(())
    }
    // ------------------------------------------------------------------------
    pub fn reset_actor_mappings(&mut self, mappings: Arc<ActorMapping>) {
        self.actors.init_from(mappings);
    }
    // ------------------------------------------------------------------------
    #[allow(dead_code)]
    #[inline]
    pub fn available_languages(&self) -> &[String] {
        &self.language.available
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn selected_language(&self) -> &str {
        &self.language.selected
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn available_actors(&self) -> &[(imgui::ImString, String)] {
        self.actors.available()
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn selected_actor(&self) -> Option<&(imgui::ImString, String)> {
        self.actors.selected()
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn drag_mode(&self) -> &PhonemeDragMode {
        &self.drag_mode
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn drag_damping(&self) -> f32 {
        self.drag_damping
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn granularity_ms(&self) -> f32 {
        self.granularity_ms
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_drag_mode(&mut self, new_mode: PhonemeDragMode) {
        self.drag_mode = new_mode;
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_language(&mut self, new_lang: &str) -> &str {
        self.language.set(new_lang)
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_actor(&mut self, actor: &str) -> Option<&str> {
        self.actors.set(actor)
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
