// ----------------------------------------------------------------------------
// graph::layout
// ----------------------------------------------------------------------------

// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub fn auto_layout_blocks<BlockId, OutSocket, InSocket>(
    blocks: &mut Blocks<BlockId, OutSocket, InSocket>,
    edges: &mut Edges<BlockId, OutSocket, InSocket>,
) -> super::Rectangle
where
    BlockId: Clone + Eq + hash::Hash + cmp::Ord + ::std::fmt::Debug,
    InSocket: Eq + ::std::fmt::Debug,
    OutSocket: Eq,
{
    //TODO as constant
    let distance_x = 200.0;
    let distance_y = 100.0;
    let start_x = 100.0;
    let start_y = 100.0;

    // debugging: color marking of back and cross edges
    // #[cfg(debug_assertions)]
    let _cloned_back_edges: Vec<(BlockId, BlockId)>;
    let _cloned_cross_edges: Vec<(BlockId, BlockId)>;

    //
    // basic layouting idea - the implementation diverges here and there
    //
    //  0. find start blocks
    //      - (at least one) source block(s) always exist
    //      - orphan blocks/subgraphs may exist, too
    //
    //  1. layout x-position (graph depth slots) of all blocks
    //      - detect cycles (backedges) and ignore them
    //      - for all links pointing to a block (except backedges) push block to
    //        minimum depth greater than any of its parents
    //        (reduces unnecessary edge-"back" drawing)
    //
    //  2. layout y-position (graph breadth slots) (inspired by Reingold-Tilford algorithm)
    //      - remove all backedges and cross edges for further processing
    //        making the graph into a forest (assuming an invisible root node graph is now a tree)
    //      - starting at tree bottom:
    //          - for every block position its child blocks equally distant from
    //            each other (for this *local* depth level only!)
    //          - center every parent based on number of children by calculating
    //            a local modifier offset (half of child block level breadth)
    //          - for every block with children check and move adjacent subtree
    //            so they do not overlap (calculate appropriate subtree contours)
    //          - TODO: after any subtree move recenter enclosed child blocks (subtrees)
    //      - starting at the top:
    //          - merge local breadth slots with subtree centering offsets to global
    //            breadth slots for every block (by adding up all offsets down the path)
    //
    //  3. scale depth and breadth slots by defined constants to calculate absolute pixel positions
    //
    // as there is no "virtual" root node to "join" graph start blocks this is handled
    // as a special case
    //

    {
        // since datastructure for blocks and edges is disconnected prepare
        // lookup tables (easier to iterate/search)
        let (mut block_links, linked_to) = {
            let mut linked_to = HashMap::new();
            let mut links_from: HashMap<&BlockId, Vec<&BlockId>> = HashMap::new();

            for edge in edges.iter() {
                let entry = links_from.entry(&edge.from.0).or_default();
                entry.push(&edge.to.0);

                let entry = linked_to.entry(&edge.to.0).or_insert_with(HashSet::new);
                entry.insert(&edge.from.0);
            }

            // sort by position of target block, dedupe afterwards
            // TODO maybe sort by outsocket position, too
            for links in links_from.values_mut() {
                // sort by y position of target block
                links.sort_by(|a, b| {
                    let a = blocks.get(a).map(|block| block.pos.1).unwrap_or(0.0);
                    let b = blocks.get(b).map(|block| block.pos.1).unwrap_or(0.0);
                    b.partial_cmp(&a).unwrap()
                });

                links.dedup();
            }
            (links_from, linked_to)
        };

        let block_pos = {
            // 0. find all start points

            let root_nodes = find_start_blocks(blocks, &linked_to, distance_x);

            // 1. layout depth (x-position) -----------------------------------

            let (mut block_pos, back_edges, cross_edges) =
                layout_block_depths(&root_nodes, &block_links, &linked_to);

            // remove all back and cross edges to have a tree (forest)
            for (src, links) in block_links.iter_mut() {
                *links = links
                    .iter()
                    .filter(|target| {
                        !back_edges.contains(&(src, target))
                            && !cross_edges.contains(&(src, target))
                    })
                    .cloned()
                    .collect();
            }

            // debugging: color marking of back and cross edges
            // (cloning required because of borrow checker)
            #[cfg(debug_assertions)]
            {
                _cloned_back_edges = back_edges
                    .iter()
                    .map(|(src, target)| ((*src).clone(), (*target).clone()))
                    .collect::<Vec<_>>();
                _cloned_cross_edges = cross_edges
                    .iter()
                    .map(|(src, target)| ((*src).clone(), (*target).clone()))
                    .collect::<Vec<_>>();
            }

            // 2. layout breadth (y-position) ---------------------------------

            layout_block_breadth(&root_nodes, &block_links, &mut block_pos);

            block_pos
        };

        // 3. scale depth and breadth to absolute pixel positions -------------
        let base_y = start_y + 0.5 * distance_y;

        for block in blocks.values_mut() {
            if let Some(pos) = block_pos.get(&block.id) {
                block.pos.0 = start_x + distance_x * pos.depth_slot as f32;
                block.pos.1 = base_y + distance_y * pos.breadth_slot;
            } else {
                block.pos.0 = -start_x;
                block.pos.1 = -base_y;
            }
        }
    }

    // debugging: color marking of back and cross edges
    #[cfg(debug_assertions)]
    for edge in edges.iter_mut() {
        if _cloned_back_edges.contains(&(edge.from.0.clone(), edge.to.0.clone())) {
            edge.draw.color = 0xFFFF_00FF;
        }
        if _cloned_cross_edges.contains(&(edge.from.0.clone(), edge.to.0.clone())) {
            edge.draw.color = 0xFF00_00FF;
        }
    }

    super::refresh_edge_positions(blocks, edges);
    super::calculate_graph_dimension(blocks)
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use std::cmp;
use std::hash;

use std::collections::{HashMap, HashSet, VecDeque};

use super::{BlockType, Blocks, Edges};
// ----------------------------------------------------------------------------
struct BlockPos {
    /// global X slot
    depth_slot: i32,
    /// local Y slot
    breadth_slot: f32,
    /// centering offset
    subtree_offset: f32,
    /// maximum depth of subtree
    subtree_depth: i32,
}
// ----------------------------------------------------------------------------
impl BlockPos {
    // ------------------------------------------------------------------------
    fn new<D: Into<i32>, B: Into<f32>>(depth_slot: D, breadth_slot: B) -> BlockPos {
        let depth = depth_slot.into();
        BlockPos {
            depth_slot: depth,
            breadth_slot: breadth_slot.into(),
            subtree_depth: depth,
            subtree_offset: 0.0,
        }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
struct RootNodeInfo<'b, BlockId> {
    id: &'b BlockId,
    depth_slot: i32,
    breadth_slot: i32,
}
// ----------------------------------------------------------------------------
fn find_start_blocks<'b, BlockId, OutSocket, InSocket>(
    blocks: &'b Blocks<BlockId, OutSocket, InSocket>,
    linked_to: &HashMap<&BlockId, HashSet<&BlockId>>,
    grid_width: f32,
) -> Vec<RootNodeInfo<'b, BlockId>>
where
    BlockId: Eq + hash::Hash + cmp::Ord,
{
    // find all start points
    //  - source blocks (depth := 0)
    //  - AND (if there are some orphan subgraphs) blocks without links to it
    //  - sort by current y-position
    let mut list: Vec<RootNodeInfo<'b, BlockId>> = blocks
        .values()
        .filter(|block| !linked_to.contains_key(&block.id))
        .enumerate()
        .map(|(node_pos, block)| {
            let depth_slot = match block.btype {
                BlockType::Source => 0,
                // use *current* depth position as initial pos for orphan blocks
                // those will move only vertically
                _ => f32::max(0.0, f32::trunc(block.pos.0 / grid_width)) as i32,
            };
            RootNodeInfo {
                id: &block.id,
                depth_slot,
                breadth_slot: node_pos as i32,
            }
        })
        .collect();

    // sort by y position
    list.sort_by(|a, b| {
        let a = blocks.get(a.id).map(|block| block.pos.1).unwrap_or(0.0);
        let b = blocks.get(b.id).map(|block| block.pos.1).unwrap_or(0.0);
        b.partial_cmp(&a).unwrap()
    });

    list
}
// ----------------------------------------------------------------------------
#[allow(clippy::type_complexity)]
fn layout_block_depths<'b, BlockId>(
    root_nodes: &[RootNodeInfo<'b, BlockId>],
    block_links: &HashMap<&'b BlockId, Vec<&'b BlockId>>,
    linked_to: &HashMap<&'b BlockId, HashSet<&'b BlockId>>,
) -> (
    HashMap<BlockId, BlockPos>,
    HashSet<(&'b BlockId, &'b BlockId)>,
    HashSet<(&'b BlockId, &'b BlockId)>,
)
where
    BlockId: Clone + Eq + hash::Hash + cmp::Ord + ::std::fmt::Debug,
{
    let mut back_edges = HashSet::new();
    let mut cross_edges: HashSet<(&BlockId, &BlockId)> = HashSet::new();

    let (mut run1_queue, mut run2_queue): (Vec<_>, VecDeque<_>) = root_nodes
        .iter()
        .map(|info| {
            (
                (None, info.id, info.depth_slot, false),
                (info.id, info.depth_slot, info.depth_slot),
            )
        })
        .unzip();

    // a) detect back/cross edges and set initial depths ----------------------

    // used to detect back edges (basically a recursion stack)
    let mut path_stack = HashSet::new();
    let mut tmp_depth = HashMap::new();

    while let Some((parent, current, current_depth, unwrapped)) = run1_queue.pop() {
        if unwrapped {
            path_stack.remove(current);
        } else {
            path_stack.insert(current);

            if !tmp_depth.contains_key(current) {
                // set initial depth slot
                tmp_depth.insert(current.clone(), (current_depth, current_depth));

                run1_queue.push((parent, current, current_depth, true));

                if let Some(links) = block_links.get(current) {
                    for target in links.iter().rev() {
                        if path_stack.contains(target) {
                            back_edges.insert((current, *target));
                        } else if let Some((_, pushed_depth)) = tmp_depth.get_mut(target) {
                            cross_edges.insert((current, target));

                            *pushed_depth = (*pushed_depth).max(current_depth + 1);
                        } else {
                            run1_queue.push((Some(current), target, current_depth + 1, false))
                        }
                    }
                }
            } else {
                cross_edges.insert((parent.unwrap(), current));
            }
        }
    }

    // b) calculate max depth and propagate delta down the subtree ------------

    let mut block_pos: HashMap<BlockId, BlockPos> = HashMap::new();

    while let Some((current, node_depth, pushed_depth)) = run2_queue.pop_front() {
        // get the maximum depth of all predecessor nodes, ignore back edges
        let final_node_depth = {
            if let Some(links) = linked_to.get(current) {
                links
                    .iter()
                    .filter(|pred| !back_edges.contains(&(pred, current)))
                    .map(|pred| {
                        let depths = tmp_depth.get(pred).expect("no depth for predecesor node");
                        depths.0.max(depths.1) + 1
                    })
                    .max()
                    .unwrap_or(node_depth)
                    .max(node_depth.max(pushed_depth))
            } else {
                node_depth.max(pushed_depth)
            }
        };

        if !block_pos.contains_key(current) {
            block_pos.insert(current.clone(), BlockPos::new(final_node_depth, -1.0));
            // update tmp depths so subsequent check over all predecessor nodes
            // has current data
            tmp_depth
                .get_mut(current)
                .expect("no depth for current node to update")
                .0 = final_node_depth;
        }

        // iterate only over linked child nodes which still don't have a position
        if let Some(links) = block_links.get(current) {
            let delta = final_node_depth - node_depth;

            for target in links.iter().filter(|t| !block_pos.contains_key(**t)) {
                let (node_depth, node_max_depth) = tmp_depth
                    .get(target)
                    .expect("merge and propagate depth: missing depth a block");

                run2_queue.push_front((target, *node_depth, node_max_depth + delta));
            }
        }
    }

    // ensure there are no negative depths!
    let max_depth = block_pos.values().map(|b| b.depth_slot).max().unwrap_or(0);
    let min_depth = block_pos.values().map(|b| b.depth_slot).min().unwrap_or(0);

    assert!(min_depth >= 0, "expected min depth to be >= 0");
    assert!(max_depth >= 0, "expected max depth to be >= 0");

    (block_pos, back_edges, cross_edges)
}
// ----------------------------------------------------------------------------
enum LayoutBlockType<'b, BlockId> {
    Virtual,
    Normal(&'b BlockId),
}
// ----------------------------------------------------------------------------
fn layout_block_breadth<'b, BlockId>(
    root_nodes: &[RootNodeInfo<'b, BlockId>],
    block_links: &HashMap<&'b BlockId, Vec<&'b BlockId>>,
    block_pos: &mut HashMap<BlockId, BlockPos>,
) where
    BlockId: Clone + Eq + hash::Hash + cmp::Ord + ::std::fmt::Debug,
{
    // a) preprocessing:
    //  - extract local subtree depth (max depth) for every node
    //  - assign initial (local) breadth slot for every node
    //  - create postorder run for subsequent processing
    let mut postorder = Vec::new();

    fn traverse_postorder<'a, BlockId: Eq + hash::Hash>(
        postorder: &mut Vec<LayoutBlockType<'a, BlockId>>,
        node: &'a BlockId,
        pos: f32,
        block_pos: &mut HashMap<BlockId, BlockPos>,
        block_links: &HashMap<&'a BlockId, Vec<&'a BlockId>>,
    ) -> i32 {
        let mut max_depth = 0;

        if let Some(links) = block_links.get(node) {
            for (pos, target) in links.iter().rev().enumerate() {
                max_depth = max_depth.max(traverse_postorder(
                    postorder,
                    *target,
                    f32::from(pos as u16),
                    block_pos,
                    block_links,
                ));
            }
        }
        let info = block_pos.get_mut(node).expect("missing block pos info");

        info.breadth_slot = pos;
        info.subtree_depth = info.depth_slot.max(max_depth);

        postorder.push(LayoutBlockType::Normal(node));
        info.subtree_depth
    }

    let mut tree_depth = 0;
    let mut virtual_childnodes = Vec::new();

    for (id, pos) in root_nodes.iter().map(|info| (info.id, info.breadth_slot)) {
        tree_depth = tree_depth.max(traverse_postorder(
            &mut postorder,
            id,
            f32::from(pos as u16),
            block_pos,
            block_links,
        ));
        virtual_childnodes.push(id);
    }

    // at this point blocks are sorted "top to bottom" (as seen on screen), e.g:
    //
    //      0
    //     /
    //    2-1
    //   /
    //  7-3
    //   \
    //    6-4
    //     \
    //      5
    //

    // b) move subtree to prevent child subtree overlapping -------------------

    let mut left_contour = vec![f32::MIN; tree_depth as usize + 1].into_boxed_slice();
    let mut right_contour = vec![f32::MAX; tree_depth as usize + 1].into_boxed_slice();

    // special handling of virtual root by providing the nodes directly as links
    let mut root_subtree_offset = 0.0;
    postorder.push(LayoutBlockType::Virtual);

    for current in postorder {
        let mut subtree_width = 0.0;

        let links = match current {
            LayoutBlockType::Normal(id) => block_links.get(id),
            LayoutBlockType::Virtual => Some(&virtual_childnodes),
        };

        // if let Some(links) = block_links.get(current) {
        if let Some(links) = links {
            // more than one subtree -> check overlapping of all
            if links.len() > 1 {
                // (ascii art rotated clockwise by 90° to save space)
                // - extract left contour for first child c_1 and right contour for its
                //   left sibling c_2
                // - repeat with next pair (c_2 and c_3) until (c_n-1 and c_n)
                //
                //     (c_n)    ...       (c_2)           (c_1)
                //
                //       r                  r               l
                //      / \                / \             / \
                //     *   r              *   r           l   *
                //    /|\                / \                 / \
                //   * * r              *   r               l   *
                //      / \                / \             / \
                //     *   r              *   r           l   *
                //        / \                            / \
                //       *   r                          l   *
                //

                left_contour.iter_mut().for_each(|x| *x = f32::MIN);

                let extract_contour = |left_contour,
                                       block_info: &HashMap<_, BlockPos>,
                                       id: &BlockId,
                                       start_depth,
                                       result: &mut [f32]| {
                    let mut queue = VecDeque::new();
                    queue.push_front((id, 0.0));

                    let mut d: usize = start_depth;

                    while let Some((current, offset)) = queue.pop_front() {
                        let info = block_info.get(current).unwrap();
                        let d2 = info.depth_slot as usize;

                        // update all slots in case the node was pushed down
                        while d <= d2 {
                            result[d] = if left_contour {
                                result[d].max(info.breadth_slot + offset)
                            } else {
                                result[d].min(info.breadth_slot + offset)
                            };
                            d += 1;
                        }

                        // if d > max_depth {
                        //     break
                        // }

                        // next depth level
                        if let Some(links) = block_links.get(current) {
                            let mut child_queue = Vec::with_capacity(links.len());
                            let mut max_child_depth = 0;

                            if left_contour {
                                // always pick the left most subtree first, add right sibling(s)
                                // only if it's a deeper subtree
                                //
                                // first collection will be a stack: leftmost child at bottom
                                for target in links.iter() {
                                    let child_depth =
                                        block_info.get(*target).unwrap().subtree_depth;

                                    if child_depth > max_child_depth {
                                        max_child_depth = child_depth;
                                        child_queue.push(target);
                                    }
                                }
                            } else {
                                // always pick the right most subtree first, add left sibling(s)
                                // only if it's a deeper subtree
                                //
                                // first collection will be a stack: rightmost child at bottom
                                for target in links.iter().rev() {
                                    let child_depth =
                                        block_info.get(*target).unwrap().subtree_depth;

                                    if child_depth > max_child_depth {
                                        max_child_depth = child_depth;
                                        child_queue.push(target);
                                    }
                                }
                            };
                            // *prepend* stack to main queue *before* queued
                            // children (of parent)
                            for child in child_queue.iter().rev() {
                                queue.push_front((child, offset + info.subtree_offset));
                            }
                        }
                    }
                };

                let mut links = links.iter().rev();
                // since links.len() > 1 there are at least two elements -> unwrap is safe
                let mut left = links.next().unwrap();

                for right in links {
                    let min_depth = {
                        let left_root = block_pos
                            .get(left)
                            .expect("left subtree root blockpos missing");

                        let right_root = block_pos
                            .get(right)
                            .expect("right root subtree block pos missing");

                        left_root.depth_slot.min(right_root.depth_slot) as usize
                    };

                    // reset contour values (not required for left as they always grow)
                    right_contour.iter_mut().for_each(|x| *x = f32::MAX);

                    extract_contour(true, block_pos, left, min_depth, &mut left_contour);
                    extract_contour(false, block_pos, right, min_depth, &mut right_contour);

                    // find max overlapping
                    let offset = left_contour
                        .iter()
                        .zip(right_contour.iter())
                        .skip(min_depth)
                        .fold(0.0, |acc: f32, (left, right)| acc.max(left - right + 1.0));

                    // TODO recenter enclosed subtrees

                    // push right sibling if overlapping was found
                    let right_root = block_pos.get_mut(right).unwrap();
                    right_root.breadth_slot += offset;
                    right_root.subtree_offset += offset;

                    if right_root.breadth_slot > subtree_width {
                        subtree_width = right_root.breadth_slot;
                    }
                    // move to next sibling
                    left = right;
                }
            }
        }

        // update subtree centering offset (now that all subtrees are moved into final position)
        match current {
            LayoutBlockType::Normal(id) => {
                let pos = block_pos.get_mut(id).expect("missing block pos info");
                pos.subtree_offset = pos.breadth_slot - 0.5 * subtree_width;
            }
            LayoutBlockType::Virtual => {
                root_subtree_offset = -0.5 * subtree_width;
            }
        }
    }

    // c) integrate offset to breadth slots in a preorder run -----------------
    let mut child_queue = root_nodes.iter().map(|info| info.id).collect::<Vec<_>>();

    // special treatment for root nodes: apply global virtual root centering offset
    for i in root_nodes {
        let pos = block_pos.get_mut(i.id).unwrap();
        pos.breadth_slot += root_subtree_offset;
        pos.subtree_offset += root_subtree_offset;
    }

    while let Some(current) = child_queue.pop() {
        let subtree_offset = block_pos
            .get_mut(current)
            .expect("missing block pos info")
            .subtree_offset;

        if let Some(links) = block_links.get(current) {
            for target in links.iter().rev() {
                let pos = block_pos.get_mut(target).expect("missing child_pos info");

                pos.breadth_slot += subtree_offset;
                pos.subtree_offset += subtree_offset;

                child_queue.push(target);
            }
        }
    }
}
// ----------------------------------------------------------------------------
