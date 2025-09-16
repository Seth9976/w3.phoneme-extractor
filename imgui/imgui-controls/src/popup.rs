//
// popup
//

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub trait PopupView<Out>: fmt::Debug {
    fn title(&self) -> &str;
    fn size(&self) -> (f32, f32) {
        (250.0, 150.0)
    }
    fn draw(&mut self, ui: &Ui<'_>) -> Option<Out>;
    fn valid(&self) -> bool {
        true
    }
}
// ----------------------------------------------------------------------------
pub trait PopupControl<In, Out>: fmt::Debug {
    fn validate(&mut self) {}
    fn process_action(&mut self, action: In) -> Option<Out>;
    fn on_ok(&self) -> (bool, Option<Out>);
    fn on_cancel(&self) -> (bool, Option<Out>) {
        (true, None)
    }
}
// ----------------------------------------------------------------------------
pub trait PopupContent<In, Out>: PopupView<Out> + PopupControl<In, Out> {}
// ----------------------------------------------------------------------------
pub struct Popup<In, Out> {
    title: ImString,
    content: Box<dyn PopupContent<In, Out>>,
    opened: bool,
    size: (f32, f32),
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::fmt;

use imgui::{ImString, Ui};
// ----------------------------------------------------------------------------
use imgui;

impl<In, Out> Popup<In, Out> {
    // ------------------------------------------------------------------------
    pub fn new(mut content: Box<dyn PopupContent<In, Out>>) -> Popup<In, Out> {
        let size = content.size();
        // make sure the ok button state is synced to validity of content
        content.validate();
        Popup {
            title: ImString::new(content.title()),
            content,
            opened: true,
            size,
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn opened(&self) -> bool {
        self.opened
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_size(mut self, size: (f32, f32)) -> Self {
        self.size = size;
        self
    }
    // ------------------------------------------------------------------------
    pub fn draw(&mut self, ui: &Ui<'_>) -> Option<Out> {
        let mut result = None;
        let mut opened = true;

        ui.window(im_str!("{}##popup", self.title.to_str()))
            .size(self.size, imgui::ImGuiCond::Always)
            .resizable(false)
            .movable(false)
            .opened(&mut opened)
            .build_modal(|| {
                result = self.content.draw(ui);

                ui.separator();
                ui.spacing();
                if ui.small_button(im_str!(" Cancel ")) {
                    result = self.on_cancel();
                }
                ui.same_line(self.size.0 - 70.0);
                if ui.small_enabled_button(im_str!("   Ok   "), self.content.valid()) {
                    result = self.on_ok();
                }
            });
        if opened {
            result
        } else {
            self.on_cancel()
        }
    }
    // ------------------------------------------------------------------------
    pub fn handle_action(&mut self, action: In) -> Option<Out> {
        self.content.process_action(action)
    }
    // ------------------------------------------------------------------------
    fn on_ok(&mut self) -> Option<Out> {
        let (close, action) = self.content.on_ok();
        self.opened = !close;
        action
    }
    // ------------------------------------------------------------------------
    fn on_cancel(&mut self) -> Option<Out> {
        let (close, action) = self.content.on_cancel();
        self.opened = !close;
        action
    }
    // ------------------------------------------------------------------------
}
