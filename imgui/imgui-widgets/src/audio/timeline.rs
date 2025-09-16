use std::ptr;

use imgui::{ImStr, ImVec2, ImVec4, Ui};

use {Interaction, MouseInteraction};

#[must_use]
pub struct Timeline<'ui, 'p> {
    ui: &'ui Ui<'ui>,
    label: &'p ImStr,
    duration: f32,
    size: ImVec2,

    view_start: f32,
    zoom: f32,
    hover_pos: &'p mut f32,
}

#[must_use]
pub struct Block<'ui, 'p> {
    ui: &'ui Ui<'ui>,
    id: &'p ImStr,
    label: &'p ImStr,

    timeframe: (f32, f32),
    timeframe_clipping: (f32, f32),

    draw_framesize: ImVec2,

    draw_border: (bool, bool),
    draw_label: bool,
    draw_borders: bool,
    draw_color: ImVec4,
    draw_label_color: ImVec4,
}

impl<'ui, 'p> Timeline<'ui, 'p> {
    pub fn new(
        ui: &'ui Ui<'ui>,
        label: &'p ImStr,
        mut duration: f32,
        hover_pos: &'p mut f32,
    ) -> Self {
        if duration <= 0.0 {
            duration = 1.0
        }
        Timeline {
            ui,
            label,
            duration,
            size: ImVec2::new(0.0, 0.0),
            view_start: 0.0,
            zoom: 1.0,
            hover_pos,
        }
    }

    #[inline]
    pub fn size<S: Into<ImVec2>>(mut self, size: S) -> Self {
        let mut size = size.into();
        // prevent division by zero
        if size.x < 1.0 {
            size.x = 1.0;
        }
        if size.y < 1.0 {
            size.y = 1.0
        }
        self.size = size;
        self
    }

    #[inline]
    pub fn view_start(mut self, mut start: f32) -> Self {
        if start < 0.0 {
            start = 0.0;
        }
        self.view_start = start;
        self
    }

    #[inline]
    pub fn zoom(mut self, factor: f32) -> Self {
        self.zoom = if factor > 0.01 { factor } else { 0.01 };
        self
    }

    pub fn build<F>(self, f: F) -> Option<Interaction<f32>>
    where
        F: FnOnce(ImVec2, ImVec2, (f32, f32)),
    {
        use sys::*;

        let (pos, view_start, view_end, interaction) = unsafe {
            let col_frame = igGetColorU32(ImGuiCol::FrameBg, 1.0);

            let mut screen_pos = ImVec2::zero();
            igGetCursorScreenPos(&mut screen_pos);

            let bb = ImRect {
                min: screen_pos,
                max: screen_pos + self.size,
            };

            igItemSize(bb, 0.0);
            if !igItemAdd(bb, igGetIDStr(self.label.as_ptr())) {
                return None;
            }

            let drawlist = igGetWindowDrawList();

            // background rectangle
            ImDrawList_AddRectFilled(
                drawlist,
                screen_pos,
                screen_pos + self.size,
                col_frame,
                0.0,
                0,
            );

            let (view_start_max, view_duration) = if self.zoom > 1.0 {
                let view_duration = self.duration / self.zoom;
                (self.duration - view_duration, view_duration)
            } else {
                (0.0, self.duration)
            };

            let view_start = if self.view_start > view_start_max {
                view_start_max
            } else {
                self.view_start
            };

            let view_end = view_start + view_duration;

            let interaction = if igIsItemHovered(ImGuiHoveredFlags::empty()) {
                let mouse_pos: ImVec2 = self.ui.imgui().mouse_pos().into();
                let mouse_wheel = self.ui.imgui().mouse_wheel();
                let canvas_pos = mouse_pos - screen_pos;
                let time_pos = view_start
                    + (view_duration * (mouse_pos.x - screen_pos.x) / self.size.x);
                *self.hover_pos = time_pos;

                if mouse_wheel != 0.0 {
                    Some(MouseInteraction::Wheel(mouse_wheel))
                } else if igIsItemClicked(0) {
                    Some(MouseInteraction::Clicked(0))
                } else if igIsItemClicked(1) {
                    Some(MouseInteraction::Clicked(1))
                } else if igIsItemClicked(2) {
                    Some(MouseInteraction::Clicked(2))
                } else if igIsMouseReleased(0) {
                    Some(MouseInteraction::Released(0))
                } else if igIsMouseReleased(1) {
                    Some(MouseInteraction::Released(1))
                } else if igIsMouseReleased(2) {
                    Some(MouseInteraction::Released(2))
                } else {
                    None
                }.map(|mouse_interaction| {
                    Interaction::new(canvas_pos, time_pos, mouse_interaction)
                })
            } else {
                None
            };

            igSameLine(screen_pos.x, -1.0);

            (screen_pos, view_start, view_end, interaction)
        };

        f(pos, self.size, (view_start, view_end));

        unsafe {
            igNewLine();
        }
        interaction
    }
}

