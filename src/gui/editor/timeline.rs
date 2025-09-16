//
// editor::timeline
//
// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
#[inline]
pub(super) fn handle_action(
    action: TimelineAction,
    settings: &Settings,
    state: &mut EditableData,
    player: &mut player::Player,
) -> Option<EditorAction> {
    let result = None;

    match action {
        TimelineAction::PhonemeDragStart(slot, click_pos, scaling) => {
            start_phonemeblock_drag(settings, slot, click_pos, scaling, state);
        }
        TimelineAction::WaveformDragStart(scaling) => {
            start_waveform_drag(state, scaling);
        }
        TimelineAction::Zoom(zoom) => {
            state.set_zoom(zoom);
        }
        TimelineAction::ZoomChange(amount, fixture) => {
            state.adjust_zoom_with_fixpoint(amount, fixture);
        }
        TimelineAction::DataOffset(offset) => {
            state.set_offset(offset);
        }
        TimelineAction::SetPlaybackStart(start) => {
            state.set_playback_start_pos(start);
            player.seek(state.playback_start_pos());
            player.clip_at(state.playback_end_pos());
        }
        TimelineAction::SetPlaybackEnd(end) => {
            state.set_playback_end_pos(end);
            player.seek(state.playback_start_pos());
            player.clip_at(state.playback_end_pos());
        }
        TimelineAction::DragStop => {
            state.dragging = None;
        }
    }
    result
}
// ----------------------------------------------------------------------------
use super::player;

// state
use super::{Controlpoint, EditableData, PhonemeDragging, Settings};

// actions
use super::{Action as EditorAction, TimelineAction};

// misc
use super::{DragOperation, PhonemeDragMode, PhonemeSegment, UiDragging};

