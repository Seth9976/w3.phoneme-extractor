//
// interactive line id assignment
//

// ----------------------------------------------------------------------------
// state
// ----------------------------------------------------------------------------
pub(super) struct IdSelectorState {
    opened: bool,
    selection: Option<SelectedAudio>,
    strings_provider: Option<CsvStringsData>,
    search_filter: input::TextField,
    search_result: Vec<(u32, ImString)>,
    search_capped: bool,
    selected_slot: i32,
    selected_text: Option<ImString>,
}
// ----------------------------------------------------------------------------
// actions
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub(super) enum Action {
    Cancel,
    ToggleAudioplayback,
    OnSearchFilterChanged(String),
    OnSelectLine(i32),
    OnAssignId(u32),
}
// ----------------------------------------------------------------------------
// view
// ----------------------------------------------------------------------------
pub(in gui) mod cmds;
pub(in gui) mod view;
// ----------------------------------------------------------------------------
// action processing
// ----------------------------------------------------------------------------
#[inline]
pub(super) fn handle_action(
    action: Action,
    state: &mut IdSelectorState,
    player: &mut player::Player,
) -> Result<Option<ActionSequence>, String> {
    let result = match action {
        OnSearchFilterChanged(new_value) => {
            cmds::refresh_search_result(state, new_value)?;
            None
        }
        OnSelectLine(selected_slot) => {
            handle_line_selection(selected_slot, state);
            None
        }
        OnAssignId(new_lineid) => {
            if let Some(selection) = state.selection.take() {
                Some(ima_seq![
                    // since the selector is a modal dialog it has to be closed first
                    // otherwise no error msg can be shown!
                    GuiAction::from(Cancel),
                    GuiAction::AssignId(create_id_assignement(
                        state.strings_provider.as_ref(),
                        new_lineid,
                        selection
                    )?)
                ])
            } else {
                None
            }
        }
        ToggleAudioplayback => {
            if let Ok(playback) = player.state() {
                let action = match playback {
                    player::PlaybackState::Idle => GuiAction::PlayAudio,
                    player::PlaybackState::Playing => GuiAction::StopAudio,
                };
                Some(ima_seq![action])
            } else {
                None
            }
        }
        Cancel => {
            state.close_selector();
            Some(ima_seq![GuiAction::StopAudio])
        }
    };
    Ok(result)
}
// ----------------------------------------------------------------------------
pub(super) fn handle_hotkey(hotkey: Hotkey, state: &mut IdSelectorState) -> Option<Action> {
    match hotkey {
        Hotkey::Space if !state.search_filter.changed() => {
            // not pressed while editing search filter textfield
            Some(ToggleAudioplayback)
        }
        _ => None,
    }
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
// placement of notice is aligned for 3 digit max
const MAX_SEARCH_RESULTS: usize = 100;

// max characters for usage in audiofile renaming
const MAX_TEXTHINT_CHARS: usize = 50;

const SEARCHFILTER_INPUT_WIDTH: f32 = 300.0;
const SEARCHFILTER_LABEL: &str = "search filter:";
// ----------------------------------------------------------------------------
use std::path::Path;

use regex::Regex;

use imgui::ImString;
use imgui_controls::input;

use imgui_controls::input::Field;

use super::hotkeys::Hotkey;
use super::player;
use super::queue::{Selection, SelectionId};

use {CsvStringsData, CsvStringsLoader, StringsProvider};

// actions
use gui::Action as GuiAction;

use super::{ActionSequence, IdAssignmentActionData};

use self::Action::*;
// ----------------------------------------------------------------------------
#[inline]
fn handle_line_selection(selected_slot: i32, state: &mut IdSelectorState) {
    state.selected_slot = selected_slot;
    state.selected_text = if selected_slot >= 0 {
        state
            .search_result
            .get(selected_slot as usize)
            .map(|(id, _)| {
                let text = if let Some(strings_provider) = &state.strings_provider {
                    strings_provider
                        .get_line(*id)
                        .map(String::as_str)
                        .unwrap_or("text not found")
                } else {
                    "missing strings provider!"
                };
                ImString::new(text)
            })
    } else {
        None
    };
}
// ----------------------------------------------------------------------------
struct SelectedAudio {
    id: SelectionId,
    duration: f32,
    audiofile: ImString,
}
// ----------------------------------------------------------------------------
impl IdSelectorState {
    // ------------------------------------------------------------------------
    pub(super) fn new() -> IdSelectorState {
        IdSelectorState {
            opened: false,
            selection: None,
            strings_provider: None,
            search_filter: input::TextField::new("##searchfilter", None::<String>)
                .set_label("search filter:")
                .set_width(SEARCHFILTER_INPUT_WIDTH),
            search_result: Vec::default(),
            search_capped: false,
            selected_slot: -1,
            selected_text: None,
        }
    }
    // ------------------------------------------------------------------------
    pub(super) fn reset(&mut self) {
        self.opened = false;
        self.selection = None;
        self.strings_provider = None;
        self.search_result = Vec::default();
        self.search_capped = false;
        self.selected_slot = -1;
        self.selected_text = None;
    }
    // ------------------------------------------------------------------------
    fn close_selector(&mut self) {
        self.opened = false;
        self.selection = None;
        // strings provider does not need to be resetted
        self.search_filter.reset();
        self.search_result = Vec::default();
        self.search_capped = false;
        self.selected_slot = -1;
        self.selected_text = None;
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn is_opened(&self) -> bool {
        self.opened
    }
    // ------------------------------------------------------------------------
    pub(super) fn init_strings_provider(
        &mut self,
        stringsfile: &Path,
        language: Option<&str>,
    ) -> Result<(), String> {
        let mut provider =
            CsvStringsData::load_with_language(stringsfile, language).map_err(|err| {
                format!(
                    "could not create string provider from \"{}\": {}.",
                    stringsfile.display(),
                    err
                )
            })?;
        provider.preprocess_lowercased();
        self.strings_provider = Some(provider);
        Ok(())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
fn create_id_assignement(
    stringprovider: Option<&CsvStringsData>,
    new_lineid: u32,
    selection: SelectedAudio,
) -> Result<IdAssignmentActionData, String> {
    let (text, actor) = if let Some(stringsprovider) = stringprovider {
        let actor = stringsprovider.get_actor(new_lineid);
        let text = stringsprovider.get_line(new_lineid)?;

        let replacer = Regex::new("[/\\?%*:|<>.$…, \"]")
            .map_err(|err| format!("failed to initialize filename escape regex: {}", err))?;

        let text = replacer
            .replace_all(text, "_")
            .chars()
            .take(MAX_TEXTHINT_CHARS)
            .collect();

        (text, actor)
    } else {
        ("-missing.strings.provider-".to_owned(), None)
    };

    Ok(IdAssignmentActionData {
        id: selection.id,
        lineid: new_lineid,
        duration: selection.duration,
        actor: actor.cloned(),
        text,
    })
}
// ----------------------------------------------------------------------------
