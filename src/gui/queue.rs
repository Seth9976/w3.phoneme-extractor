//
// gui: audio queue and selection
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct AudioQueue {
    selected: Option<SelectedSlot>,
    captions: Vec<ImString>,

    tasks: Arc<WorkerQueue>,
    info: String,
    height: Option<f32>,
}
// ----------------------------------------------------------------------------
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct SelectedSlot(usize);

pub(in gui) use queue::TaskId as SelectionId;
// ----------------------------------------------------------------------------
pub struct Selection {
    id: SelectionId,
    lineid: Option<u32>,
    audiofile: String,
    phonemefile: String,
}
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct WorkerQueue {
    dirty: AtomicBool,
    queue: Mutex<queue::ProcessingQueue>,
}
// ----------------------------------------------------------------------------
pub(super) fn load_data(
    entry: &Selection,
    data: &mut editor::EditableData,
    player: &mut player::Player,
) -> Result<(), String> {
    data.reset();

    let mut dataprovider = DataProvider::new(&entry.audiofile);

    dataprovider.load()?;
    // TODO move into dataprovider?
    let phonemes = ::phonemes::load(
        entry.lineid.expect("missing line id in selected entry"),
        &entry.phonemefile,
    )?;
    let audiodata = dataprovider.get_rawaudio(player.playback_samplerate(), false)?;

    data.set_audio(audiodata.clone(), player.playback_samplerate());
    data.set_phonemetrack((&phonemes).into());

    if let Err(msg) = player.set_data(audiodata) {
        error!("player: {}", msg);
    }

    Ok(())
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui::ImString;

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use queue;
use DataProvider;

use crate::phonemes::QualityAssessment;

use super::editor;
use super::player;
// ----------------------------------------------------------------------------
#[derive(Default)]
struct QueueStats {
    waiting: u32,
    processing: u32,
    error: u32,
    done: u32,
    total: u32,
}
// ----------------------------------------------------------------------------
impl AudioQueue {
    // ------------------------------------------------------------------------
    pub(in gui) fn tasks(&self) -> Arc<WorkerQueue> {
        self.tasks.clone()
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn clear(&mut self) -> Result<(), String> {
        match self.tasks.queue.lock() {
            Ok(mut queue) => {
                queue.clear();
                let (stats, captions) = Self::update_captions(&queue);
                self.captions = captions;
                self.info = format!("{}", stats);
                self.selected = None;
                Ok(())
            }
            Err(_) => Err(String::from("could not acquire lock on processing queue")),
        }
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn init_from_directory(&mut self, path: &str) -> Result<(), String> {
        match self.tasks.queue.lock() {
            Ok(mut queue) => {
                queue.init_from_directory(path, false)?;
                let (stats, captions) = Self::update_captions(&queue);
                self.captions = captions;
                self.info = format!("{}", stats);
                self.selected = None;
                Ok(())
            }
            Err(_) => Err(String::from("could not acquire lock on processing queue")),
        }
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn contains_task_by_lineid(&self, lineid: u32) -> bool {
        match self.tasks.queue.lock() {
            Ok(queue) => queue.contains_task_by_lineid(lineid),
            Err(_) => {
                warn!("could not acquire lock on processing queue");
                true
            }
        }
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn remove_task(&mut self, id: SelectionId) -> Result<queue::Task, String> {
        match self.tasks.queue.lock() {
            Ok(mut queue) => {
                self.tasks.dirty.store(true, Ordering::SeqCst);
                self.selected = None;
                queue.remove_task(id)
            }
            Err(_) => Err(String::from("could not acquire lock on processing queue")),
        }
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn add_audiofile(&mut self, lineid: u32, file: &str) -> Result<(), String> {
        match self.tasks.queue.lock() {
            Ok(mut queue) => {
                self.tasks.dirty.store(true, Ordering::SeqCst);
                self.selected = None;
                queue.add_audiofile(lineid, file)
            }
            Err(_) => Err(String::from("could not acquire lock on processing queue")),
        }
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn update_quality_on_selected(&mut self, quality: QualityAssessment) -> Result<(), String> {
        match self.tasks.queue.lock() {
            Ok(mut queue) => {
                self.tasks.dirty.store(true, Ordering::SeqCst);
                if let Some(task) = self.selected.and_then(|slot| queue.get_mut(slot.0)) {
                    task.set_quality_assessment(quality);
                }
                Ok(())
            }
            Err(_) => Err(String::from("could not acquire lock on processing queue")),
        }
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn refresh_captions(&mut self) {
        if self.tasks.has_changed() {
            match self.tasks.queue.lock() {
                Ok(queue) => {
                    let (stats, captions) = Self::update_captions(&queue);
                    self.captions = captions;
                    self.info = format!("{}", stats);
                }
                Err(_) => error!("could not acquire lock on processing queue"),
            }
        }
    }
    // ------------------------------------------------------------------------
    fn update_captions(tasks: &queue::ProcessingQueue) -> (QueueStats, Vec<ImString>) {
        use queue::ProcessingState::UnassignedId;

        let mut captions = Vec::new();
        let mut stats = QueueStats::default();

        for (slot, task) in tasks.iter().enumerate() {
            match task.state() {
                UnassignedId => {
                    captions.push(ImString::new(format!(
                        "{:>3}. [          ] [{}] [           ] {}",
                        slot + 1,
                        task.state(),
                        task.audiofile()
                    )));
                }
                _ => {
                    captions.push(ImString::new(format!(
                        "{:>3}. [{:>10}] [{}] [{}] {}",
                        slot + 1,
                        task.lineid(),
                        task.state(),
                        task.quality_assessment(),
                        task.audiofile()
                    )));
                }
            }
            stats.add(task);
        }

        (stats, captions)
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn force_renaming(&mut self) -> Result<(), String> {
        match self.tasks.queue.lock() {
            Ok(mut queue) => {
                self.tasks.dirty.store(true, Ordering::SeqCst);
                self.selected = None;
                queue.force_renaming();
                Ok(())
            }
            Err(_) => Err(String::from("could not acquire lock on processing queue")),
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub(in gui) fn info(&self) -> &str {
        &self.info
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn selected(&self) -> Option<SelectedSlot> {
        self.selected
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn captions(&self) -> impl Iterator<Item = &ImString> {
        self.captions.iter()
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn is_empty(&self) -> bool {
        self.captions.is_empty()
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn is_selectable(&self, slot: usize) -> Option<SelectedSlot> {
        use queue::ProcessingState::*;

        let entry = SelectedSlot(slot);

        if self.selected != Some(entry) {
            match self.tasks.queue.lock() {
                Ok(queue) => {
                    if let Some(task) = queue.get(entry.0) {
                        match task.state() {
                            UnassignedId | Finished => return Some(entry),
                            _ => {}
                        }
                    }
                }
                Err(_) => error!("could not acquire lock on processing queue"),
            }
        }
        None
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn select(&mut self, entry: SelectedSlot) -> Option<Selection> {
        use queue::ProcessingState::*;

        if self.selected != Some(entry) {
            match self.tasks.queue.lock() {
                Ok(queue) => {
                    if let Some(task) = queue.get(entry.0) {
                        match task.state() {
                            UnassignedId | Finished => {
                                self.selected = Some(entry);
                                return Some(task.into());
                            }
                            _ => {}
                        }
                    }
                }
                Err(_) => error!("could not acquire lock on processing queue"),
            }
        }
        None
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn set_area_height(&mut self, new_height: f32) {
        self.height = Some(new_height);
    }
    // ------------------------------------------------------------------------
    pub(in gui) fn area_height(&self) -> Option<f32> {
        self.height
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
use std::sync::atomic::Ordering;

impl WorkerQueue {
    // ------------------------------------------------------------------------
    fn has_changed(&self) -> bool {
        self.dirty.swap(false, Ordering::SeqCst)
    }
    // ------------------------------------------------------------------------
    pub fn take_waiting(&self) -> Option<queue::TaskData> {
        match self.queue.lock() {
            Ok(mut queue) => {
                self.dirty.store(true, Ordering::SeqCst);
                queue.take_waiting()
            }
            Err(_) => {
                error!("could not acquire lock on processing queue");
                None
            }
        }
    }
    // ------------------------------------------------------------------------
    pub fn update_taskresult(&self, result: queue::TaskResult) {
        match self.queue.lock() {
            Ok(mut queue) => {
                queue.update_taskresult(result).ok();
                self.dirty.store(true, Ordering::SeqCst);
            }
            Err(_) => error!("could not acquire lock on processing queue"),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl QueueStats {
    // ------------------------------------------------------------------------
    fn add(&mut self, task: &queue::Task) {
        use queue::ProcessingState;

        match *task.state() {
            ProcessingState::UnassignedId => self.waiting += 1,
            ProcessingState::Waiting => self.waiting += 1,
            ProcessingState::Processing => self.processing += 1,
            ProcessingState::Finished => self.done += 1,
            ProcessingState::Error(_) => self.error += 1,
        }
        self.total += 1;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl Selection {
    // ------------------------------------------------------------------------
    pub(super) fn has_lineid(&self) -> bool {
        self.lineid.is_some()
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub(super) fn audiofile(&self) -> &str {
        &self.audiofile
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub(super) fn id(self) -> SelectionId {
        self.id
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
use std::ops;

impl ops::Deref for SelectedSlot {
    type Target = usize;

    fn deref(&self) -> &usize {
        &self.0
    }
}
// ----------------------------------------------------------------------------
impl From<&queue::Task> for Selection {
    // ------------------------------------------------------------------------
    fn from(task: &queue::Task) -> Selection {
        use queue::ProcessingState::UnassignedId;

        Selection {
            id: task.id(),
            lineid: match task.state() {
                UnassignedId => None,
                _ => Some(task.lineid()),
            },
            audiofile: task.audiofile().to_owned(),
            phonemefile: task
                .phonemefile()
                .map(AsRef::as_ref)
                .unwrap_or("")
                .to_owned(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
use std::fmt;

impl fmt::Display for queue::ProcessingState {
    // ------------------------------------------------------------------------
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use queue::ProcessingState::*;

        match *self {
            UnassignedId => write!(f, "missing id"),
            Waiting => write!(f, "  waiting "),
            Processing => write!(f, "processing"),
            Finished => write!(f, "   done   "),
            Error(ref err) => write!(f, "error: {}", err),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl fmt::Display for ::phonemes::QualityAssessment {
    // ------------------------------------------------------------------------
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use phonemes::QualityAssessment::*;

        match *self {
            Unknown =>           write!(f, "           "),
            Ok =>                write!(f, "     ok    "),
            NeedsCheckWarn =>    write!(f, "needs check"),
            NeedsCheckError =>   write!(f, "NEEDS CHECK"),
            EditedOk =>          write!(f, "   edited  "),
            EditedWithErrors =>  write!(f, "  *edited  "),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl fmt::Display for QueueStats {
    // ------------------------------------------------------------------------
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.waiting + self.processing > 0 {
            if self.error > 0 {
                write!(
                    f,
                    "[{}/{} processed, {} errors]",
                    self.done, self.total, self.error
                )
            } else {
                write!(f, "[{}/{} processed]", self.done, self.total)
            }
        } else if self.error > 0 {
            write!(f, "[{} errors]", self.error)
        } else {
            write!(f, "")
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
