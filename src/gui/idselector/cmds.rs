// ----------------------------------------------------------------------------
//
// gui::cmds - more complex actions with (possible) side effects
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(in gui) fn init_selector(
    selection: Selection,
    state: &mut IdSelectorState,
    player: &mut player::Player,
) -> Result<(), String> {
    let mut dataprovider = DataProvider::new(selection.audiofile());

    dataprovider.load()?;
    let audiodata = dataprovider.get_rawaudio(player.playback_samplerate(), false)?;

    if let Err(msg) = player.set_data(audiodata) {
        error!("player: {}", msg);
    }

    let audiofile = PathBuf::from(selection.audiofile());

    // -- set the filter to prefix of audiofile and search the lines for the string
    let mut filter_preset = audiofile
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "".to_string())
        // use only a small prefix because the longer the search string the more
        // probable it won't match anything (e.g. because of typos in filename)
        .chars()
        .take(10)
        .collect::<String>();

    filter_preset = filter_preset.replace(['.', '_'], " ");

    state.search_filter = input::TextField::new("##searchfilter", Some(filter_preset.as_str()))
        .set_label(SEARCHFILTER_LABEL)
        .set_width(SEARCHFILTER_INPUT_WIDTH);

    refresh_search_result(state, filter_preset)?;

    // -- prepare some information to show
    let audiofile = audiofile
        .file_name()
        .ok_or_else(|| "failed to extract filename".to_string())?
        .to_string_lossy();

    // shorten too long filenames
    let audiofile = if audiofile.len() > 70 {
        ImString::new(format!(
            "{}...",
            audiofile.chars().take(70).collect::<String>()
        ))
    } else {
        ImString::new(audiofile)
    };

    state.selection = Some(SelectedAudio {
        audiofile,
        duration: dataprovider.get_audio_duration(),
        id: selection.id(),
    });
    state.opened = true;
    Ok(())
}
// ----------------------------------------------------------------------------
pub(super) fn refresh_search_result(
    state: &mut IdSelectorState,
    filterstring: String,
) -> Result<(), String> {
    let mut search_result = Vec::with_capacity(MAX_SEARCH_RESULTS);
    state.search_capped = false;

    if let Some(line_provider) = &state.strings_provider {
        let filter_lc = filterstring.to_lowercase();

        let mut hits = 0;
        for (id, line) in line_provider.get_all_lines_lowercased() {
            if line.contains(&filter_lc) {
                if hits > MAX_SEARCH_RESULTS {
                    state.search_capped = true;
                    break;
                }
                hits += 1;
                let line = line_provider.get_line(*id)?;
                search_result.push((*id, ImString::new(format!("[{:>10}] {}", id, line))));
            }
        }
    }
    state
        .search_filter
        .set_value(FieldValue::Str(&filterstring))?;
    state.search_result = search_result;
    state.selected_slot = -1;
    state.selected_text = None;

    Ok(())
}
// ----------------------------------------------------------------------------
//
// ----------------------------------------------------------------------------
use std::path::PathBuf;

use imgui::ImString;
use imgui_controls::input;
use imgui_controls::input::{Field, FieldValue};

// state
use super::Selection;
use super::{IdSelectorState, SelectedAudio};

// utils
use gui::player;
use {DataProvider, StringsProvider};

// misc
use super::{MAX_SEARCH_RESULTS, SEARCHFILTER_INPUT_WIDTH, SEARCHFILTER_LABEL};
// ----------------------------------------------------------------------------
