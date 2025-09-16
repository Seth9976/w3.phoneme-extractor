//
// gui::cmds - more complex actions with (possible) side effects
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(super) fn handle_close_dir(
    state: &mut State,
    worker_pool: &mut WorkerThreadPool,
) -> Result<(), String> {
    state.editor_data.reset();

    // -- reinit workerthreads
    // TODO if a different language has to be used (e.g.
    // extracted from strings file or directory name)
    // the workers have to be reinitialized. since the espeak
    // wrapper/lib is not multithreaded the threads need to be
    // stopped before new worker threads can be started

    // closing directory means the currently assigned strings will not be used
    // anymore -> stop all threads
    // -> simplest solution: send stop signal and block ui thread
    for (_, stop_signal) in &worker_pool.threads {
        // ignore result
        stop_signal.send(()).ok();
    }
    for (worker, _) in worker_pool.threads.drain(..) {
        worker.join().ok();
    }

    // store any actor mappings updates
    state.actor_mapping.store_updated().ok();

    state.audioqueue.clear()?;

    // -- reset idselector
    state.lineid_selector.reset();

    state.fileio = filebrowser::FileChooserState::new(&PathBuf::from("."));
    state.current_dir = PathBuf::from(".");

    Ok(())
}
// ----------------------------------------------------------------------------
pub(super) fn handle_change_dir(
    new_dir: PathBuf,
    state: &mut State,
    worker_pool: &mut WorkerThreadPool,
) -> Result<(), String> {
    // make sure threads are already stopped
    handle_close_dir(state, worker_pool).ok();

    // -- detect language from directory
    let lang = ::extract_language_info(&new_dir).unwrap_or_else(|| "en".to_string());

    let lang = state.settings.set_language(&lang).to_owned();
    worker_pool.params.language = lang;

    let stringsfile = match worker_pool.params.stringsfile {
        Some(ref file) => Ok(file.clone()),
        None => ::search_strings_file(&new_dir),
    }?;

    // -- reinit idselector
    state
        .lineid_selector
        .init_strings_provider(&stringsfile, Some(&worker_pool.params.language))?;

    // -- reinit actormapping
    let actor_mapping_file = match &worker_pool.params.actor_mappingsfile {
        Some(file) => file.clone(),
        None => ::search_actor_mappings_file(&stringsfile)?,
    };

    let actor_mapping = Arc::new(actors::ActorMapping::load(
        &worker_pool.params.datadir,
        Some(&actor_mapping_file),
    )?);

    // -- reinit workerthreads
    // start new worker
    for _ in worker_pool.threads.len()..worker_pool.max_count {
        worker_pool.threads.push(worker::start(
            worker_pool.params.error_channel.clone(),
            worker_pool.params.tasks.clone(),
            worker_pool.params.language.clone(),
            stringsfile.clone(),
            actor_mapping.clone(),
            worker_pool.params.datadir.clone(),
            worker_pool.params.loglevel,
        ));
        thread::sleep(::std::time::Duration::from_millis(100));
    }

    state
        .audioqueue
        .init_from_directory(&new_dir.to_string_lossy())
        .unwrap();

    state.fileio = filebrowser::FileChooserState::new(&new_dir);
    state.current_dir = new_dir;
    state.actor_mapping = actor_mapping;
    state
        .settings
        .reset_actor_mappings(state.actor_mapping.clone());

    Ok(())
}
// ----------------------------------------------------------------------------
pub(super) fn save_phoneme_track(
    outputdir: &PathBuf,
    data: &mut editor::EditableData,
) -> Result<QualityAssessment, String> {
    use phonemes::{PhonemeSegment, PhonemeTrack};

    // increase version for saving
    let version = data.phonemetrack().version() + 1;
    let mut track: PhonemeTrack<PhonemeSegment> = data.phonemetrack().into();
    track.set_version(version);
    let new_assesment = track.assess_quality();
    let updated_track = (&track).into();

    ::phonemes::store(outputdir, track)
        .map(|_| data.set_as_saved(version))
        .map(|_| {
            data.set_phonemetrack(updated_track);
            new_assesment
        })
}
// ----------------------------------------------------------------------------
pub(super) fn assign_lineid(
    data: IdAssignmentActionData,
    queue: &mut queue::AudioQueue,
) -> Result<(), String> {
    use std::fs;

    if queue.contains_task_by_lineid(data.lineid) {
        Err(format!(
            "Line id assignment failed: \n\naudioqueue already contains \
             an audiofile with the lineid {}",
            data.lineid
        ))
    } else {
        let task = queue.remove_task(data.id)?;

        let old_audiofile = PathBuf::from(task.audiofile());
        let extension = old_audiofile
            .extension()
            .ok_or("failed to extract audiofile extension")?;

        let new_audiofile = format!(
            "{:0>10}[{:.4}]{}{}.{}",
            data.lineid,
            data.duration,
            ::escape_actor(&data.actor.map(|a| format!("-{a}-")).unwrap_or_default())?,
            ::escape_textline(&data.text)?,
            extension.to_string_lossy()
        );

        info!("renaming audiofile to: {}", new_audiofile);
        let new_audiofile = old_audiofile.with_file_name(new_audiofile);

        fs::rename(&old_audiofile, &new_audiofile)
            .map_err(|err| format!("failed to rename {}: {}", old_audiofile.display(), err))?;

        // potentially existing phoneme file for this lineid will be overriden
        // because the audiofile is added with a "waiting" processing state
        queue.add_audiofile(data.lineid, &new_audiofile.to_string_lossy())
    }
}
// ----------------------------------------------------------------------------
pub(super) fn force_renaming(queue: &mut queue::AudioQueue) -> Result<(), String> {
    queue.force_renaming()?;
    queue.refresh_captions();
    Ok(())
}
// ----------------------------------------------------------------------------
//
// ----------------------------------------------------------------------------
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use crate::phonemes::QualityAssessment;

use super::actors;
use super::editor;
use super::filebrowser;
use super::queue;
use super::worker;

use super::{IdAssignmentActionData, State, WorkerThreadPool};
// ----------------------------------------------------------------------------