impl<'ui, 'p> Block<'ui, 'p> {
    pub fn new(
        ui: &'ui Ui<'ui>,
        id: &'p ImStr,
        label: &'p ImStr,
        timeframe: (f32, f32),
        timeframe_clipping: (f32, f32),
    ) -> Self {
        // clamp block
        let start = timeframe.0.clamp(timeframe_clipping.0, timeframe_clipping.1);
        let end = timeframe.1.clamp(start, timeframe_clipping.1);
        Block {
            ui,
            id,
            label,
            timeframe: (start, end),
            timeframe_clipping,
            draw_framesize: (1.0, 1.0).into(),
            draw_border: (false, false),
            draw_label: true,
            draw_borders: true,
            draw_color: (128.0, 128.0, 128.0, 1.0).into(),
            draw_label_color: (128.0, 128.0, 128.0, 1.0).into(),
        }
    }

    #[inline]
    pub fn set_draw_framesize(mut self, frame: ImVec2) -> Self {
        self.draw_framesize = frame;
        self
    }

    #[inline]
    pub fn set_always_draw_borders(mut self, start_border: bool, end_border: bool) -> Self {
        self.draw_border = (start_border, end_border);
        self
    }

    #[inline]
    pub fn set_draw_label(mut self, draw: bool) -> Self {
        self.draw_label = draw;
        self
    }

    #[inline]
    pub fn set_draw_borders(mut self, draw: bool) -> Self {
        self.draw_borders = draw;
        self
    }

    #[inline]
    pub fn set_draw_color<C: Into<ImVec4>>(mut self, col: C) -> Self {
        self.draw_color = col.into();
        self
    }

    #[inline]
    pub fn set_draw_label_color<C: Into<ImVec4>>(mut self, col: C) -> Self {
        self.draw_label_color = col.into();
        self
    }

    #[inline]
    pub fn set_alpha(mut self, alpha: f32) -> Self {
        self.draw_color.w = alpha;
        self.draw_label_color.w = alpha;
        self
    }

    pub fn build(self) -> Option<Interaction<ImVec2>> {
        use sys::*;

        unsafe {
            let col_block = igGetColorU32Vec(&self.draw_color);
            let col_label = igGetColorU32Vec(&self.draw_label_color);

            let mut screen_pos = ImVec2::zero();
            igGetCursorScreenPos(&mut screen_pos);

            let duration = self.timeframe_clipping.1 - self.timeframe_clipping.0;
            let scale_x = self.draw_framesize.x / duration;

            let x1 = screen_pos.x + (self.timeframe.0 - self.timeframe_clipping.0) * scale_x;
            let x2 = screen_pos.x + (self.timeframe.1 - self.timeframe_clipping.0) * scale_x;
            let y1 = screen_pos.y;
            let y2 = screen_pos.y + self.draw_framesize.y;

            let bb = ImRect {
                min: ImVec2::new(x1, y1),
                max: ImVec2::new(x2, y2),
            };

            igItemSize(bb, 0.0);
            igSameLine(screen_pos.x, -1.0);
            if !igItemAdd(bb, igGetIDStr(self.id.as_ptr())) {
                return None;
            }

            let drawlist = igGetWindowDrawList();

            // background rectangle
            ImDrawList_AddRectFilled(
                drawlist,
                ImVec2::new(x1, y1),
                ImVec2::new(x2, y2),
                col_block,
                0.0,
                0,
            );

            if self.draw_borders {
                ImDrawList_AddRect(
                    drawlist,
                    ImVec2::new(x1, y1),
                    ImVec2::new(x2, y2),
                    col_block,
                    0.0,
                    0,
                    1.0,
                );
            }
            if self.draw_label {
                let mut textsize = ImVec2::zero();
                igCalcTextSize(&mut textsize, self.label.as_ptr(), ptr::null(), false, 0.0);

                ImDrawList_AddText(
                    drawlist,
                    ImVec2::new(
                        x1 + (x2 - x1 - textsize.x) * 0.5,
                        y1 + (y2 - y1 - textsize.y) * 0.5,
                    ),
                    col_label,
                    self.label.as_ptr(),
                    ptr::null(),
                );
            }

            if igIsItemHovered(ImGuiHoveredFlags::empty()) {
                let mouse_pos: ImVec2 = self.ui.imgui().mouse_pos().into();
                let mouse_wheel = self.ui.imgui().mouse_wheel();
                let canvas_pos = mouse_pos - screen_pos;
                let virtual_pos = ImVec2::new(
                    (mouse_pos.x - x1) / (x2 - x1),
                    (mouse_pos.y - y1) / (y2 - y1),
                );

                if mouse_wheel != 0.0 {
                    Some(MouseInteraction::Wheel(mouse_wheel))
                } else if igIsItemClicked(0) {
                    Some(MouseInteraction::Clicked(0))
                } else if igIsItemClicked(1) {
                    Some(MouseInteraction::Clicked(1))
                } else if igIsItemClicked(2) {
                    Some(MouseInteraction::Clicked(2))
                } else if igIsMouseReleased(0) {
                    Some(MouseInteraction::Released(0))
                } else if igIsMouseReleased(1) {
                    Some(MouseInteraction::Released(1))
                } else if igIsMouseReleased(2) {
                    Some(MouseInteraction::Released(2))
                } else {
                    None
                }.map(|mouse_interaction| {
                    Interaction::new(canvas_pos, virtual_pos, mouse_interaction)
                })
            } else {
                None
            }
        }
    }
}
