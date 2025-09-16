//
// audio processing queue
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct ProcessingQueue {
    last_usedid: usize,
    tasks: Vec<Task>,
    finished: usize,
    failed: usize,
    unassigned: usize,
}
// ----------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq)]
pub enum ProcessingState {
    UnassignedId,
    Waiting,
    Processing,
    Finished,
    Error(String),
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub struct Task {
    id: TaskId,
    lineid: u32,
    audiofile: String,
    phonemefile: Option<String>,
    renaming_required: bool,
    full_rename: bool,
    state: ProcessingState,
    quality: QualityAssessment,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub struct TaskId(usize);
// ----------------------------------------------------------------------------
#[derive(Copy, Clone)]
pub enum TaskOperation {
    Rename(bool),
    Extract,
}
// ----------------------------------------------------------------------------
pub struct TaskData {
    id: TaskId,
    lineid: u32,
    audiofile: String,
    operation: TaskOperation,
}
// ----------------------------------------------------------------------------
pub enum TaskResult {
    Renamed(TaskId, String),
    Finished(TaskId, String),
    Error(TaskId, String),
}
// ----------------------------------------------------------------------------
impl Task {
    // ------------------------------------------------------------------------
    #[inline]
    pub fn id(&self) -> TaskId {
        TaskId(self.id.0)
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn lineid(&self) -> u32 {
        self.lineid
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn audiofile(&self) -> &str {
        &self.audiofile
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn phonemefile(&self) -> Option<&String> {
        self.phonemefile.as_ref()
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn state(&self) -> &ProcessingState {
        &self.state
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn quality_assessment(&self) -> &QualityAssessment {
        &self.quality
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub(super) fn set_quality_assessment(&mut self, quality: QualityAssessment) {
        self.quality = quality;
    }
    // ------------------------------------------------------------------------
    fn start(&mut self) -> TaskData {
        assert_ne!(self.state, ProcessingState::UnassignedId);

        self.state = ProcessingState::Processing;
        TaskData {
            id: TaskId(self.id.0),
            lineid: self.lineid,
            audiofile: self.audiofile.clone(),
            operation: if self.renaming_required {
                TaskOperation::Rename(self.full_rename)
            } else {
                TaskOperation::Extract
            },
        }
    }
    // ------------------------------------------------------------------------
    fn set_result(&mut self, result: TaskResult) -> Result<&Self, String> {
        assert_eq!(self.id.0, result.id().0);
        match self.state {
            ProcessingState::Processing => {
                self.state = match result {
                    TaskResult::Error(_, err) => ProcessingState::Error(err),
                    TaskResult::Renamed(_, new_audiofile) => {
                        self.renaming_required = false;
                        self.audiofile = new_audiofile;
                        if let Some(file) = self.phonemefile.as_ref() {
                            self.quality = ::phonemes::load(self.lineid, file)
                                .map(|track| track.assessed_quality())
                                .unwrap_or(QualityAssessment::Unknown);

                            ProcessingState::Finished
                        } else {
                            ProcessingState::Waiting
                        }
                    }
                    TaskResult::Finished(_, phoneme_file) => {
                        self.quality = ::phonemes::load(self.lineid, &phoneme_file)
                            .map(|track| track.assessed_quality())
                            .unwrap_or(QualityAssessment::Unknown);

                        self.phonemefile = Some(phoneme_file);
                        ProcessingState::Finished
                    }
                };
                Ok(self)
            }
            _ => Err(String::from("setting result valid only in processed state")),
        }
    }
    // ------------------------------------------------------------------------
}

// ----------------------------------------------------------------------------
impl TaskData {
    // ------------------------------------------------------------------------
    #[inline]
    pub fn lineid(&self) -> u32 {
        self.lineid
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn audiofile(&self) -> &str {
        &self.audiofile
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn operation(&self) -> TaskOperation {
        self.operation
    }
    // ------------------------------------------------------------------------
    pub fn set_error<T: Into<String>>(self, error: T) -> TaskResult {
        TaskResult::Error(self.id, error.into())
    }
    // ------------------------------------------------------------------------
    pub fn set_audiofile<T: Into<String>>(self, newname: T) -> TaskResult {
        TaskResult::Renamed(self.id, newname.into())
    }
    // ------------------------------------------------------------------------
    pub fn set_phonemefile<T: Into<String>>(self, file: T) -> TaskResult {
        TaskResult::Finished(self.id, file.into())
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl TaskResult {
    // ------------------------------------------------------------------------
    #[inline]
    fn id(&self) -> &TaskId {
        match *self {
            TaskResult::Renamed(ref id, _) => id,
            TaskResult::Finished(ref id, _) => id,
            TaskResult::Error(ref id, _) => id,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::collections::HashMap;
use std::path::PathBuf;
use std::slice::Iter;

use super::file_scanner::FileInfo;
use super::phonemes::QualityAssessment;
// ----------------------------------------------------------------------------
impl ProcessingQueue {
    // ------------------------------------------------------------------------
    pub fn new_from_directory<P: Into<PathBuf>>(
        path: P,
        force_rename: bool,
    ) -> Result<Self, String> {
        let mut queue = Self::default();
        queue.init_from_directory(path, force_rename)?;
        Ok(queue)
    }
    // ------------------------------------------------------------------------
    pub fn clear(&mut self) {
        self.tasks.clear();
        self.failed = 0;
        self.finished = 0;
        self.unassigned = 0;
    }
    // ------------------------------------------------------------------------
    pub fn init_from_directory<P: Into<PathBuf>>(
        &mut self,
        path: P,
        force_rename: bool,
    ) -> Result<(), String> {
        let path = path.into();

        // scan for supported input: audiofiles and phonemefiles
        let mut scanner = super::file_scanner::FilesScanner::new(path)?;

        let mut audio = HashMap::new();
        let mut phonemes = HashMap::new();
        let mut unassigned = Vec::new();

        for file in scanner.scan()? {
            #[allow(clippy::map_entry)]
            match file {
                FileInfo::UnlinkedAudio(ref filepath) => {
                    info!("found unlinked audiofile: {}", filepath);
                    unassigned.push(filepath.to_owned());
                }
                FileInfo::Audio(id, ref filepath, duration) => {
                    if audio.contains_key(&id) {
                        warn!(
                            "found duplicate audiofile for id [{}]: {}. skipping... ",
                            id, filepath
                        );
                    } else {
                        audio.insert(id, (filepath.to_owned(), duration.is_some()));
                    }
                }
                FileInfo::Phonemes(id, ref filepath) => {
                    if phonemes.contains_key(&id) {
                        warn!(
                            "found duplicate phoneme file for id [{}]: {}. skipping...",
                            id, filepath
                        );
                    } else {
                        phonemes.insert(id, filepath.to_owned());
                    }
                }
            }
        }

        self.tasks.clear();
        self.failed = 0;
        self.finished = 0;
        self.unassigned = unassigned.len();

        for (lineid, (audiofile, has_duration)) in audio.drain() {
            let entry = match phonemes.remove(&lineid) {
                Some(phonemes) => {
                    let (state, renaming_required) = if has_duration && !force_rename {
                        self.finished += 1;
                        (ProcessingState::Finished, false)
                    } else {
                        (ProcessingState::Waiting, true)
                    };
                    let quality = ::phonemes::load(lineid, &phonemes)
                        .map(|track| track.assessed_quality())
                        .unwrap_or(QualityAssessment::Unknown);

                    Task {
                        id: self.next_taskid(),
                        lineid,
                        audiofile,
                        phonemefile: Some(phonemes),
                        renaming_required,
                        full_rename: force_rename,
                        state,
                        quality,
                    }
                }
                None => Task {
                    id: self.next_taskid(),
                    lineid,
                    audiofile,
                    phonemefile: None,
                    renaming_required: !has_duration || force_rename,
                    full_rename: force_rename,
                    state: ProcessingState::Waiting,
                    quality: QualityAssessment::Unknown,
                },
            };
            self.tasks.push(entry);
        }

        // add unassigned with constant lineid
        for audiofile in unassigned.drain(..) {
            let id = self.next_taskid();
            self.tasks.push(Task {
                id,
                lineid: 0,
                audiofile,
                phonemefile: None,
                renaming_required: true,
                full_rename: false,
                state: ProcessingState::UnassignedId,
                quality: QualityAssessment::Unknown,
            });
        }

        // deterministic ordering by id prefix
        self.tasks.sort_by(|a, b| a.lineid.cmp(&b.lineid));

        //TODO extract initial stats (waiting/done) to enable final stat (delta)
        Ok(())
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn next_taskid(&mut self) -> TaskId {
        self.last_usedid += 1;
        TaskId(self.last_usedid)
    }
    // ------------------------------------------------------------------------
    pub fn get(&self, slot: usize) -> Option<&Task> {
        self.tasks.get(slot)
    }
    // ------------------------------------------------------------------------
    pub fn get_mut(&mut self, slot: usize) -> Option<&mut Task> {
        self.tasks.get_mut(slot)
    }
    // ------------------------------------------------------------------------
    pub fn iter(&self) -> Iter<Task> {
        self.tasks.iter()
    }
    // ------------------------------------------------------------------------
    pub fn contains_task_by_lineid(&self, lineid: u32) -> bool {
        for task in &self.tasks {
            if task.lineid == lineid {
                return true;
            }
        }
        false
    }
    // ------------------------------------------------------------------------
    pub fn remove_task(&mut self, taskid: TaskId) -> Result<Task, String> {
        use self::ProcessingState::*;

        if let Some((slot, _)) = self
            .tasks
            .iter()
            .enumerate()
            .find(|(_, t)| t.id.0 == taskid.0)
        {
            let task = self.tasks.remove(slot);
            match task.state {
                UnassignedId => self.unassigned -= 1,
                Finished => self.finished -= 1,
                Error(_) => self.failed -= 1,
                Processing => return Err("cannot remove task in processing state".to_string()),
                Waiting => {}
            }
            Ok(task)
        } else {
            Err(format!("task ({}) not found.", taskid.0))
        }
    }
    // ------------------------------------------------------------------------
    pub fn add_audiofile(&mut self, lineid: u32, file: &str) -> Result<(), String> {
        // check for dupes
        if self.contains_task_by_lineid(lineid) {
            Err(format!(
                "found duplicate audiofile for id [{}]. audiofile not added to queue",
                lineid
            ))
        } else {
            let id = self.next_taskid();
            self.tasks.push(Task {
                id,
                lineid,
                audiofile: file.to_owned(),
                phonemefile: None,
                renaming_required: false,
                full_rename: false,
                state: ProcessingState::Waiting,
                quality: QualityAssessment::Unknown,
            });

            // deterministic ordering by id prefix
            self.tasks.sort_by(|a, b| a.lineid.cmp(&b.lineid));
            Ok(())
        }
    }
    // ------------------------------------------------------------------------
    pub fn take_waiting(&mut self) -> Option<TaskData> {
        if self.failed + self.finished + self.unassigned < self.tasks.len() {
            for task in &mut self.tasks {
                if let ProcessingState::Waiting = task.state {
                    return Some(task.start());
                }
            }
        }
        None
    }
    // ------------------------------------------------------------------------
    pub fn force_renaming(&mut self) {
        for task in self.tasks.iter_mut() {
            match task.state {
                ProcessingState::Processing
                | ProcessingState::UnassignedId
                | ProcessingState::Error(_) => {}
                ProcessingState::Finished => {
                    // subtract stats
                    self.finished -= 1;

                    task.renaming_required = true;
                    task.full_rename = true;
                    task.state = ProcessingState::Waiting;
                }
                _ => {
                    task.renaming_required = true;
                    task.full_rename = true;
                    task.state = ProcessingState::Waiting;
                }
            }
        }
    }
    // ------------------------------------------------------------------------
    pub fn update_taskresult(&mut self, result: TaskResult) -> Result<&Task, String> {
        match self.tasks.iter_mut().find(|t| t.id.0 == result.id().0) {
            Some(processed_task) => {
                let task = processed_task.set_result(result)?;
                match *task.state() {
                    ProcessingState::Finished => self.finished += 1,
                    ProcessingState::Error(_) => self.failed += 1,
                    _ => {}
                }
                Ok(task)
            }
            None => Err(format!("update of unknown task: {}", result.id().0)),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
