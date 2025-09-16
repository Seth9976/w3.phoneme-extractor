//
// gui/auxiliary windows/helpers
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(in gui) fn show_windows(ui: &Ui<'_>, windows: &mut WindowState, help: &HelpSystem) {
    if windows.show_help {
        render_help_window(ui, &mut windows.show_help, help);
    }

    // -- modals
    if windows.show_about {
        render_about_window(ui, &mut windows.show_about);
    }

    if windows.error.show() {
        windows::error(ui, &mut windows.error);
    }

    if windows.info.show() {
        windows::info(ui, &mut windows.info);
    }
}
// ----------------------------------------------------------------------------
pub(in gui) fn set_error(error: &mut windows::ErrorWindow, msg: &str) {
    error.set_msg(msg);
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui;
use imgui::Ui;
use imgui_support::windows;

use super::{HelpSystem, HelpTopic, WindowState};
// ----------------------------------------------------------------------------
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
// ----------------------------------------------------------------------------
fn render_about_window(ui: &Ui<'_>, opened: &mut bool) {
    ui.window(im_str!("About"))
        .size((600.0, 350.0), imgui::ImGuiCond::Always)
        .resizable(false)
        .movable(false)
        .opened(opened)
        .build_modal(|| {
            ui.text(im_str!("Witcher 3 Phoneme-Extractor"));
            ui.same_line(0.0);
            ui.text(format!(
                "v{}",
                VERSION.unwrap_or("unknown"),
            ));

            ui.separator();
            ui.text_wrapped(im_str!(
                "\nWitcher 3 Phoneme Extractor is part of radish modding tools.\n\n\
                radish modding tools are a collection of community created \
                modding tools aimed to enable the creation of new quests for \
                \"The Witcher 3: Wild Hunt\" game by CD Projekt Red.\n\n\
                The full package can be downloaded from nexusmods: \n\
                https://www.nexusmods.com/witcher3/mods/3620"
            ));
            ui.new_line();
            ui.text_wrapped(im_str!("Phoneme-Extractor sourcecode repository:\n\
                https://codeberg.org/rmemr/w3.phoneme-extractor\n\n\
                This program uses \n\
                the CMU Pocketsphinx library (https://github.com/cmusphinx/pocketsphinx),\n\
                the CMU Sphinx common libraries (https://github.com/cmusphinx/sphinxbase),\n\
                the eSpeak Library (http://espeak.sourceforge.net).\n\
                Used UI Framework: \"Dear ImGui\" by Omar Cornut and others \
                (https://github.com/ocornut/imgui)\n\n\
                See Cargo.toml for all used rust crates and libraries."));

            ui.text_wrapped(im_str!(""));
        });
}
// ----------------------------------------------------------------------------
fn render_help_window(ui: &Ui<'_>, opened: &mut bool, help: &HelpSystem) {
    ui.window(im_str!("Documentation"))
        .size((700.0, 500.0), imgui::ImGuiCond::FirstUseEver)
        .opened(opened)
        .build(|| {
            let mut is_missing = true;
            for topic in help.topics() {
                match topic {
                    HelpTopic::General(header) => {
                        is_missing = false;
                        ui.tree_node(header).build(|| {
                            if let Some(helptxt) = help.get(topic) {
                                ui.text_wrapped(helptxt);
                            }
                            ui.new_line();
                            ui.separator();
                        });
                    }
                }
            }

            if is_missing {
                ui.text(im_str!("Documentation missing!"));
            }
        });
}
// ----------------------------------------------------------------------------
