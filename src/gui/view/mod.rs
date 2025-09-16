//
// gui::view
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------

pub mod auxiliary;
pub mod menu;

// ----------------------------------------------------------------------------
#[inline]
pub(super) fn show_editor(
    ui: &Ui<'_>,
    screenspace: &ScreenSpaceManager,
    fonts: &Fonts,
    state: &State,
) -> Option<Action> {
    let mut result =
        queue::render_selection(ui, screenspace.selection_queue(), fonts, &state.audioqueue);

    if state.editor_data.is_available() {
        ui.with_style_var(StyleVar::WindowRounding(0.0), || {
            if let Some(action) = editor::view::timeline::render(
                ui,
                fonts,
                screenspace.timeline(),
                &state.editor_data,
                state.player.playpos(),
            ) {
                result = Some(action.into());
            }

            if let Some(action) = editor::view::table::render(
                ui,
                fonts,
                screenspace.phoneme_table(),
                &state.editor_data,
            ) {
                result = Some(action.into());
            }

            if let Some(action) = editor::view::misc::render(
                ui,
                fonts,
                screenspace.data_info(),
                &state.settings,
                state.editor_data.phonemetrack(),
            ) {
                result = Some(action.into());
            }
        });
    }
    result
}
// ----------------------------------------------------------------------------
#[inline]
pub(super) fn show_lineid_selector(
    ui: &Ui<'_>,
    fonts: &Fonts,
    state: &mut idselector::IdSelectorState,
) -> Option<Action> {
    if state.is_opened() {
        idselector::view::render(ui, fonts, state).map(Into::into)
    } else {
        None
    }
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui::{StyleVar, Ui};

use gui::support::Fonts;

// actions
use super::{Action, MenuSelection};

// state
use super::help::{HelpSystem, HelpTopic};
use super::{State, WindowState};

// subviews
use super::editor;
use super::idselector;

// util
use super::{ScreenSpaceManager, UiArea};
// ----------------------------------------------------------------------------
mod queue;
// ----------------------------------------------------------------------------
