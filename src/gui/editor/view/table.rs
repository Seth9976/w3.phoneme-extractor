//
// interactive view::table
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(in gui) fn render(
    ui: &Ui<'_>,
    fonts: &Fonts,
    area: &UiArea,
    data: &EditableData,
) -> Option<EditorAction> {
    let mut result = None;

    ui.window(im_str!("Phoneme Segment Positions"))
        // .title_bar(false)
        .menu_bar(false)
        .movable(false)
        .resizable(false)
        .collapsible(false)
        .no_bring_to_front_on_focus(true)
        .position(area.pos, imgui::ImGuiCond::Always)
        .size(area.size, imgui::ImGuiCond::Always)
        .build(|| {
            let duration = data.audio.duration_ms();

            phoneme_table_header(ui, area.size.0, &mut result);

            let start_marker = (data.start_marker as f32 / data.audio.rate as f32) * 1000.0;
            let end_marker = (data.end_marker as f32 / data.audio.rate as f32) * 1000.0;

            ui.child_frame(im_str!("##phoneme_table"), (0.0, 0.0))
                .build(|| {
                    ui.columns(3, im_str!("phoneme_table"), false);

                    ui.set_column_offset(1, 60.0);
                    ui.set_column_offset(2, area.size.0 - 150.0);

                    for (i, segment) in data.phonemetrack.phonemes().iter().enumerate() {
                        if segment.word_start {
                            if i > 0 {
                                ui.spacing();
                                ui.separator();
                            }
                            ui.spacing();
                        }

                        let mut vec2: [i32; 2] = [segment.start as i32, segment.end as i32];
                        let mut weight = segment.weight;
                        let mut activate = segment.active;
                        let mut row_hovered = false;

                        if ui.checkbox(im_str!("##active_{}", i), &mut activate) {
                            result = Some(EditorAction::ActivatePhonemeSegment(i, activate))
                        }

                        // highlight table row if its segment is within marked audio area
                        let highlight = (end_marker >= segment.start as f32)
                            && (segment.end as f32 >= start_marker);
                        let col = segment.color(highlight);

                        ui.with_color_var(ImGuiCol::Text, col, || {
                            ui.same_line(0.0);
                            ui.with_font(fonts.phonemes(), || {
                                ui.text(im_str!("{}", segment.phoneme));
                            });

                            row_hovered |= ui.is_item_hovered();

                            ui.next_column();
                            if segment.word_start {
                                ui.spacing();
                            }

                            ui.with_item_width(-1.0, || {
                                if ui
                                    .slider_int2(
                                        im_str!("##segment{}", i),
                                        &mut vec2,
                                        0,
                                        duration as i32,
                                    )
                                    .display_format(im_str!("%.0f ms"))
                                    .build()
                                {
                                    result = Some(EditorAction::SetPhonemeSegmentPos(
                                        i,
                                        vec2[0] as f32,
                                        vec2[1] as f32,
                                    ))
                                }
                            });
                            row_hovered |= ui.is_item_hovered();

                            ui.next_column();
                            if segment.word_start {
                                ui.spacing();
                            }

                            ui.with_item_width(-1.0, || {
                                if ui
                                    .slider_float(im_str!("##{}", i), &mut weight, 0.5, 1.5)
                                    .display_format(im_str!("%.02f"))
                                    .build()
                                {
                                    result = Some(EditorAction::SetPhonemeSegmentWeight(i, weight))
                                }
                            });
                            row_hovered |= ui.is_item_hovered();
                        });
                        if row_hovered && !segment.warnings.is_empty() {
                            ui.tooltip(|| {
                                let mut warnings = segment
                                    .warnings
                                    .iter()
                                    .map(|w| imgui::ImString::new(w.long()));

                                if let Some(txt) = warnings.next() {
                                    ui.text(txt);
                                }
                                for txt in warnings {
                                    ui.separator();
                                    ui.text(txt);
                                }
                            });
                        }

                        ui.next_column();
                    }
                    ui.columns(1, im_str!("phoneme_table"), false);
                });
        });

    result
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui;
use imgui::ImGuiCol;
use imgui::Ui;

// actions
use super::EditorAction;

// state
use super::EditableData;

// misc
use super::{Fonts, UiArea};
// ----------------------------------------------------------------------------
// ui rendering helpers
// ----------------------------------------------------------------------------
#[inline]
fn phoneme_table_header(ui: &Ui<'_>, width: f32, action: &mut Option<EditorAction>) {
    ui.columns(3, im_str!("phoneme_table"), false);

    ui.set_column_offset(1, 80.0);
    ui.set_column_offset(2, width - 150.0);

    let timing_col_width = ui.get_column_width(1);
    if ui.small_button(im_str!("gap close")) {
        *action = Some(EditorAction::AutoCloseGaps);
    }

    ui.next_column();
    ui.same_line(timing_col_width * 0.25 - 16.0);
    ui.text(im_str!("start"));
    ui.same_line(timing_col_width * 0.75 - 16.0);
    ui.text(im_str!(" end "));

    ui.next_column();
    ui.same_line(ui.get_column_width(2) * 0.5 - 20.0);
    ui.text(im_str!("weight"));

    ui.next_column();
    ui.separator();
    ui.columns(1, im_str!("phoneme_table"), false);
}
// ----------------------------------------------------------------------------
