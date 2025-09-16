//
// gui: graph with connectable blocks
//

// TODO: update all BlockId trait guards with trait alias as soon as its stable

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub struct PlaneState<BlockId> {
    offset: ImVec2,
    zoom: f32,
    dim: Rectangle,
    zoom_restriction: (f32, f32),
    grid: (f32, f32),
    canvas_size: (f32, f32),
    show_block_extra_infos: bool,

    dragging: Option<DragOperation>,
    selected: Option<BlockId>,
}
// ----------------------------------------------------------------------------
pub type Blocks<BlockId, OutSocket, InSocket> =
    BTreeMap<BlockId, Block<BlockId, OutSocket, InSocket>>;
pub type Edges<BlockId, OutSocket, InSocket> = Vec<Edge<BlockId, OutSocket, InSocket>>;
// ----------------------------------------------------------------------------
pub struct Edge<BlockId, OutSocket, InSocket> {
    from: (BlockId, OutSocket),
    to: (BlockId, InSocket),

    // raw coordinates for drawing (end modified while dragged)
    start: ImVec2,
    end: ImVec2,

    draw: EdgeDrawProperties,
}
// ----------------------------------------------------------------------------
pub enum BlockType {
    Source,
    Sink,
    Other,
}
// ----------------------------------------------------------------------------
pub struct Block<BlockId, OutSocket, InSocket> {
    id: BlockId,
    btype: BlockType,
    name: ImString,
    // center position
    pos: (f32, f32),
    size: (f32, f32),
    in_sockets: Vec<InSocket>,
    out_sockets: Vec<OutSocket>,

    tooltip: Vec<ImString>,
    draw: BlockDrawProperties,
}
// ----------------------------------------------------------------------------
pub struct BlockDrawProperties {
    rounding: f32,
    color: u32,
    border_color: u32,
    highlight: bool,
}
// ----------------------------------------------------------------------------
#[derive(Clone)]
pub struct EdgeDrawProperties {
    width: f32,
    color: u32,
}
// ----------------------------------------------------------------------------
pub fn show<BlockId, OutSocket, InSocket>(
    ui: &Ui<'_>,
    canvas_size: (f32, f32),
    plane: &PlaneState<BlockId>,
    blocks: &Blocks<BlockId, OutSocket, InSocket>,
    edges: &[Edge<BlockId, OutSocket, InSocket>],
) -> Option<Interaction<BlockId, OutSocket, InSocket>>
where
    BlockId: Clone + Eq,
    OutSocket: Clone + AsRef<ImStr>,
    InSocket: Clone + AsRef<ImStr>,
{
    show_plane(ui, canvas_size, plane, blocks, edges)
}
// ----------------------------------------------------------------------------
pub fn refresh_edge_positions<BlockId, OutSocket, InSocket>(
    blocks: &Blocks<BlockId, OutSocket, InSocket>,
    edges: &mut Edges<BlockId, OutSocket, InSocket>,
) where
    BlockId: Ord,
    InSocket: Eq,
    OutSocket: Eq,
{
    for edge in edges.iter_mut() {
        // TODO cache slots in edges (don't forget to purge cache on socket changes!)
        if let Some(from) = blocks.get(&edge.from.0) {
            if let Some(slot) = from.find_out_socket_slot(&edge.from.1) {
                edge.start = from.out_socket_position(slot);
            }
        }

        if let Some(to) = blocks.get(&edge.to.0) {
            if let Some(slot) = to.find_in_socket_slot(&edge.to.1) {
                edge.end = to.in_socket_position(slot);
            }
        }
    }
}
// ----------------------------------------------------------------------------
pub fn calculate_graph_dimension<BlockId, OutSocket, InSocket>(
    blocks: &Blocks<BlockId, OutSocket, InSocket>,
) -> Rectangle {
    use std::f32;

    let start = Rectangle {
        min: (f32::MAX, f32::MAX).into(),
        max: (f32::MIN, f32::MIN).into(),
    };

    blocks.values().fold(start, |size, block| Rectangle {
        min: (
            f32::min(size.min.x, block.pos.0 - block.size.0 * 0.5),
            f32::min(size.min.y, block.pos.1 - block.size.1 * 0.5),
        )
            .into(),
        max: (
            f32::max(size.max.x, block.pos.0 + block.size.0 * 0.5),
            f32::max(size.max.y, block.pos.1 + block.size.1 * 0.5),
        )
            .into(),
    })
}
// ----------------------------------------------------------------------------
pub use self::layout::auto_layout_blocks;

