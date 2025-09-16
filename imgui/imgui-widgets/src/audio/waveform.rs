use std::cmp;

use imgui::{ImStr, ImVec2, Ui};

use {Interaction, MouseInteraction};

#[must_use]
pub struct WaveForm<'ui, 'p> {
    ui: &'ui Ui<'ui>,
    label: &'p ImStr,
    values: &'p [i16],
    values_offset: usize,
    visible_range: &'p mut (usize, usize),
    max_value: i16,
    zoom_x: f32,
    marker1_pos: usize,
    marker2_pos: usize,
    marker_area: (usize, usize),
    graph_size: ImVec2,
}

impl<'ui, 'p> WaveForm<'ui, 'p> {
    pub fn new(
        ui: &'ui Ui<'ui>,
        label: &'p ImStr,
        values: &'p[i16],
        visible_range: &'p mut (usize, usize),
    ) -> Self {
        WaveForm {
            ui,
            label,
            values,
            values_offset: 0,
            visible_range,
            max_value: i16::MAX,
            zoom_x: 1.0,
            marker1_pos: 0,
            marker2_pos: 0,
            marker_area: (0, 0),
            graph_size: ImVec2::new(0.0, 0.0),
        }
    }

    #[inline]
    pub fn max_value(mut self, value: i16) -> Self {
        self.max_value = i16::abs(value);
        self
    }

    #[inline]
    pub fn offset(mut self, sample: usize) -> Self {
        self.values_offset = sample;
        self
    }

    #[inline]
    pub fn zoom_x(mut self, factor: f32) -> Self {
        // prevent division by zero
        self.zoom_x = if factor > 0.01 { factor } else { 0.01 };
        self
    }

    #[inline]
    pub fn marker1_pos(mut self, pos: usize) -> Self {
        self.marker1_pos = cmp::min(pos, self.values.len());
        self
    }

    #[inline]
    pub fn marker2_pos(mut self, pos: usize) -> Self {
        self.marker2_pos = cmp::min(pos, self.values.len());
        self
    }

    #[inline]
    pub fn marker_area(mut self, start: usize, end: usize) -> Self {
        if start < end {
            self.marker_area = (start, cmp::min(end, self.values.len()));
        }
        self
    }

    #[inline]
    pub fn graph_size<S: Into<ImVec2>>(mut self, graph_size: S) -> Self {
        let mut size = graph_size.into();
        // prevent division by zero
        if size.x < 1.0 {
            size.x = 1.0;
        }
        if size.y < 1.0 {
            size.y = 1.0;
        }
        self.graph_size = size;
        self
    }

    pub fn build(self) -> Option<Interaction<usize>> {
        use sys::*;

        unsafe {
            let col_frame = igGetColorU32(ImGuiCol::FrameBg, 1.0);
            let col_line = igGetColorU32(ImGuiCol::PlotLines, 1.0);
            let col_marker = igGetColorU32(ImGuiCol::PlotLinesHovered, 1.0);

            let mut screen_pos = ImVec2::zero();
            igGetCursorScreenPos(&mut screen_pos);

            let bb = ImRect {
                min: screen_pos,
                max: screen_pos + self.graph_size,
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
                screen_pos + self.graph_size,
                col_frame,
                0.0,
                0,
            );

            let len = self.values.len();
            let win_size = f32::trunc(len as f32 / self.zoom_x);
            let scale_x = self.graph_size.x / win_size;
            // reduce number of plotting points as it doesn't add much quality anyway
            let sampling_points = self.graph_size.x * 0.5;

            // discrete sampling step so scrolling (changing of offset) does not alias plotted graph
            let sampling_step = f32::ceil(win_size / sampling_points) as usize;

            let win_start = cmp::max(
                0,
                cmp::min(
                    self.values_offset - self.values_offset % sampling_step,
                    len.saturating_sub(win_size as usize),
                ),
            );
            let win_end = cmp::min(len, win_start.saturating_add(win_size as usize));
            *self.visible_range = (win_start, win_end);

            let mut p0 = ImVec2::zero();
            let mut p1;
            let scale_y = self.graph_size.y * 0.5 / f32::from(self.max_value);
            let base_y = screen_pos.y + 0.5 * self.graph_size.y;

            for i in 0..sampling_points as usize {
                let index = win_start + i * sampling_step;

                if index < win_end {
                    let v = f32::from(self.values[index]) * scale_y;

                    p1 = ImVec2::new(
                        screen_pos.x + scale_x * (i * sampling_step) as f32,
                        f32::min(self.graph_size.y, v),
                    );

                    if i > 0 {
                        ImDrawList_AddLine(
                            drawlist,
                            ImVec2::new(p0.x, base_y - p0.y),
                            ImVec2::new(p1.x, base_y - p1.y),
                            col_line,
                            1.0,
                        );
                        // ImDrawList_AddLine(drawlist, ImVec2::new(p0.x, base_y + p0.y), ImVec2::new(p1.x, base_y + p1.y), col_line, 1.0);
                    }
                    p0 = p1;
                }
            }

            if self.marker_area.0 < self.marker_area.1 {
                let area_start = cmp::max(self.marker_area.0, win_start);
                let area_end = cmp::min(self.marker_area.1, win_end);

                let x1 = screen_pos.x
                    + scale_x
                        * (area_start - area_start % sampling_step).saturating_sub(win_start)
                            as f32;
                let x2 = screen_pos.x
                    + scale_x
                        * (area_end - area_end % sampling_step).saturating_sub(win_start) as f32;
                ImDrawList_AddRectFilled(
                    drawlist,
                    (x1, screen_pos.y).into(),
                    (x2, screen_pos.y + self.graph_size.y).into(),
                    igGetColorU32(ImGuiCol::FrameBg, 0.5),
                    1.0,
                    0,
                );
            }

            if self.marker1_pos > win_start && self.marker1_pos < win_end {
                let x = screen_pos.x
                    + scale_x
                        * (self.marker1_pos - self.marker1_pos % sampling_step - win_start) as f32;
                ImDrawList_AddLine(
                    drawlist,
                    ImVec2::new(x, screen_pos.y),
                    ImVec2::new(x, screen_pos.y + self.graph_size.y),
                    col_marker,
                    1.0,
                );
            }

            if self.marker2_pos > win_start && self.marker2_pos < win_end {
                let x = screen_pos.x
                    + scale_x
                        * (self.marker2_pos - self.marker2_pos % sampling_step - win_start) as f32;
                ImDrawList_AddLine(
                    drawlist,
                    ImVec2::new(x, screen_pos.y),
                    ImVec2::new(x, screen_pos.y + self.graph_size.y),
                    col_line,
                    1.0,
                );
            }

            if igIsItemHovered(ImGuiHoveredFlags::empty()) {
                let mouse_pos: ImVec2 = self.ui.imgui().mouse_pos().into();
                let mouse_wheel = self.ui.imgui().mouse_wheel();
                let canvas_pos = mouse_pos - screen_pos;
                let sample_pos = win_start
                    + (win_size * (mouse_pos.x - screen_pos.x) / self.graph_size.x) as usize;

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
                    Interaction::new(canvas_pos, sample_pos, mouse_interaction)
                })
            } else {
                None
            }
        }
    }
}
