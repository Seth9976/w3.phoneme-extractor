//
// gui::view::idselector
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(in gui) fn render(ui: &Ui<'_>, fonts: &Fonts, state: &mut IdSelectorState) -> Option<Action> {
    let mut result = None;

    let mut opened = true;

    ui.window(im_str!(" Voiceline id assignment"))
        .size((750.0, 415.0), imgui::ImGuiCond::Always)
        .resizable(false)
        .movable(false)
        .opened(&mut opened)
        .build_modal(|| {
            let button_size = (100.0, 0.0);

            if let Some(selection) = &state.selection {
                ui.text(im_str!("audiofile: "));
                ui.same_line(0.0);
                ui.with_font(fonts.phonemes(), || {
                    ui.text(&selection.audiofile);
                });

                ui.same_line(750.0 - 109.0);
                if ui.enabled_button(im_str!("play"), button_size, true) {
                    result = Some(Action::ToggleAudioplayback);
                }
            }

            // -- search filter + result list
            let mut selected = state.selected_slot;
            ui.separator();
            ui.spacing();
            ui.with_font(fonts.phonemes(), || {
                if let Some(action) = state.search_filter.draw(ui).map(|action| {
                    Action::OnSearchFilterChanged(action.as_str().unwrap_or("").to_string())
                }) {
                    result = Some(action);
                } else {
                    state.search_filter.reset_changed();
                }
            });
            if state.search_capped {
                ui.same_line(531.0);
                ui.text(im_str!("showing only first {} matches", MAX_SEARCH_RESULTS));
            } else {
                ui.same_line(671.0);
                ui.text(im_str!("{:>2} matches", state.search_result.len()));
            }
            ui.spacing();
            ui.with_font(fonts.phonemes(), || {
                if ui
                    .list_box2(
                        state
                            .search_result
                            .iter()
                            .map(|(_, caption)| caption.borrow())
                            .collect::<Vec<_>>()
                            .as_slice(),
                        &mut selected,
                    )
                    .height_in_items(10)
                    .autowidth()
                    .build()
                {
                    result = Some(Action::OnSelectLine(selected));
                }
            });
            // -- selected lines full text (if it is a long line)
            ui.spacing();
            ui.text(im_str!("full text:"));
            ui.separator();
            ui.with_font(fonts.phonemes(), || {
                ui.with_region_height(im_str!("##new_socket"), 2.5, || {
                    if let Some(text) = &state.selected_text {
                        ui.text_wrapped(text);
                    }
                });
            });
            ui.separator();

            // -- buttons
            ui.spacing();
            ui.new_line();
            ui.same_line(750.0 / 2.0 - 350.0);
            if ui.enabled_button(im_str!("cancel"), button_size, true) {
                result = Some(Action::Cancel);
            }
            ui.same_line(750.0 / 2.0 + 250.0);
            if ui.enabled_button(im_str!("assign id"), button_size, state.selected_slot >= 0) {
                result = state
                    .search_result
                    .get(state.selected_slot as usize)
                    .map(|(id, _)| Action::OnAssignId(*id));
            }
        });

    // X-button click closes window (sets opened flag to false)
    if opened {
        result
    } else {
        Some(Action::Cancel)
    }
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::borrow::Borrow;

use imgui::Ui;

use imgui_controls::input::Field;

use gui::support::Fonts;

// state
use super::IdSelectorState;

// actions
use super::Action;

// misc
use super::MAX_SEARCH_RESULTS;

// util

// ----------------------------------------------------------------------------
