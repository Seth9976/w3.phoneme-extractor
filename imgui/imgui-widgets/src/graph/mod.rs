use imgui::{ImStr, ImVec2, Ui};

use super::{GraphWidgets, Rectangle};

pub use self::block::Socket as BlockSocket;

use self::plane::Plane;
use self::block::Block;
use self::edge::Edge;

mod plane;
mod block;
mod edge;

impl GraphWidgets for Ui<'_> {
    fn graph(&self) -> Widgets {
        Widgets::new(self)
    }
}

pub struct Widgets<'ui> {
    ui: &'ui Ui<'ui>,
}

impl<'ui> Widgets<'ui> {
    fn new(ui: &'ui Ui) -> Widgets<'ui> {
        Widgets { ui }
    }

    pub fn plane<'p, T: Into<Rectangle>>(
        &self,
        id: &'p ImStr,
        dimension: T,
        canvas_size: (f32, f32),
    ) -> Plane<'ui, 'p> {
        Plane::new(self.ui, id, dimension.into(), canvas_size)
    }

    pub fn block<'p, T: Into<ImVec2>, InSocket, OutSocket>(
        &self,
        id: &'p ImStr,
        label: &'p ImStr,
        pos: T,
        size: T,
    ) -> Block<'ui, 'p, InSocket, OutSocket>
    where
        InSocket: AsRef<ImStr>,
        OutSocket: AsRef<ImStr>,
    {
        Block::new(self.ui, id, label, pos.into(), size.into())
    }

    pub fn edge<'p, T: Into<ImVec2>>(&self, out_point: T, in_point: T) -> Edge<'p> {
        Edge::new(out_point.into(), in_point.into())
    }
}

pub fn calculate_in_socket_position(
    socket_slot: usize,
    sockets: usize,
    rounding: f32,
    block_pos: ImVec2,
    block_size: ImVec2
) -> ImVec2
{
    block::calculate_in_socket_pos(socket_slot, sockets, rounding, block_pos, block_size)
}

pub fn calculate_out_socket_position(
    socket_slot: usize,
    sockets: usize,
    rounding: f32,
    block_pos: ImVec2,
    block_size: ImVec2
) -> ImVec2
{
    block::calculate_out_socket_pos(socket_slot, sockets, rounding, block_pos, block_size)
}
