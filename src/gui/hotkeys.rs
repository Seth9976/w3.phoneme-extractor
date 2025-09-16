//
// hotkey mapping
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
#[derive(Default)]
pub(super) struct HotkeyState {
    space: bool,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum Hotkey {
    Space,
}
// ----------------------------------------------------------------------------
pub(super) fn check_pressed(ui: &Ui<'_>, hotkeystate: &mut HotkeyState) -> Option<Action> {
    let mut result = None;
    let keys_down = ui.imgui().keys_down();

    let space_down = keys_down[32];

    if hotkeystate.space != space_down && space_down {
        result = Some(Action::HotkeyPressed(Hotkey::Space));
    }

    hotkeystate.space = space_down;
    result
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use super::Action;
use super::Ui;
// ----------------------------------------------------------------------------
