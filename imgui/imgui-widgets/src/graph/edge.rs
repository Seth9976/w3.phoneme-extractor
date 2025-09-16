use imgui::{ImStr, ImVec2};
use sys::ImU32;

#[must_use]
pub struct Edge<'p> {
    label: Option<&'p ImStr>,

    start: ImVec2,
    end: ImVec2,
    scale: f32,

    color: ImU32,
    width: f32,
}

impl<'p> Edge<'p> {
    pub(super) fn new(start: ImVec2, end: ImVec2) -> Self {
        Edge {
            label: None,
            start,
            end,
            scale: 1.0,
            color: 0xFFFF_FFFF,
            width: 1.0,
        }
    }

    #[inline]
    pub fn set_label(mut self, label: &'p ImStr) -> Self {
        self.label = Some(label);
        self
    }

    #[inline]
    pub fn set_color(mut self, color: ImU32) -> Self {
        self.color = color;
        self
    }

    #[inline]
    pub fn set_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    #[inline]
    pub fn set_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn build(self, offset: ImVec2) {
        use sys::*;

        let p1 = self.start - offset;
        let p2 = self.end - offset;

        let mut screen_pos = ImVec2::zero();
        unsafe {
            igGetCursorScreenPos(&mut screen_pos);
        }

        // calculate two support points (out is always on the right, in always on the left)
        let x = if p1.x < p2.x {
            f32::min(p2.x - p1.x, 100.0)
        } else {
            f32::min(p1.x - p2.x, 100.0)
        };
        let distance = match p1.y {
            y1 if y1 < p2.y => ImVec2::new(x, f32::min(p2.y - p1.y, 25.0)),
            y1 if y1 > p2.y => ImVec2::new(x, f32::max(p2.y - p1.y, -25.0)),
            _ => ImVec2::new(x, 0.0),
        };

        let s1 = screen_pos + (p1 + distance) * self.scale;
        let s2 = screen_pos + (p2 - distance) * self.scale;
        let p1 = screen_pos + p1 * self.scale;
        let p2 = screen_pos + p2 * self.scale;

        unsafe {
            let drawlist = igGetWindowDrawList();

            ImDrawList_AddBezierCurve(drawlist, p1, s1, s2, p2, self.color, self.width, 0);
        }
    }
}
