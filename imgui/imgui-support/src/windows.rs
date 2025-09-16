//
// gui/auxiliary windows/helpers
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
#[derive(Default)]
pub struct ErrorWindow {
    show: bool,
    msg: ImString,
}
#[derive(Default)]
pub struct InfoWindow {
    show: bool,
    msg: ImString,
}
// ----------------------------------------------------------------------------
pub enum Confirm {
    Yes,
    No,
    Cancel,
}
// ----------------------------------------------------------------------------
pub fn info(ui: &Ui<'_>, info_window: &mut InfoWindow) {
    show_info_window(ui, info_window);
}
// ----------------------------------------------------------------------------
pub fn error(ui: &Ui<'_>, error_window: &mut ErrorWindow) {
    show_error_window(ui, error_window);
}
// ----------------------------------------------------------------------------
pub fn request_yes_no(ui: &Ui<'_>, text: &str) -> Option<Confirm> {
    show_confirm_window(ui, text)
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui;
use imgui::{Ui, ImString};
// ----------------------------------------------------------------------------
impl InfoWindow {
    // ------------------------------------------------------------------------
    #[inline]
    pub fn show(&self) -> bool {
        self.show
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_msg(&mut self, msg: &str) {
        self.msg = ImString::new(msg);
        self.show = true;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl ErrorWindow {
    // ------------------------------------------------------------------------
    #[inline]
    pub fn show(&self) -> bool {
        self.show
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_msg(&mut self, msg: &str) {
        self.msg = ImString::new(msg.replace(". ", "\n"));
        self.show = true;
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
#[inline]
fn show_error_window(ui: &Ui<'_>, info: &mut ErrorWindow) {
    let mut opened_window = true;

    let (opened, msg) = (&mut info.show, &mut info.msg);
    ui.window(im_str!("Error"))
        .size((400.0, 150.0), imgui::ImGuiCond::Always)
        .resizable(false)
        .movable(false)
        .opened(opened)
        .build_modal(|| {
            ui.with_region_height(im_str!("msg"), -1.0, || {
                ui.text_wrapped(msg);
            });

            ui.separator();
            ui.spacing();
            ui.new_line();
            ui.same_line(175.0);
            opened_window = !ui.small_button(im_str!("  OK  "));
        });
    *opened = opened_window;
}
// ----------------------------------------------------------------------------
#[inline]
fn show_info_window(ui: &Ui<'_>, info: &mut InfoWindow) {
    let mut opened_window = true;

    let (opened, msg) = (&mut info.show, &mut info.msg);
    ui.window(im_str!("Info"))
        .size((400.0, 150.0), imgui::ImGuiCond::Always)
        .resizable(false)
        .movable(false)
        .opened(opened)
        .build_modal(|| {
            ui.with_region_height(im_str!("msg"), -1.0, || {
                ui.text_wrapped(msg);
            });

            ui.separator();
            ui.spacing();
            ui.new_line();
            ui.same_line(175.0);
            opened_window = !ui.small_button(im_str!("  OK  "));
        });
    *opened = opened_window;
}
// ----------------------------------------------------------------------------
#[inline]
fn show_confirm_window(ui: &Ui<'_>, text: &str) -> Option<Confirm> {
    let mut result = None;
    let mut opened = true;
    ui.window(im_str!("Confirm"))
        .size((290.0, 170.0), imgui::ImGuiCond::Always)
        .resizable(false)
        .movable(false)
        .opened(&mut opened)
        .build_modal(|| {
            ui.with_region_height(im_str!("msg"), -1.0, || {
                ui.text_wrapped(&ImString::new(text));
            });

            ui.separator();
            ui.spacing();
            if ui.small_button(im_str!("  Yes   ")) {
                result = Some(Confirm::Yes);
            }
            ui.same_line(115.0);
            if ui.small_button(im_str!("   No   ")) {
                result = Some(Confirm::No);
            }
            ui.same_line(220.0);
            if ui.small_button(im_str!(" Cancel ")) {
                result = Some(Confirm::Cancel);
            }
        });
    result
}
// ----------------------------------------------------------------------------
