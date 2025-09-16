//
// graph::actions
//

// TODO: update all BlockId trait guards with trait alias as soon as its stable

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum Interaction<BlockId, OutSocket, InSocket> {
    Block(BlockInteraction<BlockId, OutSocket, InSocket>),
    Deselect,
    DragStart,
    DragStop,
    Zoom(f32, ImVec2),
    ContextMenu(ImVec2),
    None,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub enum BlockInteraction<BlockId, OutSocket, InSocket> {
    Select(BlockId),
    SelectSocket(BlockId, Socket<OutSocket, InSocket>),
    EdgeDragCancel,
    EdgeDisconnect,
    EdgeConnectTo(BlockId, Socket<OutSocket, InSocket>),
    ContextMenu(BlockId),
}
// ----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub enum Socket<OutSocket, InSocket> {
    Out(OutSocket),
    In(InSocket),
}
// ----------------------------------------------------------------------------
pub fn update_running_actions<BlockId, OutSocket, InSocket>(
    plane: &mut PlaneState<BlockId>,
    blocks: &mut Blocks<BlockId, OutSocket, InSocket>,
    edges: &mut Edges<BlockId, OutSocket, InSocket>,
) -> Option<Interaction<BlockId, OutSocket, InSocket>>
where
    BlockId: Clone + Eq + Hash + Ord,
    InSocket: Eq,
    OutSocket: Eq,
{
    let mut drag_stopped = None;
    if let Some(ref mut active_dragging) = plane.dragging {
        let pzoom = plane.zoom;

        let drag_active = match *active_dragging {
            DragOperation::Plane(ref mut drag) => {
                let poffset = &mut plane.offset;
                drag.update(|offset, delta| *poffset = *offset - delta / pzoom)
            }
            DragOperation::Block(ref mut drag, slot) => {
                let grid = plane.grid;
                blocks.values_mut().skip(slot).take(1).for_each(|block| {
                    drag.update(|offset, delta| {
                        let pos = *offset + delta / pzoom;
                        block.pos = (
                            (pos.x / grid.0).round() * grid.0,
                            (pos.y / grid.1).round() * grid.1,
                        );
                    });
                });
                super::refresh_edge_positions(blocks, edges);
                drag.active()
            }
            DragOperation::Edge(ref mut drag, slot, _) => {
                if let Some(edge) = edges.get_mut(slot) {
                    drag.update(|offset, delta| {
                        let pos = *offset + delta / pzoom;
                        edge.end = (pos.x, pos.y).into();
                    });
                }
                if !drag.active() {
                    super::refresh_edge_positions(blocks, edges);
                }
                drag.active()
            }
        };
        if !drag_active {
            drag_stopped = Some(Interaction::DragStop);
        }
    }
    drag_stopped
}
// ----------------------------------------------------------------------------
pub fn perform_default_action<Id, OutSocket, InSocket>(
    interaction: Interaction<Id, OutSocket, InSocket>,
    plane: &mut PlaneState<Id>,
    blocks: &mut Blocks<Id, OutSocket, InSocket>,
    edges: &mut Edges<Id, OutSocket, InSocket>,
) -> Option<Id>
where
    Id: Clone + Eq + Hash + Ord,
    OutSocket: Clone + Eq,
    InSocket: Clone + Eq + Default,
{
    match interaction {
        Interaction::Deselect => deselect_block(plane, blocks, edges),
        Interaction::Zoom(direction, fixpoint) => adjust_zoom(plane, direction, fixpoint),
        Interaction::DragStart => start_plane_drag(plane),
        Interaction::DragStop => plane.dragging = None,
        Interaction::Block(interaction) => match interaction {
            BlockInteraction::Select(ref id) => {
                select_block(id, plane, blocks, edges);
                start_block_drag(plane, id, blocks);
            }
            BlockInteraction::SelectSocket(ref blockid, ref socket) => match *socket {
                Socket::Out(ref socket) => {
                    return create_new_edge_drag(plane, blockid, socket, blocks, edges);
                }
                Socket::In(ref socket) => {
                    return start_edge_end_drag(plane, blockid, socket, blocks, edges);
                }
            },
            BlockInteraction::EdgeDragCancel => {
                cancel_edge_drag(plane, blocks, edges);
            }
            BlockInteraction::EdgeDisconnect => {
                remove_edge(plane, edges);
            }
            BlockInteraction::EdgeConnectTo(blockid, socket) => {
                if let Socket::In(socket) = socket {
                    connect_edge_to(plane, blockid, socket, edges);
                    super::refresh_edge_positions(blocks, edges);
                }
            }
            BlockInteraction::ContextMenu(_) => {}
        },
        Interaction::ContextMenu(_) => {}
        Interaction::None => {}
    }
    None
}
// ----------------------------------------------------------------------------
pub fn deselect_block<Id, OutSocket, InSocket>(
    plane: &mut PlaneState<Id>,
    blocks: &mut Blocks<Id, OutSocket, InSocket>,
    edges: &mut Edges<Id, OutSocket, InSocket>,
) where
    Id: Clone + Eq + Hash + Ord,
{
    blocks
        .values_mut()
        .for_each(|block| block.set_highlight(false));
    edges.iter_mut().for_each(Edge::unhighlight);

    plane.selected = None;
}
// ----------------------------------------------------------------------------
pub fn select_block<Id, OutSocket, InSocket>(
    blockid: &Id,
    plane: &mut PlaneState<Id>,
    blocks: &mut Blocks<Id, OutSocket, InSocket>,
    edges: &mut Edges<Id, OutSocket, InSocket>,
) where
    Id: Clone + Eq + Hash + Ord,
{
    edges.iter_mut().for_each(|edge| {
        if edge.from.0 == *blockid {
            edge.highlight_as_outgoing();
        } else if edge.to().0 == *blockid {
            edge.highlight_as_incoming();
        } else {
            edge.unhighlight();
        }
    });

    blocks
        .iter_mut()
        .for_each(|(id, block)| block.set_highlight(id == blockid));

    plane.selected = Some(blockid.clone());
}
// ----------------------------------------------------------------------------
pub fn start_block_drag<Id, OutSocket, InSocket>(
    plane: &mut PlaneState<Id>,
    id: &Id,
    blocks: &Blocks<Id, OutSocket, InSocket>,
) where
    Id: Eq,
{
    if let Some((slot, block)) = blocks
        .values()
        .enumerate()
        .find(|&(_, block)| *id == block.id)
    {
        plane.dragging = Some(DragOperation::Block(
            UiDragging::start(ImVec2::from(block.pos)),
            slot,
        ));
    }
}
// ----------------------------------------------------------------------------
pub fn start_plane_drag<BlockId>(plane: &mut PlaneState<BlockId>) {
    plane.dragging = Some(DragOperation::Plane(UiDragging::start(plane.offset)));
}
// ----------------------------------------------------------------------------
pub fn adjust_zoom<BlockId>(plane: &mut PlaneState<BlockId>, direction: f32, fixpoint: ImVec2) {
    let new_zoom = plane.zoom() + 0.1 * direction;
    plane.set_zoom_with_fixpoint(new_zoom, fixpoint);
}
// ----------------------------------------------------------------------------
// edge handling
// ----------------------------------------------------------------------------
pub fn create_new_edge_drag<BlockId, OutSocket, InSocket>(
    plane: &mut PlaneState<BlockId>,
    blockid: &BlockId,
    socket: &OutSocket,
    blocks: &mut Blocks<BlockId, OutSocket, InSocket>,
    edges: &mut Edges<BlockId, OutSocket, InSocket>,
) -> Option<BlockId>
where
    BlockId: Clone + Eq + Hash + Ord,
    InSocket: Clone + Eq + Default,
    OutSocket: Clone + Eq,
{
    // dragging from an out socket always creates a new edge
    edges.push(Edge::new(
        blockid.clone(),
        socket.clone(),
        blockid.clone(),
        InSocket::default(),
    ));
    let slot = edges.len() - 1;

    // new edge starts at out socket but end points to mouse position
    super::refresh_edge_positions(blocks, edges);

    if let Some(edge) = edges.get_mut(slot) {
        // new edge must be highlighted
        edge.highlight_as_outgoing();
        edge.end = edge.start;

        plane.dragging = Some(DragOperation::Edge(UiDragging::start(edge.end), slot, true));
    }
    if Some(blockid) != plane.selected.as_ref() {
        select_block(blockid, plane, blocks, edges);
        Some(blockid.clone())
    } else {
        None
    }
}
// ----------------------------------------------------------------------------
pub fn start_edge_end_drag<BlockId, OutSocket, InSocket>(
    plane: &mut PlaneState<BlockId>,
    blockid: &BlockId,
    socket: &InSocket,
    blocks: &mut Blocks<BlockId, OutSocket, InSocket>,
    edges: &mut Edges<BlockId, OutSocket, InSocket>,
) -> Option<BlockId>
where
    BlockId: Clone + Eq + Hash + Ord,
    InSocket: Eq,
{
    // find all edges pointing to this socket
    //  a) if none exists, don't start a drag
    //  a) otherwise pick the one which starts in currently selected block
    //  b) otherwise pick one and select the block it starts from
    let new_start_block: Option<BlockId> = {
        let candidates = edges
            .iter()
            .enumerate()
            .filter(|&(_, edge)| (blockid, socket) == (&edge.to.0, &edge.to.1))
            .collect::<Vec<_>>();

        let edge = candidates
            .iter()
            .find(|&&(_, edge)| plane.selected.as_ref() == Some(&edge.from.0))
            .or_else(|| candidates.first());

        if let Some(&(slot, edge)) = edge {
            plane.dragging = Some(DragOperation::Edge(
                UiDragging::start(edge.end),
                slot,
                false,
            ));
        }

        edge.iter()
            .filter(|&&&(_, edge)| Some(&edge.from.0) != plane.selected.as_ref())
            .map(|&&(_, edge)| edge.from.0.clone())
            .next()
    };
    if let Some(ref new_start_block) = new_start_block {
        select_block(new_start_block, plane, blocks, edges);
    }
    new_start_block
}
// ----------------------------------------------------------------------------
pub fn cancel_edge_drag<BlockId, OutSocket, InSocket>(
    plane: &mut PlaneState<BlockId>,
    blocks: &Blocks<BlockId, OutSocket, InSocket>,
    edges: &mut Edges<BlockId, OutSocket, InSocket>,
) where
    BlockId: Clone + Eq + Hash + Ord,
    InSocket: Eq,
    OutSocket: Eq,
{
    if let Some(DragOperation::Edge(_, _, new_edge)) = plane.dragging {
        if new_edge {
            remove_edge(plane, edges);
        }
    }
    plane.dragging = None;
    super::refresh_edge_positions(blocks, edges);
}
// ----------------------------------------------------------------------------
pub fn remove_edge<BlockId, OutSocket, InSocket>(
    plane: &mut PlaneState<BlockId>,
    edges: &mut Edges<BlockId, OutSocket, InSocket>,
) {
    if let Some(DragOperation::Edge(_, slot, _)) = plane.dragging {
        if edges.len() > slot {
            edges.remove(slot);
            plane.dragging = None;
        }
    }
}
// ----------------------------------------------------------------------------
pub fn connect_edge_to<BlockId, OutSocket, InSocket>(
    plane: &mut PlaneState<BlockId>,
    blockid: BlockId,
    socket: InSocket,
    edges: &mut Edges<BlockId, OutSocket, InSocket>,
) where
    BlockId: Eq,
    InSocket: Eq,
    OutSocket: Eq,
{
    if let Some(DragOperation::Edge(_, slot, _)) = plane.dragging {
        let new_target = (blockid, socket);

        // if the edge is a loop (from == to) or if such an edge already exists
        // remove it completely - otherwise it's a new connection
        let remove_edge = match edges.get(slot) {
            Some(edge) => {
                edge.from.0 == new_target.0
                    || edges
                        .iter()
                        .enumerate()
                        .any(|(i, e)| i != slot && e.from == edge.from && e.to == new_target)
            }
            _ => false,
        };

        if remove_edge {
            edges.remove(slot);
        } else if let Some(edge) = edges.get_mut(slot) {
            edge.to = new_target
        }
        plane.dragging = None;
    }
}
// ----------------------------------------------------------------------------
use std::hash::Hash;

use super::widgets::UiDragging;
use imgui::ImVec2;

use super::DragOperation;
use super::{Blocks, Edge, Edges, PlaneState};
// ----------------------------------------------------------------------------
