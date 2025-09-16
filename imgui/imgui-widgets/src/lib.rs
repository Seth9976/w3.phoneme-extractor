extern crate imgui;
extern crate imgui_sys as sys;

use imgui::ImVec2;

pub mod audio;
pub mod graph;
// ----------------------------------------------------------------------------
pub trait AudioWidgets {
    fn audio(&self) -> audio::Widgets;
}

pub trait GraphWidgets {
    fn graph(&self) -> graph::Widgets;
}
// ----------------------------------------------------------------------------
#[derive(Clone, Copy, Debug)]
pub struct Rectangle {
    pub min: ImVec2,
    pub max: ImVec2,
}
// ----------------------------------------------------------------------------
pub struct UiDragging<T> {
    active: bool,
    start_offset: T,
    button: u8,
    prev_delta: ImVec2,
}
// ----------------------------------------------------------------------------
pub struct Interaction<T: Copy> {
    /// abs mouse position within visibile part
    mpos: ImVec2,
    /// absolute position within the virtual dimension of the element (in case
    /// only part is visible due to panning and zoom)
    vpos: T,
    /// kind of mouse interaction that occured
    mouse: MouseInteraction,
}
// ----------------------------------------------------------------------------
pub enum MouseInteraction {
    Clicked(u8),
    Released(u8),
    Wheel(f32),
}
// ----------------------------------------------------------------------------
impl<T: Copy> Interaction<T> {
    fn new(mpos: ImVec2, vpos: T, interation: MouseInteraction) -> Interaction<T> {
        Interaction {
            mpos,
            vpos,
            mouse: interation,
        }
    }

    #[inline]
    pub fn position(&self) -> ImVec2 {
        self.mpos
    }

    #[inline]
    pub fn virtual_position(&self) -> T {
        self.vpos
    }

    #[inline]
    pub fn mouse(&self) -> &MouseInteraction {
        &self.mouse
    }
}
// ----------------------------------------------------------------------------
impl Rectangle {
    pub fn extend(mut self, value: f32) -> Self {
        self.min = (self.min.x - value, self.min.y - value).into();
        self.max = (self.max.x + value, self.max.y + value).into();
        self
    }
}
// ----------------------------------------------------------------------------
impl<T> UiDragging<T> {
    // ------------------------------------------------------------------------
    pub fn start(start_offset: T) -> UiDragging<T> {
        // figure out which button is pressed
        let button = unsafe {
            let button = if sys::igIsMouseDown(0) {
                0
            } else if sys::igIsMouseDown(1) {
                1
            } else {
                2
            };
            sys::igResetMouseDragDelta(button);
            button
        };

        UiDragging {
            active: true,
            start_offset,
            button: button as u8,
            prev_delta: ImVec2::zero(),
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn active(&self) -> bool {
        self.active
    }
    // ------------------------------------------------------------------------
    pub fn update<F>(&mut self, f: F) -> bool
    where
        F: FnOnce(&T, ImVec2),
    {
        if self.active {
            let is_dragging = unsafe { sys::igIsMouseDown(i32::from(self.button)) };
            if is_dragging {
                let mut drag_delta = ImVec2::zero();
                unsafe {
                    sys::igGetMouseDragDelta(&mut drag_delta, i32::from(self.button), -1.0);
                }

                if self.prev_delta != drag_delta {
                    self.prev_delta = drag_delta;
                    f(&self.start_offset, drag_delta);
                }
            } else {
                unsafe {
                    sys::igResetMouseDragDelta(i32::from(self.button));
                }
                self.active = false;
            }
        }
        self.active
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
