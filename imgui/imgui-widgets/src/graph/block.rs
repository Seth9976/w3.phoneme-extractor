use std::ptr;

use imgui::{ImGuiRoundingFlags, ImStr, ImVec2, Ui};

use {Interaction, MouseInteraction};

pub enum Socket<'a, InSocket: 'a, OutSocket: 'a> {
    In(&'a InSocket),
    Out(&'a OutSocket),
}

#[must_use]
pub struct Block<'ui, 'p, InSocket: 'p, OutSocket: 'p> {
    ui: &'ui Ui<'ui>,
    id: &'p ImStr,
    label: &'p ImStr,
    in_sockets: Option<&'p [InSocket]>,
    out_sockets: Option<&'p [OutSocket]>,

    pos: ImVec2,
    size: ImVec2,
    scale: f32,
    highlight: bool,

    rounding: f32,
    rounding_flags: ImGuiRoundingFlags,

    draw_label: bool,
    draw_borders: bool,
    draw_sockets: bool,
    draw_socket_labels: bool,

    hover_highlight_in_socket: bool,
    hover_highlight_out_socket: bool,
    hover_draw_in_socket_labels: bool,
    hover_draw_out_socket_labels: bool,

    color: u32,
    border_color: u32,
}

impl<'ui, 'p, InSocket, OutSocket> Block<'ui, 'p, InSocket, OutSocket>
where
    InSocket: AsRef<ImStr>,
    OutSocket: AsRef<ImStr>,
{
    pub(super) fn new(
        ui: &'ui Ui<'ui>,
        id: &'p ImStr,
        label: &'p ImStr,
        pos: ImVec2,
        size: ImVec2,
    ) -> Self {
        use sys::*;

        Block {
            ui,
            id,
            label,
            in_sockets: None,
            out_sockets: None,
            pos,
            size,
            scale: 1.0,
            highlight: false,

            rounding: 0.0,
            rounding_flags: ImGuiRoundingFlags::empty(),

            draw_label: true,
            draw_borders: true,
            draw_sockets: false,
            draw_socket_labels: false,

            hover_highlight_out_socket: false,
            hover_highlight_in_socket: false,

            hover_draw_out_socket_labels: false,
            hover_draw_in_socket_labels: false,

            color: unsafe { igGetColorU32(ImGuiCol::FrameBg, 1.0) },

            border_color: unsafe { igGetColorU32(ImGuiCol::Border, 1.0) },
        }
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
    pub fn set_draw_sockets(mut self, sockets: bool, labels: bool) -> Self {
        self.draw_sockets = sockets;
        self.draw_socket_labels = labels;
        self
    }

    #[inline]
    pub fn set_rounding(mut self, strength: f32) -> Self {
        self.rounding = strength;
        self.rounding_flags.set(ImGuiRoundingFlags::All, true);
        self
    }

    #[inline]
    pub fn set_color(mut self, color: u32) -> Self {
        self.color = color;
        self
    }

    #[inline]
    pub fn set_border_color(mut self, color: u32) -> Self {
        self.border_color = color;
        self
    }

    #[inline]
    pub fn set_block_highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    #[inline]
    pub fn set_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    #[inline]
    pub fn set_in_sockets(mut self, sockets: &'p[InSocket]) -> Self {
        self.in_sockets = Some(sockets);
        self
    }

    #[inline]
    pub fn set_out_sockets(mut self, sockets: &'p[OutSocket]) -> Self {
        self.out_sockets = Some(sockets);
        self
    }

    #[inline]
    pub fn set_hover_hightlight_socket(mut self, out_sockets: bool, in_sockets: bool) -> Self {
        self.hover_highlight_out_socket = out_sockets;
        self.hover_highlight_in_socket = in_sockets;
        self
    }

    #[inline]
    pub fn set_hover_socket_labels(mut self, out_sockets: bool, in_sockets: bool) -> Self {
        self.hover_draw_out_socket_labels = out_sockets;
        self.hover_draw_in_socket_labels = in_sockets;
        self
    }

    fn draw_sockets<'socket, T>(
        &self,
        screen_pos: ImVec2,
        offset: ImVec2,
        sockets: &'socket [T],
        in_socket: bool,
        mouse_pos: ImVec2,
    ) -> Option<&'socket T>
    where
        T: AsRef<ImStr>,
    {
        use sys::*;

        let mut result = None;

        // socket size: 3.0, 6.0
        let socket_size = 3.0;
        let hover_width = 5.0;

        // setup values depending on socket type
        let (socket_rect, base_x, hover_highlight, hover_labels) = if in_socket {
            (
                (
                    ImVec2::new(-socket_size, -socket_size),
                    ImVec2::new(0.0, socket_size),
                ),
                self.pos.x - self.size.x * 0.5,
                self.hover_highlight_in_socket,
                self.hover_draw_in_socket_labels,
            )
        } else {
            (
                (
                    ImVec2::new(0.0, -socket_size),
                    ImVec2::new(socket_size, socket_size),
                ),
                self.pos.x + self.size.x * 0.5,
                self.hover_highlight_out_socket,
                self.hover_draw_out_socket_labels,
            )
        };

        let mut textsize = ImVec2::zero();
        let usable_height = self.size.y - 2.0 * self.rounding;

        let len = (sockets.len() + 1) as f32;
        let x = base_x - offset.x;

        // these are constant for every in socket
        let hover_x1 = screen_pos.x + (x - hover_width) * self.scale;
        let hover_x2 = screen_pos.x + (x + hover_width) * self.scale;
        let is_socket_x_hover = hover_x1 < mouse_pos.x && mouse_pos.x < hover_x2;

        let hover_height = f32::max(3.0, f32::min(hover_width, 0.5 * usable_height / len));

        unsafe {
            let col_text = igGetColorU32(ImGuiCol::Text, 1.0);
            let hover_col = igGetColorU32(ImGuiCol::PlotLinesHovered, 1.0);

            let drawlist = igGetWindowDrawList();

            for (i, socket) in sockets.iter().enumerate() {
                let center_y = self.pos.y - offset.y + ((i + 1) as f32 / len - 0.5) * usable_height;

                let center = ImVec2::new(x, center_y);

                let hover_y1 = screen_pos.y + (center_y - hover_height) * self.scale;
                let hover_y2 = screen_pos.y + (center_y + hover_height) * self.scale;
                let is_socket_y_hover = hover_y1 < mouse_pos.y && mouse_pos.y < hover_y2;
                let is_hovered = is_socket_x_hover && is_socket_y_hover && result.is_none();

                if is_hovered {
                    result = Some(socket);
                }

                let (socket_color, text_color) = if is_hovered && hover_highlight {
                    (hover_col, hover_col)
                } else {
                    (self.border_color, col_text)
                };

                let p1 = screen_pos + center * self.scale + socket_rect.0;
                let p2 = screen_pos + center * self.scale + socket_rect.1;

                ImDrawList_AddRectFilled(
                    drawlist,
                    p1,
                    p2,
                    socket_color,
                    0.0,
                    ImGuiRoundingFlags::empty().bits(),
                );

                if self.draw_socket_labels || (is_hovered && hover_labels) {
                    let p1 = if in_socket {
                        igCalcTextSize(
                            &mut textsize,
                            socket.as_ref().as_ptr(),
                            ptr::null(),
                            false,
                            0.0,
                        );
                        screen_pos + center * self.scale + (-textsize.x - 8.0, -8.0).into()
                    } else {
                        screen_pos + center * self.scale + (8.0, -8.0).into()
                    };

                    ImDrawList_AddText(
                        drawlist,
                        p1,
                        text_color,
                        socket.as_ref().as_ptr(),
                        ptr::null(),
                    );
                }

                // // -- TMP test for hover rectangle
                // ImDrawList_AddRect(
                //     drawlist,
                //     (hover_x1, hover_y1).into(),
                //     (hover_x2, hover_y2).into(),
                //     hover_col,
                //     0.0,
                //     ImGuiRoundingFlags::empty().bits(),
                //     1.0,
                // );
            }
        }
        result
    }

    #[allow(clippy::type_complexity)]
    pub fn build(
        self,
        offset: ImVec2,
    ) -> Option<(Interaction<ImVec2>, Option<Socket<'p, InSocket, OutSocket>>)> {
        use sys::*;

        let b1 = self.pos - self.size * 0.5;
        let b2 = self.pos + self.size * 0.5;

        // early bailout calculations with *imgui* first so even if widget is not
        // drawn the id is known and can be correctly used with ui.is_item_hovered
        let (screen_pos, p1, p2) = unsafe {
            let mut screen_pos = ImVec2::zero();
            igGetCursorScreenPos(&mut screen_pos);

            let p1 = screen_pos + (b1 - offset) * self.scale;
            let p2 = screen_pos + (b2 - offset) * self.scale;

            let bb = ImRect {
                min: p1 - ImVec2::new(5.0, 0.0) * self.scale,
                max: p2 + ImVec2::new(5.0, 0.0) * self.scale,
            };

            igItemSize(bb, 0.0);
            igSameLine(screen_pos.x, -1.0);

            if !igItemAdd(bb, igGetIDStr(self.id.as_ptr())) {
                return None;
            }
            (screen_pos, p1, p2)
        };

        unsafe {
            let col_text = igGetColorU32(ImGuiCol::Text, 1.0);

            let drawlist = igGetWindowDrawList();

            // background rectangle
            ImDrawList_AddRectFilled(
                drawlist,
                p1,
                p2,
                self.color,
                self.rounding,
                self.rounding_flags.bits(),
            );

            if self.draw_borders {
                ImDrawList_AddRect(
                    drawlist,
                    p1,
                    p2,
                    self.border_color,
                    self.rounding,
                    self.rounding_flags.bits(),
                    1.0,
                );
            }

            if self.draw_label {
                igPushClipRect(p1, p2, true);

                let mut textsize = ImVec2::zero();
                igCalcTextSize(&mut textsize, self.label.as_ptr(), ptr::null(), false, 0.0);

                ImDrawList_AddText(
                    drawlist,
                    p1 + (p2 - p1 - textsize) * 0.5,
                    col_text,
                    self.label.as_ptr(),
                    ptr::null(),
                );
                igPopClipRect();
            }

            let hovered_socket = if self.draw_sockets {
                let mouse_pos: ImVec2 = self.ui.imgui().mouse_pos().into();

                let mut hovered_in_socket = None;
                let mut hovered_out_socket = None;

                if let Some(sockets) = self.in_sockets {
                    hovered_in_socket =
                        self.draw_sockets(screen_pos, offset, sockets, true, mouse_pos)
                            .map(|slot| Socket::In(slot));
                }
                if let Some(sockets) = self.out_sockets {
                    hovered_out_socket =
                        self.draw_sockets(screen_pos, offset, sockets, false, mouse_pos)
                            .map(|slot| Socket::Out(slot))
                }

                hovered_in_socket.or(hovered_out_socket)
            } else {
                None
            };

            if self.highlight {
                // simple overlay with default highlight color and some alpha
                ImDrawList_AddRectFilled(
                    drawlist,
                    p1,
                    p2,
                    igGetColorU32(ImGuiCol::PlotLinesHovered, 0.5),
                    self.rounding,
                    self.rounding_flags.bits(),
                );
                ImDrawList_AddRect(
                    drawlist,
                    p1,
                    p2,
                    igGetColorU32(ImGuiCol::PlotLinesHovered, 0.9),
                    self.rounding,
                    self.rounding_flags.bits(),
                    2.0,
                );
            }

            if igIsItemHovered(ImGuiHoveredFlags::empty()) {
                let mouse_pos: ImVec2 = self.ui.imgui().mouse_pos().into();
                let canvas_pos = mouse_pos - screen_pos;

                if igIsItemClicked(0) {
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
                    (
                        Interaction::new(canvas_pos, mouse_pos, mouse_interaction),
                        hovered_socket,
                    )
                })
            } else {
                None
            }
        }
    }
}

#[inline]
pub fn calculate_in_socket_pos(
    socket_slot: usize,
    sockets: usize,
    rounding: f32,
    block_pos: ImVec2,
    block_size: ImVec2
) -> ImVec2
{
    let sockets = sockets as f32 + 1.0;
    let usable_height = block_size.y - 2.0 * rounding;

    let center_y = block_pos.y + ((socket_slot + 1) as f32 / sockets - 0.5) * usable_height;

    ImVec2::new(block_pos.x - block_size.x * 0.5, center_y)
}

#[inline]
pub fn calculate_out_socket_pos(
    socket_slot: usize,
    sockets: usize,
    rounding: f32,
    block_pos: ImVec2,
    block_size: ImVec2
) -> ImVec2
{
    let sockets = sockets as f32 + 1.0;
    let usable_height = block_size.y - 2.0 * rounding;

    let center_y = block_pos.y + ((socket_slot + 1) as f32 / sockets - 0.5) * usable_height;

    ImVec2::new(block_pos.x + block_size.x * 0.5, center_y)
}