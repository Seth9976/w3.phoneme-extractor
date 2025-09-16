//
// gui: background workerthread for processing tasks (extracting phonemes from audio)
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub(super) fn start(
    errorchannel: mpsc::Sender<String>,
    tasks: Arc<queue::WorkerQueue>,
    language: String,
    stringsfile: PathBuf,
    actor_mappings: Arc<actors::ActorMapping>,
    datadir: PathBuf,
    loglevel: LevelFilter,
) -> (thread::JoinHandle<()>, mpsc::Sender<()>) {
    let (stop_channel, stop_signal) = mpsc::channel::<()>();

    let handle = thread::spawn(move || {
        let id = thread::current().id();

        info!("> starting worker thread {:?}.", id);
        let mut stop = false;

        match ::init_phoneme_extraction(&language, &stringsfile, &datadir, loglevel) {
            Ok(mut processor) => {
                while !stop {
                    while let Some(task) = tasks.take_waiting() {
                        use text::StringsProvider;

                        info!("id {:>10}: file: {}", task.lineid(), task.audiofile());

                        // try to resolve this actorname to predefined set of available actors
                        // and keep the information to store this updated alias mapping
                        if let Some(actor) = processor.strings.get_actor(task.lineid()) {
                            actor_mappings.resolve(actor);
                        }

                        tasks.update_taskresult(processor.process(task));

                        // stop queue while processing
                        if stop_signal.try_recv().is_ok() {
                            stop = true;
                            break;
                        }
                    }

                    // stop queue while waiting for new tasks
                    if stop_signal.try_recv().is_ok() {
                        stop = true;
                    } else {
                        actor_mappings.store_updated().ok();
                        thread::sleep(Duration::from_millis(1000));
                    }
                }
            }
            Err(msg) => {
                errorchannel
                    .send(msg)
                    .unwrap_or_else(|e| error!("workerthread: send failed ({})", e));
            }
        }
        info!("> stopped worker thread {:?}.", id);
    });

    (handle, stop_channel)
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use logger::LevelFilter;

use super::actors;

use super::queue;
// ----------------------------------------------------------------------------
