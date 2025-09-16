//
// editor::update
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
#[inline]
pub(super) fn handle_action(
    action: EditorAction,
    state: &mut EditableData,
    settings: &mut Settings,
    player: &mut player::Player,
) -> Option<Action> {
    use self::EditorAction::*;
    let mut result = None;

    match action {
        Timeline(action) => {
            timeline::handle_action(action, settings, state, player);
        }

        SetDragMode(mode) => settings.set_drag_mode(mode),
        SetActor(actor) => {
            if let Some(new_actor_id) = settings.set_actor(&actor) {
                // Note: check if the *unmodified* track actor is not set yet
                // because once the actor is set on the edited track it would
                // not be possible to change it again in the current session
                // Note: once track is saved the actual track actor won't be
                // editable anymore - only the mapped to!
                let initial_actor = state.unmodified.actor();
                let has_default_actor =
                    initial_actor.is_none() || initial_actor == Some(&String::from("default"));

                if has_default_actor {
                    state.phonemetrack.set_actor(new_actor_id)
                }

                if let Some(actor) = state.phonemetrack.actor() {
                    result = Some(Action::UpdateActorMapping(
                        actor.to_string(),
                        new_actor_id.to_string(),
                    ));
                }
            }
        }

        AutoCloseGaps => {
            ::phonemes::auto_close_gaps(state.audio.duration_ms(), &mut state.phonemetrack);
        }

        ActivatePhonemeSegment(slot, activated) => {
            ::phonemes::update_timings_on_activation(
                state.audio.duration_ms(),
                state.phonemetrack.phonemes_mut(),
                slot,
                activated,
            );
        }

        SetPhonemeSegmentPos(slot, start, end) => {
            let granularity = settings.granularity_ms();
            let duration = state.audio.duration_ms();

            state
                .phonemetrack
                .phonemes_mut()
                .iter_mut()
                .skip(slot)
                .take(1)
                .for_each(|segment| {
                    segment.start = clamp_ms(start, granularity, 0, segment.end);
                    segment.end = clamp_ms(end, granularity, segment.start, duration);
                });
        }

        SetPhonemeSegmentWeight(slot, weight) => {
            state
                .phonemetrack
                .phonemes_mut()
                .iter_mut()
                .skip(slot)
                .take(1)
                .for_each(|segment| segment.weight = weight);
        }
    }
    result
}
// ----------------------------------------------------------------------------
use super::player;
use super::timeline;

// state
use super::{EditableData, Settings};

// actions
use super::Action as EditorAction;
use gui::Action;

// misc
use super::clamp_ms;
// ----------------------------------------------------------------------------