use super::clamp_ms;
// ----------------------------------------------------------------------------
fn start_waveform_drag(state: &mut EditableData, scaling: f32) {
    state.dragging = Some(DragOperation::Waveform(
        UiDragging::start(state.offset),
        scaling,
    ));
}
// ----------------------------------------------------------------------------
fn start_phonemeblock_drag(
    settings: &Settings,
    slot: usize,
    click_pos: f32,
    scaling: f32,
    state: &mut EditableData,
) {
    let drag = PhonemeDragging::new(
        state.phonemetrack.phonemes(),
        slot,
        click_pos,
        scaling,
        state.audio.duration_ms() as f32,
        settings.drag_mode(),
        settings.drag_damping(),
    );
    if let Some(drag) = drag {
        state.dragging = Some(DragOperation::PhonemeBlock(drag));
    }
}
// ----------------------------------------------------------------------------
impl Controlpoint {
    fn new(value: u32, weight: f32) -> Controlpoint {
        Controlpoint {
            value: value as f32,
            weight,
        }
    }
}
// ----------------------------------------------------------------------------
impl PhonemeDragging {
    // ------------------------------------------------------------------------
    fn new(
        phonemes: &[PhonemeSegment],
        clicked_slot: usize,
        clicked_pos: f32,
        scaling: f32,
        audio_duration: f32,
        dragmode: &PhonemeDragMode,
        damping: f32,
    ) -> Option<PhonemeDragging> {
        let (left_border, right_border) = if clicked_pos < 0.3 {
            (true, false)
        } else if clicked_pos > 0.7 {
            (false, true)
        } else {
            return None;
        };

        let clicked_time = match phonemes.get(clicked_slot) {
            Some(clicked_block) if left_border => clicked_block.start,
            Some(clicked_block) /* otherwise */=> clicked_block.end,
            None => return None,
        };

        let timeline = (0.0, audio_duration);

        let (first_slot, last_slot) = match *dragmode {
            PhonemeDragMode::None => ((clicked_slot, clicked_slot), (clicked_slot, clicked_slot)),
            PhonemeDragMode::Neighbour => {
                Self::find_adjacent_segments(clicked_slot, left_border, phonemes)
            }
            PhonemeDragMode::Words => Self::find_word_boundary(clicked_slot, phonemes),
        };
        let (first_slot, first_active_slot) = first_slot;
        let (last_slot, last_active_slot) = last_slot;

        let (block_start, block_end) = {
            let block_start = phonemes.get(first_slot).map_or(0, |block| block.start);
            let block_end = phonemes
                .get(last_slot)
                .or_else(|| phonemes.last())
                .map_or(block_start, |block| block.end);

            (block_start, block_end)
        };

        let mut valid_range = (block_start, block_end);
        if clicked_slot <= first_active_slot && left_border {
            valid_range.0 = timeline.0 as u32;
        }
        if clicked_slot >= last_active_slot && right_border {
            valid_range.1 = timeline.1 as u32;
        }

        let border_points = phonemes
            .iter()
            .skip(first_slot)
            .take_while(|segment| segment.end <= block_end)
            .map(|segment| {
                let weight_left = Self::calculate_proportional_drag_weight(
                    segment.start,
                    clicked_time,
                    block_start,
                    block_end,
                    damping,
                );

                let weight_right = Self::calculate_proportional_drag_weight(
                    segment.end,
                    clicked_time,
                    block_start,
                    block_end,
                    damping,
                );
                (
                    Controlpoint::new(segment.start, scaling * weight_left),
                    Controlpoint::new(segment.end, scaling * weight_right),
                )
            })
            .collect();

        Some(PhonemeDragging {
            // only delta will be used in update so start position is irrelevant
            dragging: UiDragging::start(0.0),

            drag_start_slot: first_slot,
            border_points,

            valid_range,
        })
    }
    // ------------------------------------------------------------------------
    fn find_adjacent_segments(
        clicked_slot: usize,
        left_border: bool,
        phonemes: &[PhonemeSegment],
    ) -> ((usize, usize), (usize, usize)) {
        let (first_slot, last_slot) = if left_border {
            (clicked_slot.saturating_sub(1), clicked_slot)
        } else {
            (
                clicked_slot,
                usize::min(
                    phonemes.len().saturating_sub(1),
                    clicked_slot.saturating_add(1),
                ),
            )
        };

        if last_slot <= first_slot {
            ((first_slot, first_slot), (first_slot, first_slot))
        } else {
            let mut last_end = u32::MAX;
            let (first, last) = phonemes
                .iter()
                .enumerate()
                .filter(|&(_, block)| block.active)
                .skip(first_slot)
                .take(last_slot - first_slot)
                .fold((first_slot, last_slot), |acc, (i, block)| {
                    // break link on timing gaps (these are NOT necessarily word boundaries!)
                    if last_end < block.start {
                        last_end = block.end;
                        match i {
                            i if i == first_slot => unreachable!(),
                            i if i == clicked_slot => (i, usize::max(i, acc.1)),
                            _ => (acc.0, usize::max(acc.0, acc.1.saturating_sub(1))),
                        }
                    } else {
                        last_end = block.end;
                        acc
                    }
                });
            // no distinction between active/noactive necessary for adjacent segments
            ((first, first), (last, last))
        }
    }
    // ------------------------------------------------------------------------
    fn find_word_boundary(
        clicked_slot: usize,
        phonemes: &[PhonemeSegment],
    ) -> ((usize, usize), (usize, usize)) {
        // find word boundary
        let (start, end) = phonemes
            .iter()
            .enumerate()
            .filter(|&(_, block)| block.word_start)
            .fold((0, phonemes.len() - 1), |acc, (i, _)| {
                if i <= clicked_slot {
                    // found the word start slot (end still points to last)
                    (i, acc.1)
                } else {
                    // crossed the cliked position and found another word start slot
                    (acc.0, usize::min(acc.1, i - 1))
                }
            });

        // find first and last *active* block within word
        let (first_active, last_active) = phonemes
            .iter()
            .enumerate()
            .skip(start)
            .take(end - start + 1)
            .filter(|&(_, block)| block.active)
            .fold((clicked_slot, clicked_slot), |acc, (i, _)| {
                (usize::min(acc.0, i), (usize::max(acc.1, i)))
            });

        ((start, first_active), (end, last_active))
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn calculate_proportional_drag_weight(
        pos: u32,
        reference_pos: u32,
        block_start: u32,
        block_end: u32,
        damping: f32,
    ) -> f32 {
        match pos {
            pos if pos < reference_pos => {
                damping * (pos as f32 - block_start as f32)
                    / (reference_pos as f32 - block_start as f32)
            }
            pos if pos > reference_pos => {
                damping * (block_end as f32 - pos as f32)
                    / (block_end as f32 - reference_pos as f32)
            }
            _ => 1.0,
        }
    }
    // ------------------------------------------------------------------------
    pub(super) fn update(&mut self, phonemes: &mut [PhonemeSegment], granularity: f32) -> bool {
        let (global_min, global_max) = self.valid_range;
        let start_slot = self.drag_start_slot;
        let border_points = self.border_points.iter();

        self.dragging.update(|_offset, delta| {
            let mut local_min = global_min;
            let local_max = global_max;
            phonemes
                .iter_mut()
                .skip(start_slot)
                .zip(border_points)
                .for_each(|(ref mut p, (start, end))| {
                    let new_start = clamp_ms(
                        f32::trunc(start.value + delta.x * start.weight),
                        granularity,
                        global_min,
                        global_max,
                    );
                    let new_end = clamp_ms(
                        f32::trunc(end.value + delta.x * end.weight),
                        granularity,
                        global_min,
                        global_max,
                    );

                    // prevent overlapping of blocks
                    p.start = u32::min(local_max, u32::max(local_min, new_start));
                    p.end = u32::min(local_max, u32::max(p.start, new_end));
                    local_min = p.end;
                });
        })
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
