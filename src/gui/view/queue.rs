//
// gui::view::queue
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(in gui) fn render_selection(
    ui: &Ui<'_>,
    area: &UiArea,
    fonts: &Fonts,
    queue: &queue::AudioQueue,
) -> Option<Action> {
    let mut result = None;

    ui.with_style_var(StyleVar::WindowRounding(0.0), || {
        ui.window(im_str!("Audio selection {}", queue.info()))
            .menu_bar(false)
            .movable(false)
            .resizable(false)
            .collapsible(false)
            .no_bring_to_front_on_focus(true)
            .position(area.pos, imgui::ImGuiCond::Always)
            .size(area.size, imgui::ImGuiCond::Always)
            .build(|| {
                let mut selected = queue.selected().map(|i| *i as i32).unwrap_or(-1);

                // (area_height - titlebar) / item_height
                let height_in_items = (((area.size.1 - 40.0) / 19.0).trunc() as i32).clamp(5, 50);

                ui.with_font(fonts.phonemes(), || {
                    if ui
                        .list_box2(
                            queue
                                .captions()
                                .map(Borrow::borrow)
                                .collect::<Vec<_>>()
                                .as_slice(),
                            &mut selected,
                        )
                        .height_in_items(height_in_items)
                        .autowidth()
                        .build()
                    {
                        if let Some(selected_entry) = queue.is_selectable(selected as usize) {
                            result = Some(Action::OnSelectEntry(selected_entry))
                        }
                    }
                });
            });
    });
    result
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::borrow::Borrow;

use imgui;
use imgui::{StyleVar, Ui};

use gui::support::Fonts;

// actions
use super::Action;

// misc
use gui::queue;

// util
use super::UiArea;
// ----------------------------------------------------------------------------
