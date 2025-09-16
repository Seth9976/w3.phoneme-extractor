//
// interactive view::timeline
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(in gui) fn render(
    ui: &Ui<'_>,
    fonts: &Fonts,
    area: &UiArea,
    data: &EditableData,
    playpos: usize,
) -> Option<EditorAction> {
    let mut result = None;

    ui.window(im_str!("Audio Timeline"))
        .title_bar(false)
        .menu_bar(false)
        .movable(false)
        .resizable(false)
        .no_bring_to_front_on_focus(true)
        .position(area.pos, imgui::ImGuiCond::Always)
        .size(area.size, imgui::ImGuiCond::Always)
        .build(|| {
            let padding_right = 20.0;
            let waveform_width = ui.get_contentregion_avail_width() - padding_right;

            ui.with_item_width(-padding_right, || {
                let mut visible_range = (0, 0);
                let mut zoom = data.zoom;
                let mut offset = data.offset as i32;

                if let Some(interaction) = ui
                    .audio()
                    .waveform(im_str!("Waveform"), &data.audio.samples, &mut visible_range)
                    .offset(data.offset)
                    .zoom_x(data.zoom)
                    .max_value(data.audio.max)
                    .marker1_pos(data.start_marker)
                    .marker_area(data.start_marker, data.end_marker)
                    .marker2_pos(playpos)
                    .graph_size((waveform_width, 150.0))
                    .build()
                {
                    use self::MouseInteraction::*;
                    use self::TimelineAction::*;

                    let scaling = (visible_range.1 - visible_range.0) as f32 / waveform_width;

                    result = match *interaction.mouse() {
                        Clicked(0) => Some(SetPlaybackStart(interaction.virtual_position()).into()),
                        Clicked(1) => Some(SetPlaybackEnd(interaction.virtual_position()).into()),
                        Clicked(2) => Some(WaveformDragStart(scaling).into()),
                        Wheel(zoom) => {
                            Some(ZoomChange(zoom, interaction.virtual_position()).into())
                        }
                        _ => None,
                    };
                }

                ui.same_line(waveform_width + 10.0);

                if ui
                    .vslider_float(im_str!("##zoom"), &mut zoom, MIN_ZOOM, MAX_ZOOM)
                    .size((15.0, 150.0))
                    .power(5.0)
                    .display_format(im_str!(""))
                    .build()
                {
                    result = Some(TimelineAction::Zoom(zoom).into())
                }

                let mut hovered_position = -1.0;

                if let Some(interaction) = ui
                    .audio()
                    .timeline(
                        im_str!("PhonemeTrack"),
                        data.audio.duration,
                        &mut hovered_position,
                    )
                    .view_start(data.offset as f32 / data.audio.rate as f32)
                    .zoom(data.zoom)
                    .size((waveform_width, 30.0))
                    .build(|_pos, size, visible_timeframe| {
                        ui.with_font(fonts.phonemes(), || {
                            show_phoneme_blocks(
                                ui,
                                size,
                                visible_timeframe,
                                data.phonemetrack.phonemes(),
                                &mut result,
                            );
                        })
                    })
                {
                    // prefer block interactions
                    if result.is_none() {
                        result = match *interaction.mouse() {
                            MouseInteraction::Wheel(zoom) => Some(
                                TimelineAction::ZoomChange(
                                    zoom,
                                    (data.audio.samples.len() as f32
                                        * interaction.virtual_position()
                                        / data.audio.duration)
                                        as usize,
                                )
                                .into(),
                            ),
                            _ => None,
                        };
                    }
                }
                // data.hover_marker = f32::trunc(hovered_position * data.audio.rate as f32) as usize;

                // scale position slider (max) to respect size of window
                let max_offset = calc_window_max_start_offset(data.zoom, data.audio.samples.len());

                if ui
                    .slider_int(im_str!("##pos"), &mut offset, 0, max_offset)
                    .display_format(im_str!(""))
                    .build()
                {
                    // since range is [0, max_offset] conversion to usize is correct
                    result = Some(TimelineAction::DataOffset(offset as usize).into());
                }
            });
        });

    result
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui::Ui;
use imgui::{self, ImGuiCol};
use imgui_widgets::{AudioWidgets, MouseInteraction};

// actions
use super::{EditorAction, TimelineAction};

// state
use super::EditableData;

// misc
use super::PhonemeSegment;
use super::{Fonts, UiArea};
use super::{MAX_ZOOM, MIN_ZOOM};

// ----------------------------------------------------------------------------
// ui rendering helpers
// ----------------------------------------------------------------------------
#[inline]
fn show_phoneme_blocks(
    ui: &Ui<'_>,
    frame_size: imgui::ImVec2,
    visible_timeframe: (f32, f32),
    phonemes: &[PhonemeSegment],
    result: &mut Option<EditorAction>,
) {
    let (start, end) = (visible_timeframe.0, visible_timeframe.1);

    // phonemesegment times are ms
    let start_ms = f32::trunc(start * 1000.0) as u32;
    let end_ms = f32::trunc(end * 1000.0) as u32;
    let scaling = (end_ms - start_ms) as f32 / frame_size.x;

    let draw_captions = (end - start) / frame_size.x < 0.006;
    let draw_borders = (end - start) / frame_size.x < 0.003;

    for (i, segment) in phonemes.iter().enumerate() {
        if segment.active && segment.start < end_ms && segment.end > start_ms {
            let col = segment.color(false);
            ui.with_color_var(ImGuiCol::Text, col, || {
                if let Some(interaction) = ui
                    .audio()
                    .timeline_block(
                        im_str!("PhonemeSegment##{}", i),
                        im_str!("{}", segment.phoneme),
                        (segment.start as f32 / 1000.0, segment.end as f32 / 1000.0),
                        (start, end),
                    )
                    .set_draw_framesize(frame_size)
                    .set_always_draw_borders(false, false)
                    //.color depending on score?
                    // .set_draw_color((128.0, 128.0, 128.0 + 128.0 * (1.0 - segment.score), 1.0))
                    .set_draw_color(col)
                    .set_alpha(0.2 + (segment.weight - 0.5) / 2.0)
                    .set_draw_label_color((1.0, 1.0, 1.0, 0.75 + (segment.weight - 0.5) / 2.0))
                    .set_draw_label(draw_captions)
                    .set_draw_borders(draw_borders)
                    .build()
                {
                    *result = match *interaction.mouse() {
                        MouseInteraction::Clicked(0) => Some(
                            TimelineAction::PhonemeDragStart(
                                i,
                                interaction.virtual_position().x,
                                scaling,
                            )
                            .into(),
                        ),
                        _ => None,
                    };
                }
            });
            if ui.is_item_hovered() && !segment.warnings.is_empty() {
                ui.tooltip(|| {
                    let mut warnings = segment
                        .warnings
                        .iter()
                        .map(|w| imgui::ImString::new(w.short()));

                    if let Some(txt) = warnings.next() {
                        ui.text(txt);
                    }
                    for txt in warnings {
                        ui.separator();
                        ui.text(txt);
                    }
                });
            }
        }
    }
}
// ----------------------------------------------------------------------------
#[inline]
fn calc_window_max_start_offset(zoom: f32, datapoints: usize) -> i32 {
    let win_size = f32::trunc(datapoints as f32 / zoom);
    if (i32::MAX as f32) < (datapoints as f32 - win_size) {
        i32::MAX
    } else {
        (datapoints as f32 - win_size) as i32
    }
}
// ----------------------------------------------------------------------------
