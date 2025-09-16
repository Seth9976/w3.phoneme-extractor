//
// gui::view::menu
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(in gui) fn render(
    ui: &Ui<'_>,
    screenspace: &ScreenSpaceManager,
    state: &State,
) -> Option<MenuSelection> {
    let mut result = None;
    ui.main_menu_bar(|| {
        ui.menu(im_str!("File")).build(|| {
            if ui.menu_item(im_str!("Load audio")).build() {
                result = Some(MenuSelection::LoadFile);
            }
            if ui
                .menu_item(im_str!("Save current"))
                // .enabled(state.data.changed())
                .enabled(state.editor_data.is_available())
                .build()
            {
                result = Some(MenuSelection::SaveFile);
            }

            if ui
                .menu_item(im_str!("Close directory"))
                // .enabled(state.editor_data.is_available())
                .build()
            {
                result = Some(MenuSelection::CloseDirectory);
            }
            ui.separator();
            if ui
                .menu_item(im_str!("Rename all files"))
                .enabled(!state.audioqueue.is_empty())
                .build()
            {
                result = Some(MenuSelection::RenameAllFiles);
            }
            ui.separator();
            if ui.menu_item(im_str!("Quit")).build() {
                result = Some(MenuSelection::Quit);
            }
        });

        ui.menu(im_str!("Settings")).build(|| {
            // queue area height
            let mut height = screenspace.selection_queue().size.1;

            if ui
                .slider_float(im_str!("queue panel height"), &mut height, 150.0, 750.0)
                .display_format(im_str!("%.0f"))
                .build()
            {
                result = Some(MenuSelection::ResizeQueueHeight(height));
            }
        });

        ui.menu(im_str!("Help")).build(|| {
            if state.help.topics().count() > 0 {
                if ui.menu_item(im_str!("Documentation")).build() {
                    result = Some(MenuSelection::ShowHelp);
                }
                ui.separator();
            }
            if ui.menu_item(im_str!("About")).build() {
                result = Some(MenuSelection::ShowAbout);
            }
        });

        ui.separator();
        ui.text_disabled(im_str!("Language: {}", state.settings.selected_language()));
    });
    result
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui::Ui;

use super::MenuSelection;

use super::{ScreenSpaceManager, State};
// ----------------------------------------------------------------------------
