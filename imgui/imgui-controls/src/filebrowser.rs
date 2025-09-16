//
// gui: file / directory chooser
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub struct FileChooserState {
    input_path: ImString,
    input_path_valid: bool,

    path: ImString,
    quick_dir: Vec<ImString>,

    drives: Vec<ImString>,
    dirs: Vec<ImString>,
    files: Vec<ImString>,
}

#[derive(Debug)]
pub enum Selection {
    Dir(String),
    // File(String),
}
// ----------------------------------------------------------------------------
pub fn show(ui: &Ui<'_>, settings: &mut FileChooserState, open_window: &mut bool)
    -> Option<Selection>
{
    if *open_window {
        show_window(ui, settings, open_window)
    } else {
        None
    }
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::io;

use std::path::{Path, PathBuf};
use std::borrow::Borrow;

use imgui;
use imgui::{Ui, ImString, ImGuiCol};
// ----------------------------------------------------------------------------
fn show_window(
    ui: &Ui<'_>,
    state: &mut FileChooserState,
    opened: &mut bool,
) -> Option<Selection> {
    let mut cancel = false;
    let mut selection = None;
    ui.window(im_str!("Select directory"))
        .size((700.0, 290.0), imgui::ImGuiCond::FirstUseEver)
        // .resizable(false)
        // .movable(false)
        .opened(opened)
        .build_modal(|| {
            let mut new_path = None;

            render_path_input(ui, state, &mut new_path);
            render_quickdir(ui, state, &mut new_path);

            ui.spacing();
            ui.separator();

            if !state.drives.is_empty() {
                ui.with_item_width(40.0, || {
                    render_drive_list(ui, state, &mut new_path);
                    ui.same_line(0.0);
                });
            }

            ui.with_item_width(200.0, || {
                render_dir_list(ui, state, &mut new_path);
                ui.same_line(0.0);
            });

            ui.with_item_width(-1.0, || {
                render_file_list(ui, state, &mut new_path);
            });

            if let Some(new_path) = new_path {
                update_path(&new_path, state);
            }

            // -- buttons
            ui.separator();
            ui.spacing();
            render_buttons(ui, || {
                    selection = Some(Selection::Dir(state.path.to_str().to_owned()))
                }, || {
                    cancel = true
                });
        });

    if selection.is_some() || cancel {
        *opened = false;
    }
    selection
}
// ----------------------------------------------------------------------------
const FILEPATH_MAX_LENGTH: usize = 255;

impl Default for FileChooserState {
    fn default() -> FileChooserState {
        FileChooserState {
            input_path: ImString::with_capacity(FILEPATH_MAX_LENGTH),
            input_path_valid: false,

            path: ImString::with_capacity(FILEPATH_MAX_LENGTH),
            quick_dir: Vec::default(),

            drives: Vec::default(),
            dirs: Vec::default(),
            files: Vec::default(),
        }
    }
}
// ----------------------------------------------------------------------------
impl FileChooserState {
    pub fn new(path: &Path) -> FileChooserState {
        let mut state = FileChooserState{
            drives: scan_for_disknames(),
            ..Default::default()
        };
        update_path(path, &mut state);
        state
    }
    // ------------------------------------------------------------------------
    pub fn current_path(&self) -> PathBuf {
        PathBuf::from(AsRef::<str>::as_ref(&self.path))
    }
}
// ----------------------------------------------------------------------------
#[inline]
fn render_path_input(
    ui: &Ui<'_>, state: &mut FileChooserState, new_path: &mut Option<PathBuf>)
{
    let red = (1.0, 0.0, 0.0, 1.0);
    let default = (255.0, 255.0, 255.0, 1.0);
    let colvars = if state.input_path_valid {
        [(ImGuiCol::Text, default)]
    } else {
        [(ImGuiCol::Text, red)]
    };

    ui.text(im_str!("current path:"));
    ui.same_line(0.0);

    ui.with_color_vars(&colvars, || {
        ui.with_item_width(-1.0, || {
            if ui.input_text(im_str!("##fileio_cd"), &mut state.input_path)
                .build()
            {
                let p = PathBuf::from(&state.input_path.to_str());
                if p.is_dir() && state.path != state.input_path {
                    *new_path = Some(p);
                } else {
                    state.input_path_valid = false;
                }
            }
        });
    });
}
// ----------------------------------------------------------------------------
#[inline]
fn render_quickdir(
    ui: &Ui<'_>, state: &mut FileChooserState, new_path: &mut Option<PathBuf>)
{
    ui.new_line();
    for (i, dir) in state.quick_dir.iter().enumerate() {
        ui.same_line(0.0);
        if ui.small_button(dir) {
            *new_path = Some(
                state.quick_dir
                    .iter()
                    .take(i + 1)
                    .map(|imstr| imstr.to_str())
                    .collect()
            )
        }
    }
}
// ----------------------------------------------------------------------------
#[inline]
fn render_drive_list(
    ui: &Ui<'_>, state: &mut FileChooserState, new_path: &mut Option<PathBuf>)
{
    let mut selected = -1;
    if ui.list_box2(
            state.drives.iter()
                .map(Borrow::borrow)
                .collect::<Vec<_>>()
                .as_slice(),
            &mut selected
        )
        .height_in_items(10)
        .label(im_str!("##fileio_drives"))
        .build()
    {
        if let Some(drive) = state.drives.get(selected as usize) {
            let p = PathBuf::from(drive.to_str());
            if p.exists() {
                *new_path = Some(p);
            }
        }
    }
}
// ----------------------------------------------------------------------------
#[inline]
fn render_dir_list(
    ui: &Ui<'_>, state: &mut FileChooserState, new_path: &mut Option<PathBuf>)
{
    let mut selected = -1;
    if ui.list_box2(
            state.dirs.iter()
                .map(Borrow::borrow)
                .collect::<Vec<_>>()
                .as_slice(),
            &mut selected
        )
        .height_in_items(10)
        .label(im_str!("##fileio_dirs"))
        .build()
    {
        if let Some(dir) = state.dirs.get(selected as usize) {
            let mut p = PathBuf::from(&state.path.to_str());
            p.push(dir.to_str());
            if p.is_dir() {
                *new_path = Some(p);
            }
        }
    }
}
// ----------------------------------------------------------------------------
#[inline]
fn render_file_list(
    ui: &Ui<'_>, state: &mut FileChooserState, _new_path: &mut Option<PathBuf>)
{
    let mut selected = -1;
    if ui.list_box2(
            state.files.iter()
                .map(Borrow::borrow)
                .collect::<Vec<_>>()
                .as_slice(),
            &mut selected
        )
        .height_in_items(10)
        .label(im_str!("##fileio_files"))
        .build()
    {
        trace!("files selected: {}", selected);
    }
}
// ----------------------------------------------------------------------------
#[inline]
fn render_buttons<OK, CANCEL>(ui: &Ui, ok: OK, cancel: CANCEL)
where
    OK: FnOnce(),
    CANCEL: FnOnce(),
{
    if ui.small_button(im_str!("   Ok   ")) {
        ok();
    }
    ui.same_line(0.0);
    if ui.small_button(im_str!(" Cancel ")) {
        cancel();
    }
}
// ----------------------------------------------------------------------------
#[inline]
fn update_path(new_path: &Path, state: &mut FileChooserState) {
    match new_path.canonicalize() {
        Ok(new_path) => {
            let new_path = strip_unc_prefix(new_path);

            state.input_path_valid = true;
            state.input_path = ImString::new(new_path.to_string_lossy());
            state.input_path.reserve_exact(FILEPATH_MAX_LENGTH);
            state.path = ImString::new(new_path.to_string_lossy());

            state.quick_dir = split_path(&new_path);

            state.dirs = Vec::new();
            state.files = Vec::new();

            match scan_dir(&new_path) {
                Ok((dirs, files)) => {
                    state.dirs = dirs;
                    state.files = files;
                },
                Err(why) => error!("{}", why),
            }
        },
        Err(why) => error!("{}", why),
    }
}
// ----------------------------------------------------------------------------
fn scan_dir(path: &Path) -> io::Result<(Vec<ImString>, Vec<ImString>)>
{
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in path.read_dir()? {
        let entry = entry?;
        let filetype = entry.file_type()?;

        let name = ImString::new(entry.file_name().to_string_lossy());

        if filetype.is_dir() {
            dirs.push(name);
        } else if filetype.is_file() {
            files.push(name)
        }
    }

    dirs.sort();
    files.sort();

    // prepend after sort to ensure it's always on top
    dirs.insert(0, ImString::new(".."));

    Ok((dirs, files))
}
// ----------------------------------------------------------------------------
// platform dependent functions
// ----------------------------------------------------------------------------
// linux
// ----------------------------------------------------------------------------
#[cfg(not(windows))]
#[inline]
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    path
}
// ----------------------------------------------------------------------------
#[cfg(not(windows))]
#[inline]
fn split_path(path: &Path) -> Vec<ImString> {
    path.components()
        .map(|c| ImString::new(c.as_os_str().to_string_lossy()))
        .collect()
}
// ----------------------------------------------------------------------------
#[cfg(not(windows))]
#[inline]
fn scan_for_disknames() -> Vec<ImString> {
    vec![]
}
// ----------------------------------------------------------------------------
// windows
// ----------------------------------------------------------------------------
#[cfg(windows)]
#[inline]
fn strip_unc_prefix(path: PathBuf) -> PathBuf {
    use std::iter::FromIterator;
    use std::path::{Component, Prefix};

    PathBuf::from_iter(path.components()
        .filter_map(|c| match c {
            Component::Prefix(p) => {
                match p.kind() {
                    Prefix::VerbatimDisk(disk) => Some(format!("{}:", disk as char)),

                    _ => None,
                }
            },
            _ => Some(c.as_os_str().to_string_lossy().into_owned()),
        }))
}
// ----------------------------------------------------------------------------
#[cfg(windows)]
#[inline]
fn split_path(path: &PathBuf) -> Vec<ImString> {
    use std::path::{Component, Prefix};

    path.components()
        .filter_map(|c| match c {
            Component::Prefix(c) => match c.kind() {
                Prefix::VerbatimDisk(disk) => Some(ImString::new(format!("{}:\\", disk as char))),

                Prefix::Disk(disk) => Some(ImString::new(format!("{}:\\", disk as char))),

                Prefix::VerbatimUNC(server, share) => Some(ImString::new(format!(
                    "\\\\{}\\{}",
                    server.to_string_lossy(),
                    share.to_string_lossy()
                ))),

                _ => None,
            },

            Component::RootDir => None,

            _ => Some(ImString::new(c.as_os_str().to_string_lossy())),
        })
        .collect()
}
// ----------------------------------------------------------------------------
#[cfg(windows)]
#[inline]
fn scan_for_disknames() -> Vec<ImString> {
    let mut drives = Vec::with_capacity(16);

    for disk in b'A' .. b'[' {
        let drive = format!("{}:", disk as char);
        if PathBuf::from(&drive).exists() {
            drives.push(ImString::new(drive));
        }
    }
    drives
}
// ----------------------------------------------------------------------------
