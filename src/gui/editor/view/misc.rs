//
// interactive view::misc
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(in gui) fn render(
    ui: &Ui<'_>,
    fonts: &Fonts,
    area: &UiArea,
    settings: &Settings,
    track: &PhonemeTrack<PhonemeSegment>,
) -> Option<EditorAction> {
    let mut result = None;

    let padding = ui.imgui().style().window_padding;
    ui.with_style_var(
        imgui::StyleVar::WindowPadding((padding.x, 0.0).into()),
        || {
            ui.window(im_str!("Additional Information"))
                .show_borders(false)
                .title_bar(false)
                .menu_bar(false)
                .movable(false)
                .resizable(false)
                .collapsible(false)
                .no_bring_to_front_on_focus(true)
                .position(area.pos, imgui::ImGuiCond::Always)
                .size(area.size, imgui::ImGuiCond::Always)
                .build(|| {
                    if ui
                        .collapsing_header(im_str!("input text"))
                        .default_open(true)
                        .build()
                    {
                        show_input_text(ui, fonts, track.input_text());
                    };

                    if ui.collapsing_header(im_str!("phoneme translation")).build() {
                        show_phoneme_translation(ui, fonts, track.translation());
                    };

                    if ui
                        .collapsing_header(im_str!("phoneme block drag mode"))
                        .build()
                    {
                        result = show_drag_settings(ui, settings);
                    }

                    if ui.collapsing_header(im_str!("actor profiles")).build() {
                        if let Some(selection) = show_actor_selection(ui, settings, track.actor()) {
                            result = Some(selection);
                        }
                    }

                    if ui.collapsing_header(im_str!("loaded phoneme data")).build() {
                        show_phonemes_traceback(ui, fonts, track);
                    }
                });
        },
    );
    result
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui;
use imgui::Ui;

use super::EditorAction;

use super::Settings;
use super::{Fonts, UiArea};
use super::{PhonemeDragMode, PhonemeSegment, PhonemeTrack};
// ----------------------------------------------------------------------------
// ui rendering helpers
// ----------------------------------------------------------------------------
#[inline]
fn show_input_text(ui: &Ui, fonts: &Fonts, input_text: &str) {
    ui.with_font(fonts.phonemes(), || {
        ui.spacing();
        ui.spacing();
        ui.text_wrapped(&imgui::ImString::new(input_text));
        ui.spacing();
        ui.spacing();
    });
}
// ----------------------------------------------------------------------------
#[inline]
fn show_phoneme_translation(ui: &Ui, fonts: &Fonts, translation: &str) {
    ui.with_font(fonts.phonemes(), || {
        ui.spacing();
        ui.spacing();
        ui.text_wrapped(&imgui::ImString::new(translation));
        ui.spacing();
        ui.spacing();
    });
}
// ----------------------------------------------------------------------------
#[inline]
fn show_drag_settings(ui: &Ui, settings: &Settings) -> Option<EditorAction> {
    let mut result = None;
    ui.spacing();
    if ui.radio_button_bool(
        im_str!("word bound"),
        *settings.drag_mode() == PhonemeDragMode::Words,
    ) {
        result = Some(EditorAction::SetDragMode(PhonemeDragMode::Words));
    }
    ui.same_line(0.0);
    if ui.radio_button_bool(
        im_str!("adjacent"),
        *settings.drag_mode() == PhonemeDragMode::Neighbour,
    ) {
        result = Some(EditorAction::SetDragMode(PhonemeDragMode::Neighbour));
    }
    ui.same_line(0.0);
    if ui.radio_button_bool(
        im_str!("unconstrained"),
        *settings.drag_mode() == PhonemeDragMode::None,
    ) {
        result = Some(EditorAction::SetDragMode(PhonemeDragMode::None));
    }
    ui.spacing();
    result
}
// ----------------------------------------------------------------------------
#[inline]
fn show_actor_selection(
    ui: &Ui,
    settings: &Settings,
    track_actor: Option<&String>,
) -> Option<EditorAction> {
    let unknown = &(imgui::ImString::default(), String::default());
    let mut result = None;
    ui.spacing();

    ui.text(im_str!("actor: "));
    ui.same_line(75.0);

    let actor = imgui::ImString::new(track_actor.map(|s| s.as_str()).unwrap_or("?"));
    ui.with_item_width(150.0, || ui.text(&actor));

    ui.same_line(250.0);
    let popupid = im_str!("actor_selection");

    ui.text(im_str!("mapped to: "));
    ui.same_line(100.0);

    let (selected_label, selected_id) = settings.selected_actor().unwrap_or(unknown);

    ui.same_line(0.0);
    if ui.selectable(
        selected_label,
        false,
        ::imgui::ImGuiSelectableFlags::empty(),
        (150.0, 0.0),
    ) {
        ui.open_popup(popupid);
    }

    ui.popup(popupid, || {
        ui.spacing();
        for (option, id) in settings.available_actors() {
            if ui.selectable(
                option,
                id == selected_id,
                ::imgui::ImGuiSelectableFlags::empty(),
                (0.0, 0.0),
            ) && id != selected_id
            {
                result = Some(EditorAction::SetActor(id.to_owned()));
            }
        }
        ui.spacing();
    });
    ui.spacing();
    result
}
// ----------------------------------------------------------------------------
#[inline]
fn show_phonemes_traceback(ui: &Ui, fonts: &Fonts, track: &PhonemeTrack<PhonemeSegment>) {
    ui.child_frame(im_str!("##phoneme_traceback_table"), (0.0, -5.0))
        .build(|| {
            ui.with_font(fonts.phonemes(), || {
                ui.text(imgui::ImString::new(
                    ";phoneme |start|  end|weight| score| status     | match + pocketsphinx timing",
                ));
                ui.separator();

                ui.child_frame(im_str!("##phoneme_traceback"), (0.0, -5.0))
                    .build(|| {
                        phoneme_traceback(ui, track);
                    });
            });
        });
}
// ----------------------------------------------------------------------------
#[inline]
fn phoneme_traceback(ui: &Ui<'_>, track: &PhonemeTrack<PhonemeSegment>) {
    let empty = String::from("");

    for (i, segment) in track.phonemes().iter().enumerate() {
        if i > 0 && segment.word_start {
            ui.separator();
        }

        let color_modifier = if segment.active { 0.9 } else { 0.7 };
        let color = if segment.score < 0.0 {
            (color_modifier, color_modifier, 0.1, color_modifier)
        } else {
            (
                color_modifier,
                color_modifier,
                color_modifier,
                color_modifier,
            )
        };

        // workaround for misalignment of columns for the ; deactivated-marker
        if segment.active {
            ui.text_colored(
                color,
                im_str!(" {}", segment.traceback.as_ref().unwrap_or(&empty)),
            );
        } else {
            ui.text_colored(
                color,
                im_str!("{}", segment.traceback.as_ref().unwrap_or(&empty)),
            );
        }
    }
}
// ----------------------------------------------------------------------------
