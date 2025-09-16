use imgui::{ImStr, ImVec2, Ui};

use {Interaction, MouseInteraction, Rectangle};

#[must_use]
pub struct Plane<'ui, 'p> {
    ui: &'ui Ui<'ui>,
    id: &'p ImStr,
    canvas_size: ImVec2,

    /// virtual dimension of plane
    dimension: Rectangle,
    /// top left corner
    offset: ImVec2,
    zoom: f32,
}

impl<'ui, 'p> Plane<'ui, 'p> {
    pub(super) fn new(
        ui: &'ui Ui<'ui>,
        id: &'p ImStr,
        dimension: Rectangle,
        canvas_size: (f32, f32),
    ) -> Self {
        let width = if canvas_size.0 <= 1.0 {
            1.0
        } else {
            canvas_size.0
        };
        let height = if canvas_size.1 <= 1.0 {
            1.0
        } else {
            canvas_size.1
        };

        Plane {
            ui,
            id,
            canvas_size: ImVec2::new(width, height),
            dimension,
            offset: ImVec2::zero(),
            zoom: 1.0,
        }
    }

    #[inline]
    pub fn set_zoom(mut self, factor: f32) -> Self {
        self.zoom = if factor > 0.01 { factor } else { 0.01 };
        self
    }

    #[inline]
    pub fn set_offset(mut self, offset: ImVec2) -> Self {
        self.offset = offset;
        self
    }

    pub fn build<F>(self, f: F) -> Option<Interaction<ImVec2>>
    where
        F: FnOnce((ImVec2, ImVec2)),
    {
        use sys::*;

        let (visible_area, interaction) = unsafe {
            let col_frame = igGetColorU32(ImGuiCol::FrameBg, 1.0);

            let mut screen_pos = ImVec2::zero();
            igGetCursorScreenPos(&mut screen_pos);

            let bb = ImRect {
                min: screen_pos,
                max: screen_pos + self.canvas_size,
            };

            igItemSize(bb, 0.0);
            igSameLine(screen_pos.x, -1.0);
            if !igItemAdd(bb, igGetIDStr(self.id.as_ptr())) {
                return None;
            }

            igPushClipRect(bb.min, bb.max, true);

            let drawlist = igGetWindowDrawList();

            // background rectangle
            ImDrawList_AddRectFilled(
                drawlist,
                screen_pos,
                screen_pos + self.canvas_size,
                col_frame,
                0.0,
                0,
            );

            // calculate visible part of virtual graph (not canvas!) area based on size, zoom,
            // offset and canvas size
            let top_left = ImVec2::new(
                self.offset
                    .x
                    .clamp(self.dimension.min.x, self.dimension.max.x),
                self.offset
                    .y
                    .clamp(self.dimension.min.y, self.dimension.max.y),
            );
            let bottom_right = ImVec2::new(
                (top_left.x + self.canvas_size.x / self.zoom)
                    .clamp(self.dimension.min.x, self.dimension.max.x),
                (top_left.y + self.canvas_size.y / self.zoom)
                    .clamp(self.dimension.min.y, self.dimension.max.y),
            );
            let visible_area = (top_left, bottom_right);

            let interaction = if igIsItemHovered(ImGuiHoveredFlags::empty()) {
                let mouse_pos: ImVec2 = self.ui.imgui().mouse_pos().into();
                let mouse_wheel = self.ui.imgui().mouse_wheel();

                let canvas_pos = mouse_pos - screen_pos;
                let plane_pos = ImVec2::new(
                    f32::min(top_left.x + canvas_pos.x / self.zoom, bottom_right.x),
                    f32::min(top_left.y + canvas_pos.y / self.zoom, bottom_right.y),
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
                }
                .map(|mouse_interaction| Interaction::new(canvas_pos, plane_pos, mouse_interaction))
            } else {
                None
            };
            (visible_area, interaction)
        };

        f(visible_area);

        unsafe {
            igPopClipRect();
            igNewLine();
        }
        interaction
    }
}
// ----------------------------------------------------------------------------