pub use self::actions::perform_default_action;
pub use self::actions::update_running_actions;
pub use self::actions::{BlockInteraction, Interaction, Socket};

pub mod action {
    pub use super::actions::*;
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
const GRID_WIDTH_MAX: f32 = 1000.0;

use std::borrow::Borrow;
use std::cmp::Ord;
use std::collections::BTreeMap;

use imgui::sys;
use imgui::{ImStr, ImString, ImVec2, Ui};
use imgui_widgets as widgets;
use imgui_widgets::{GraphWidgets, Rectangle};
// ----------------------------------------------------------------------------
mod actions;
mod layout;
// ----------------------------------------------------------------------------
enum DragOperation {
    Plane(widgets::UiDragging<ImVec2>),
    Block(widgets::UiDragging<ImVec2>, usize),
    Edge(widgets::UiDragging<ImVec2>, usize, bool),
}
// ----------------------------------------------------------------------------
#[inline]
fn show_plane<BlockId, OutSocket, InSocket>(
    ui: &Ui<'_>,
    canvas_size: (f32, f32),
    plane: &PlaneState<BlockId>,
    blocks: &Blocks<BlockId, OutSocket, InSocket>,
    edges: &[Edge<BlockId, OutSocket, InSocket>],
) -> Option<Interaction<BlockId, OutSocket, InSocket>>
where
    BlockId: Clone + Eq,
    OutSocket: Clone + AsRef<ImStr>,
    InSocket: Clone + AsRef<ImStr>,
{
    let mut result = None;
    if let Some(interaction) = ui
        .graph()
        .plane(im_str!("##graphplane"), plane.dim, canvas_size)
        .set_zoom(plane.zoom)
        .set_offset(plane.offset)
        .build(|_| {
            // -- draw edges
            for edge in edges {
                ui.graph()
                    .edge(edge.start, edge.end)
                    .set_color(edge.draw.color)
                    .set_width(edge.draw.width)
                    .set_scale(plane.zoom)
                    .build(plane.offset);
            }
            // -- draw blocks above edges
            if let Some(interaction) = draw_blocks(ui, plane, blocks) {
                result = Some(Interaction::Block(interaction));
            }
        })
    {
        use self::widgets::MouseInteraction::*;

        // prefer block interaction
        if result.is_none() {
            result = match *interaction.mouse() {
                Clicked(0) => Some(Interaction::Deselect),
                Clicked(2) => Some(Interaction::DragStart),
                Wheel(zoom) => Some(Interaction::Zoom(zoom, interaction.position())),
                Released(0) => Some(Interaction::Block(BlockInteraction::EdgeDisconnect)),
                Clicked(1) => Some(Interaction::ContextMenu(interaction.virtual_position())),

                _ => None,
            };
        }
    }

    result
}
// ----------------------------------------------------------------------------
#[inline]
fn draw_blocks<BlockId, OutSocket, InSocket>(
    ui: &Ui<'_>,
    plane: &PlaneState<BlockId>,
    blocks: &Blocks<BlockId, OutSocket, InSocket>,
) -> Option<BlockInteraction<BlockId, OutSocket, InSocket>>
where
    BlockId: Clone + Eq,
    OutSocket: Clone + AsRef<ImStr>,
    InSocket: Clone + AsRef<ImStr>,
{
    let mut result = None;

    let draw_borders = plane.zoom >= 0.4;
    let draw_labels = plane.zoom >= 0.9;

    let draw_sockets = plane.zoom >= 0.6;
    let draw_socket_labels = plane.zoom >= 1.7;

    let is_drag = plane.dragging.is_some();
    let is_edge_drag = is_drag && plane.is_edge_drag_active();

    for (i, block) in blocks.values().enumerate() {
        let selected = matches!(plane.selected, Some(ref id) if *id == block.id);

        if let Some((interaction, socket)) = ui
            .graph()
            .block(
                im_str!("block##{}", i),
                block.name.borrow(),
                block.pos,
                block.size,
            )
            .set_rounding(block.draw.rounding)
            .set_draw_label(draw_labels)
            .set_draw_borders(draw_borders)
            .set_draw_sockets(draw_sockets, draw_socket_labels)
            .set_color(block.draw.color)
            .set_border_color(block.draw.border_color)
            .set_block_highlight(block.draw.highlight)
            .set_in_sockets(&block.in_sockets)
            .set_out_sockets(&block.out_sockets)
            .set_hover_hightlight_socket(!is_drag, !is_drag || (!selected && is_edge_drag))
            .set_hover_socket_labels(!is_drag, !is_drag || (!selected && is_edge_drag))
            .set_scale(plane.zoom)
            .build(plane.offset)
        {
            use self::widgets::graph::BlockSocket;
            use self::widgets::MouseInteraction::*;

            result = match *interaction.mouse() {
                Clicked(0) => match socket {
                    None => Some(BlockInteraction::Select(block.id.clone())),

                    Some(BlockSocket::Out(socket)) if !is_drag => {
                        Some(BlockInteraction::SelectSocket(
                            block.id.clone(),
                            Socket::Out(socket.clone()),
                        ))
                    }

                    Some(BlockSocket::In(socket)) if !is_drag => {
                        Some(BlockInteraction::SelectSocket(
                            block.id.clone(),
                            Socket::In(socket.clone()),
                        ))
                    }
                    _ => None,
                },

                Released(button) if button == 0 && is_edge_drag => match socket {
                    Some(BlockSocket::In(socket)) if !selected => {
                        Some(BlockInteraction::EdgeConnectTo(
                            block.id.clone(),
                            Socket::In(socket.clone()),
                        ))
                    }
                    _ => Some(BlockInteraction::EdgeDragCancel),
                },

                Clicked(1) => Some(BlockInteraction::ContextMenu(block.id.clone())),
                _ => None,
            };
        }
        if plane.show_block_extra_infos
            && !draw_labels
            && !block.tooltip.is_empty()
            && ui.is_item_hovered()
        {
            ui.tooltip(|| {
                let mut tooltip = block.tooltip.iter();
                if let Some(txt) = tooltip.next() {
                    ui.text(txt);
                }
                for txt in tooltip {
                    ui.separator();
                    ui.text(txt);
                }
            });
        }
    }
    result
}
// ----------------------------------------------------------------------------
// colors in imgui are RGBA with range 0 - 1.0
type Color = (f32, f32, f32, f32);
// ----------------------------------------------------------------------------
// precalculated white with slightly lowered alpha
const COL_EDGE: u32 = 0x99FF_FFFF;
const COL_EDGE_HIGHLIGHT_1: Color = (0.0, 0.7, 0.0, 1.0);
const COL_EDGE_HIGHLIGHT_2: Color = (0.0, 0.0, 0.7, 1.0);
// ----------------------------------------------------------------------------
impl<BlockId, OutSocket, InSocket> Block<BlockId, OutSocket, InSocket> {
    // ------------------------------------------------------------------------
    pub fn new(
        block_type: BlockType,
        id: BlockId,
        name: String,
    ) -> Block<BlockId, OutSocket, InSocket> {
        Block {
            id,
            btype: block_type,
            name: ImString::new(name.clone()),
            pos: (0.0, 0.0),
            size: (100.0, 50.0),
            draw: BlockDrawProperties::default(),
            in_sockets: Vec::new(),
            out_sockets: Vec::new(),
            tooltip: vec![ImString::new(name)],
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_draw_properties(&mut self, properties: BlockDrawProperties) -> &mut Self {
        self.draw = properties;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_position(&mut self, pos: (f32, f32)) -> &mut Self {
        self.pos = pos;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_out_sockets(&mut self, sockets: Vec<OutSocket>) {
        self.out_sockets = sockets;
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_in_sockets(&mut self, sockets: Vec<InSocket>) {
        self.in_sockets = sockets;
    }
    // ------------------------------------------------------------------------
    pub fn set_tooltip<T: Into<ImString>>(&mut self, txt: Vec<T>) -> &mut Self {
        self.tooltip.clear();
        for txt in txt {
            self.tooltip.push(txt.into());
        }
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn id(&self) -> &BlockId {
        &self.id
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn pos(&self) -> (f32, f32) {
        self.pos
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn name(&self) -> &ImStr {
        self.name.borrow()
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn in_sockets(&self) -> &Vec<InSocket> {
        &self.in_sockets
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn out_sockets(&self) -> &Vec<OutSocket> {
        &self.out_sockets
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_highlight(&mut self, highlight: bool) {
        self.draw.highlight = highlight;
    }
    // ------------------------------------------------------------------------
    fn find_in_socket_slot(&self, socket: &InSocket) -> Option<usize>
    where
        InSocket: Eq,
    {
        self.in_sockets
            .iter()
            .enumerate()
            .find(|&(_, s)| s == socket)
            .map(|(slot, _)| slot)
    }
    // ------------------------------------------------------------------------
    fn find_out_socket_slot(&self, socket: &OutSocket) -> Option<usize>
    where
        OutSocket: Eq,
    {
        self.out_sockets
            .iter()
            .enumerate()
            .find(|&(_, s)| s == socket)
            .map(|(slot, _)| slot)
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn in_socket_position(&self, socket_slot: usize) -> ImVec2 {
        self::widgets::graph::calculate_in_socket_position(
            socket_slot,
            self.in_sockets.len(),
            self.draw.rounding,
            self.pos.into(),
            self.size.into(),
        )
    }
    // ------------------------------------------------------------------------
    #[inline]
    fn out_socket_position(&self, socket_slot: usize) -> ImVec2 {
        self::widgets::graph::calculate_out_socket_position(
            socket_slot,
            self.out_sockets.len(),
            self.draw.rounding,
            self.pos.into(),
            self.size.into(),
        )
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl BlockDrawProperties {
    // ------------------------------------------------------------------------
    pub fn new<C: Into<sys::ImVec4>>(
        rounding: f32,
        color: C,
        border_color: C,
    ) -> BlockDrawProperties {
        BlockDrawProperties {
            rounding,
            color: unsafe { sys::igGetColorU32Vec(&color.into()) },
            border_color: unsafe { sys::igGetColorU32Vec(&border_color.into()) },
            highlight: false,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<B, O, I> Edge<B, O, I> {
    // ------------------------------------------------------------------------
    pub fn new(from: B, out_socket: O, to: B, in_socket: I) -> Edge<B, O, I> {
        Edge {
            from: (from, out_socket),
            to: (to, in_socket),

            start: ImVec2::zero(),
            end: ImVec2::zero(),
            draw: EdgeDrawProperties::default(),
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn from(&self) -> &(B, O) {
        &self.from
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn to(&self) -> &(B, I) {
        &self.to
    }
    // ------------------------------------------------------------------------
    pub fn highlight_as_incoming(&mut self) {
        self.draw.color = unsafe { sys::igGetColorU32Vec(&COL_EDGE_HIGHLIGHT_1.into()) };
        self.draw.width = 2.0;
    }
    // ------------------------------------------------------------------------
    pub fn highlight_as_outgoing(&mut self) {
        self.draw.color = unsafe { sys::igGetColorU32Vec(&COL_EDGE_HIGHLIGHT_2.into()) };
        self.draw.width = 2.0;
    }
    // ------------------------------------------------------------------------
    pub fn unhighlight(&mut self) {
        self.draw.color = COL_EDGE;
        self.draw.width = 1.0
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<BlockId> PlaneState<BlockId> {
    // ------------------------------------------------------------------------
    pub fn new(size: Rectangle) -> PlaneState<BlockId> {
        let mut p = PlaneState::default();
        p.set_dim(size);
        // approximation
        p.set_canvas_size((
            (size.max.x - size.min.x) / 10.0,
            (size.max.y - size.min.y) / 10.0,
        ));
        p
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn zoom(&self) -> f32 {
        self.zoom
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn offset(&self) -> ImVec2 {
        self.offset
    }
    // ------------------------------------------------------------------------
    pub fn is_edge_drag_active(&self) -> bool {
        matches!(self.dragging, Some(DragOperation::Edge(_, _, _)))
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_offset(&mut self, offset: (f32, f32)) -> &mut Self {
        self.offset = offset.into();
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_zoom(&mut self, zoom: f32) -> &mut Self {
        self.zoom = f32::max(
            self.zoom_restriction.0,
            f32::min(self.zoom_restriction.1, zoom),
        );
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_grid_width(&mut self, grid: (f32, f32)) -> &mut Self {
        self.grid = (
            grid.0.clamp(1.0, GRID_WIDTH_MAX),
            grid.1.clamp(1.0, GRID_WIDTH_MAX),
        );
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_canvas_size(&mut self, size: (f32, f32)) -> &mut Self {
        self.canvas_size = size;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_dim(&mut self, size: Rectangle) -> &mut Self {
        self.dim = size;
        self
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn set_show_block_tooltip(&mut self, show: bool) -> &mut Self {
        self.show_block_extra_infos = show;
        self
    }
    // ------------------------------------------------------------------------
    pub fn auto_center(&mut self) {
        self.offset = self.dim.min
            + (self.dim.max - self.dim.min - ImVec2::from(self.canvas_size) / self.zoom) * 0.5;
    }
    // ------------------------------------------------------------------------
    pub fn set_zoom_with_fixpoint(&mut self, zoom: f32, fixpoint: ImVec2) {
        let prev_zoom = self.zoom;
        self.set_zoom(zoom);
        // some math...
        let new_offset =
            self.offset - fixpoint * ((prev_zoom - self.zoom) / (prev_zoom * self.zoom));

        self.offset = ImVec2::new(
            f32::min(
                self.dim.max.x - self.canvas_size.0 / self.zoom,
                f32::max(self.dim.min.x, new_offset.x),
            ),
            f32::min(
                self.dim.max.y - self.canvas_size.1 / self.zoom,
                f32::max(self.dim.min.y, new_offset.y),
            ),
        );
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
// Default trait impl
// ----------------------------------------------------------------------------
impl<BlockId> Default for PlaneState<BlockId> {
    fn default() -> PlaneState<BlockId> {
        PlaneState {
            offset: (0.0, 0.0).into(),
            dim: Rectangle {
                min: (0.0, 0.0).into(),
                max: (500.0, 500.0).into(),
            },
            zoom: 1.0,
            zoom_restriction: (0.2, 3.0),
            grid: (1.0, 1.0),
            canvas_size: (500.0, 500.0),
            show_block_extra_infos: true,
            dragging: None,
            selected: None,
        }
    }
}
// ----------------------------------------------------------------------------
impl Default for BlockDrawProperties {
    fn default() -> BlockDrawProperties {
        BlockDrawProperties {
            rounding: 0.0,
            // precalculated white
            color: 0xFFFF_FFFF,
            border_color: 0xFFFF_FFFF,
            highlight: false,
        }
    }
}
// ----------------------------------------------------------------------------
impl Default for EdgeDrawProperties {
    fn default() -> EdgeDrawProperties {
        EdgeDrawProperties {
            width: 1.0,
            color: COL_EDGE,
        }
    }
}
// ----------------------------------------------------------------------------
