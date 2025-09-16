//
// gui::update - simple(r) actions for updating state, mapping to other actions
// or set of actions
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(super) fn handle_menu_selection(
    selection: MenuSelection,
    state: &mut State,
) -> Option<ActionSequence> {
    match selection {
        MenuSelection::ShowHelp => {
            state.windows.show_help = true;
            None
        }
        MenuSelection::ShowAbout => {
            state.windows.show_about = true;
            None
        }
        MenuSelection::RenameAllFiles => {
            Some(ima_seq![Action::GuardModifiedData, Action::RenameAllFiles])
        }
        MenuSelection::ResizeQueueHeight(new_height) => {
            Some(ima_seq![Action::ResizeQueueHeight(new_height)])
        }
        MenuSelection::LoadFile => {
            Some(ima_seq![Action::GuardModifiedData, Action::OpenFileBrowser])
        }
        MenuSelection::SaveFile => Some(ima_seq![Action::SaveCurrent]),
        MenuSelection::CloseDirectory => {
            Some(ima_seq![Action::GuardModifiedData, Action::CloseDir])
        }
        MenuSelection::Quit => Some(ima_seq![Action::GuardModifiedData, Action::Quit]),
    }
}
// ----------------------------------------------------------------------------
pub(super) fn handle_filebrowser_selection(
    selection: filebrowser::Selection,
    _state: &mut State,
) -> ActionSequence {
    match selection {
        filebrowser::Selection::Dir(ref dir) => {
            ima_seq![Action::CloseDir, Action::ChangeDir(PathBuf::from(dir))]
        }
    }
}
// ----------------------------------------------------------------------------
pub(super) fn handle_hotkey(hotkey: Hotkey, state: &mut State) -> Option<Action> {
    match hotkey {
        Hotkey::Space => {
            if let Ok(playback) = state.player.state() {
                match playback {
                    player::PlaybackState::Idle => Some(Action::PlayAudio),
                    player::PlaybackState::Playing => Some(Action::StopAudio),
                }
            } else {
                None
            }
        }
    }
}
// ----------------------------------------------------------------------------
//
// ----------------------------------------------------------------------------
use std::path::PathBuf;

use super::filebrowser;
use super::hotkeys::Hotkey;
use super::player;

use super::{Action, ActionSequence, MenuSelection};

use super::State;
// ----------------------------------------------------------------------------
