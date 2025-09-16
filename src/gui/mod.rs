//
// gui for interactive phoneme extraction
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------

// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
mod support;

mod actors;
mod editor;
mod hotkeys;
mod idselector;
mod player;
mod queue;
mod settings;

mod phonemes;
mod worker;

mod help;

mod cmds;
mod update;
mod utils;
mod view;
use std::path::{Path, PathBuf};

use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use imgui::Ui;
use logger::LevelFilter;

use imgui_controls::filebrowser;
use imgui_support::windows;

use imgui_support::actions;
use imgui_support::actions::ActionState;

use self::utils::{ScreenSpaceManager, UiArea};

const CLEAR_COLOR: [f32; 4] = [114.0 / 255.0, 144.0 / 255.0, 154.0 / 255.0, 1.0];
// ----------------------------------------------------------------------------
// state
// ----------------------------------------------------------------------------
struct State {
    windows: WindowState,
    audioqueue: queue::AudioQueue,

    editor_data: editor::EditableData,
    lineid_selector: idselector::IdSelectorState,
    actor_mapping: Arc<actors::ActorMapping>,
    player: player::Player,

    settings: settings::Settings,

    fileio: filebrowser::FileChooserState,
    current_dir: PathBuf,

    hotkeys: hotkeys::HotkeyState,
    help: help::HelpSystem,
}
// ----------------------------------------------------------------------------
#[derive(Default)]
struct WindowState {
    show_help: bool,
    show_about: bool,
    show_filebrowser: bool,
    info: windows::InfoWindow,
    error: windows::ErrorWindow,
}
// ----------------------------------------------------------------------------
// actions
// ----------------------------------------------------------------------------
#[derive(Debug)]
struct IdAssignmentActionData {
    id: queue::SelectionId,
    lineid: u32,
    duration: f32,
    actor: Option<String>,
    text: String,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
enum Action {
    GuardModifiedData,
    Confirm(String, Vec<Action>, Vec<Action>),
    SaveCurrent,
    CloseDir,
    ChangeDir(PathBuf),
    OnSelectEntry(queue::SelectedSlot),
    SelectEntry(queue::SelectedSlot),
    AssignId(IdAssignmentActionData),
    UpdateActorMapping(String, String),
    Menu(MenuSelection),
    Editor(editor::Action),
    OpenFileBrowser,
    RenameAllFiles,
    FileBrowser(filebrowser::Selection),
    LineIdSelector(idselector::Action),
    Quit,
    PlayAudio,
    StopAudio,
    HotkeyPressed(hotkeys::Hotkey),
    ResizeQueueHeight(f32),
}
// ----------------------------------------------------------------------------
type ActionSequence = actions::Sequence<Action>;
// ----------------------------------------------------------------------------
#[derive(Debug)]
enum MenuSelection {
    ShowHelp,
    ShowAbout,
    RenameAllFiles,
    ResizeQueueHeight(f32),
    LoadFile,
    SaveFile,
    CloseDirectory,
    Quit,
}
// ----------------------------------------------------------------------------
// workerpool
// ----------------------------------------------------------------------------
type WorkerThread = (thread::JoinHandle<()>, mpsc::Sender<()>);

struct WorkerInitParams {
    error_channel: mpsc::Sender<String>,
    tasks: Arc<queue::WorkerQueue>,
    language: String,
    stringsfile: Option<PathBuf>,
    actor_mappingsfile: Option<PathBuf>,
    datadir: PathBuf,
    loglevel: LevelFilter,
}
// ----------------------------------------------------------------------------
struct WorkerThreadPool {
    max_count: usize,
    threads: Vec<WorkerThread>,
    params: WorkerInitParams,
}
// ----------------------------------------------------------------------------
// main loop
// ----------------------------------------------------------------------------
#[allow(clippy::too_many_arguments)]
pub fn run(
    app_name: String,
    helpfile: &Path,
    startdir: Option<PathBuf>,
    language: String,
    stringsfile: Option<PathBuf>,
    actor_mappingsfile: Option<PathBuf>,
    datadir: PathBuf,
    workerthreads: usize,
    loglevel: LevelFilter,
) -> Result<(), String> {
    let mut actions = ActionState::default();
    let mut state = State::new(player::init()?);
    let mut dont_quit = true;

    // ignore missing help file (help won't be available but everthing else works)
    state.help.load(helpfile).ok();

    if let Some(startdir) = startdir {
        actions.push(Action::ChangeDir(startdir));
    }

    // -- user error info comm channel
    let (error_channel, error_receiver) = mpsc::channel::<String>();

    // -- scan for supported (pocketsphinx) languages
    if let Err(err) = state.settings.detect_language_support(&datadir) {
        error_channel
            .send(err)
            .unwrap_or_else(|e| error!("language support error: error info send failed ({e})"));
    }
    state.settings.set_language(&language);

    // -- prepare workerthreads
    let mut worker_threadpool = WorkerThreadPool {
        max_count: workerthreads,
        threads: Vec::new(),
        params: WorkerInitParams {
            error_channel: error_channel.clone(),
            tasks: state.audioqueue.tasks(),
            language: state.settings.selected_language().to_owned(),
            stringsfile,
            actor_mappingsfile,
            datadir: datadir.clone(),
            loglevel,
        },
    };

    support::run(app_name, datadir, CLEAR_COLOR, |ui, fonts, win_close| {
        if win_close {
            actions.include(ima_prio_seq![Action::GuardModifiedData, Action::Quit]);
        }

        let screenspace =
            ScreenSpaceManager::new(ui.imgui().display_size(), state.audioqueue.area_height());

        actions.filter_push(view::menu::render(ui, &screenspace, &state));

        // -- main content
        actions.filter_push(view::show_editor(ui, &screenspace, fonts, &state));

        // -- additional global interaction windows
        actions.filter_push(show_filebrowser(ui, &mut state));
        actions.filter_push(view::show_lineid_selector(
            ui,
            fonts,
            &mut state.lineid_selector,
        ));

        actions.filter_push(hotkeys::check_pressed(ui, &mut state.hotkeys));

        // must be called AFTER view drawing and BEFORE action perform since it
        // cleans some one-time state flags (e.g. context menu opening)
        actions.filter_push(running_actions_tick(&mut state));

        // perform "blocks" on interactive action (e.g. confirma dialog)
        // "blocks" meaning: redraws associated ui until some user interaction
        // takes place and continues with next action in specified queue afterwards
        while let Some(action) = actions::perform(ui, &mut actions) {
            match handle_action(action, &mut actions, &mut state, &mut worker_threadpool) {
                Ok(do_quit) => dont_quit = !do_quit,
                Err(error_msg) => {
                    actions.clear();
                    view::auxiliary::set_error(&mut state.windows.error, &error_msg);
                }
            }
        }

        // generic error handling from error channel (e.g. workerthread)
        if let Ok(msg) = error_receiver.try_recv() {
            view::auxiliary::set_error(&mut state.windows.error, &msg);
        }

        // auxiliary windows contain error and info popups -> draw after cmds to
        // show update/cmd execution errors
        view::auxiliary::show_windows(ui, &mut state.windows, &state.help);

        dont_quit
    })
}
// ----------------------------------------------------------------------------
// main loop functions for better readablity
// ----------------------------------------------------------------------------
#[inline]
fn running_actions_tick(state: &mut State) -> Option<Action> {
    state.audioqueue.refresh_captions();
    editor::running_actions_tick(&state.settings, &mut state.editor_data).map(Into::into)
}
// ----------------------------------------------------------------------------
#[inline]
fn handle_action(
    action: Action,
    actions: &mut ActionState<Action>,
    state: &mut State,
    worker_pool: &mut WorkerThreadPool,
) -> Result<bool, String> {
    match action {
        Action::HotkeyPressed(hotkey) => {
            if state.editor_data.is_available() {
                actions.filter_push(update::handle_hotkey(hotkey, state));
            } else if state.lineid_selector.is_opened() {
                actions.filter_push(idselector::handle_hotkey(
                    hotkey,
                    &mut state.lineid_selector,
                ));
            }
            Ok(())
        }

        Action::PlayAudio => state.player.play(),
        Action::StopAudio => state.player.stop(),

        Action::Editor(action) => {
            actions.filter_push(editor::handle_action(
                action,
                &mut state.editor_data,
                &mut state.settings,
                &mut state.player,
            ));
            Ok(())
        }

        Action::LineIdSelector(action) => {
            actions.filter_include(idselector::handle_action(
                action,
                &mut state.lineid_selector,
                &mut state.player,
            )?);
            Ok(())
        }

        Action::AssignId(data) => cmds::assign_lineid(data, &mut state.audioqueue),

        Action::UpdateActorMapping(actor, mapped_to) => {
            state.actor_mapping.update(&actor, &mapped_to);
            Ok(())
        }

        Action::GuardModifiedData => {
            if state.editor_data.changed() || state.actor_mapping.changed() {
                actions.include(ima_prio_seq![Action::Confirm(
                    String::from("Data changed. Save modified phoneme timings?"),
                    vec![Action::SaveCurrent],
                    vec![],
                )]);
            }
            Ok(())
        }
        Action::RenameAllFiles => {
            cmds::force_renaming(&mut state.audioqueue)?;
            Ok(())
        }

        Action::Confirm(text, yes_actions, no_actions) => {
            actions.set_interactive(text, yes_actions, no_actions);
            Ok(())
        }

        Action::SaveCurrent => cmds::save_phoneme_track(&state.current_dir, &mut state.editor_data)
            .and_then(|quality_assesment| {
                state
                    .audioqueue
                    .update_quality_on_selected(quality_assesment)
            })
            .and_then(|_| state.actor_mapping.store_updated()),

        Action::OnSelectEntry(new_entry) => {
            actions.include(ima_seq![
                Action::GuardModifiedData,
                Action::SelectEntry(new_entry)
            ]);
            Ok(())
        }

        Action::SelectEntry(new_entry) => match state.audioqueue.select(new_entry) {
            Some(entry) => {
                if entry.has_lineid() {
                    state.editor_data.reset();
                    queue::load_data(&entry, &mut state.editor_data, &mut state.player).map(|_| {
                        let selected_mapping =
                            if let Some(actor) = state.editor_data.unmodified().actor() {
                                state.actor_mapping.resolve(actor)
                            } else {
                                String::default()
                            };
                        state.settings.set_actor(&selected_mapping);
                    })
                } else {
                    state.editor_data.reset();

                    idselector::cmds::init_selector(
                        entry,
                        &mut state.lineid_selector,
                        &mut state.player,
                    )
                }
            }
            None => Err("could not select queue entry".into()),
        },

        // -- filebrowser
        Action::OpenFileBrowser => {
            state.windows.show_filebrowser = true;
            Ok(())
        }

        Action::FileBrowser(selection) => {
            actions.include(update::handle_filebrowser_selection(selection, state));
            Ok(())
        }

        Action::CloseDir => Ok(cmds::handle_close_dir(state, worker_pool)?),
        Action::ChangeDir(new_dir) => Ok(cmds::handle_change_dir(new_dir, state, worker_pool)?),

        Action::Menu(selection) => {
            actions.filter_include(update::handle_menu_selection(selection, state));
            Ok(())
        }

        // -- misc
        Action::ResizeQueueHeight(new_height) => {
            state.audioqueue.set_area_height(new_height);
            Ok(())
        }

        Action::Quit => return Ok(true),
    }
    // default do not quit
    .map(|_| false)
}
// ----------------------------------------------------------------------------
#[inline]
fn show_filebrowser(ui: &Ui<'_>, state: &mut State) -> Option<Action> {
    if state.windows.show_filebrowser {
        filebrowser::show(ui, &mut state.fileio, &mut state.windows.show_filebrowser)
            .map(Into::into)
    } else {
        None
    }
}
// ----------------------------------------------------------------------------
impl State {
    // ------------------------------------------------------------------------
    fn new(player: player::Player) -> State {
        State {
            windows: WindowState::default(),
            audioqueue: queue::AudioQueue::default(),

            settings: settings::Settings::default(),
            editor_data: editor::EditableData::new(),
            lineid_selector: idselector::IdSelectorState::new(),
            actor_mapping: Arc::new(actors::ActorMapping::default()),
            player,

            fileio: filebrowser::FileChooserState::new(&PathBuf::from(".")),
            current_dir: PathBuf::from("."),

            hotkeys: hotkeys::HotkeyState::default(),
            help: help::HelpSystem::default(),
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// converter
// ----------------------------------------------------------------------------
impl From<MenuSelection> for Action {
    fn from(selection: MenuSelection) -> Action {
        Action::Menu(selection)
    }
}
// ----------------------------------------------------------------------------
impl From<editor::Action> for Action {
    fn from(action: editor::Action) -> Action {
        Action::Editor(action)
    }
}
// ----------------------------------------------------------------------------
impl From<idselector::Action> for Action {
    fn from(action: idselector::Action) -> Action {
        Action::LineIdSelector(action)
    }
}
// ----------------------------------------------------------------------------
impl From<filebrowser::Selection> for Action {
    fn from(selection: filebrowser::Selection) -> Action {
        Action::FileBrowser(selection)
    }
}
// ----------------------------------------------------------------------------
