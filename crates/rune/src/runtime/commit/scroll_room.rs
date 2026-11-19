//! `El::scroll_room`: a viewport keeps a strip clear across from each axis its content
//! overflows on, as a browser's classic scroll bar takes its width from an `overflow: auto` box.
//! The strip is padding at the viewport's end edge or bottom. Whether it is taken is decided
//! from the placed layout, kept on the node, and the next commit's style starts from it, so a
//! viewport whose overflow holds lays out once.

use taffy::util::MaybeResolve;

use super::*;

/// The end edge's strip and the bottom's, in points, as `taken` says: the vertical overflow's
/// and the sideways one's.
pub(super) fn room_sides(room: f32, taken: [bool; 2]) -> [f32; 2] {
    taken.map(|t| if t { room } else { 0.0 })
}

/// Adds the strips `taken` names to `style`'s end and bottom padding, or with `sign` -1 takes
/// them away. A percentage padding stays as it is.
pub(super) fn pad_room(style: &mut CompactStyle, room: f32, taken: [bool; 2], sign: f32) {
    let [end, bottom] = room_sides(room, taken).map(|r| r * sign);
    let padding = style.padding();
    if end != 0.0
        && let Some(p) = padding.right.maybe_resolve(None, |_, _| 0.0)
    {
        style.set_pad_right(p + end);
    }
    if bottom != 0.0
        && let Some(p) = padding.bottom.maybe_resolve(None, |_, _| 0.0)
    {
        style.set_pad_bottom(p + bottom);
    }
}

/// The strips the last layout gave viewport `id` with `room` asked, which this commit's style
/// starts from.
pub(super) fn room_taken_before(
    memo_ctx: &memo_reuse::MemoFlattenCtx<'_>,
    id: NodeId,
    room: f32,
) -> [bool; 2] {
    if room <= 0.0 {
        return [false; 2];
    }
    memo_ctx
        .old_node_index
        .get(&id)
        .and_then(|&i| memo_ctx.old_nodes.get(i))
        .filter(|old| old.id == id)
        .map_or([false; 2], |old| old.room_taken)
}

/// How far viewport `i`'s content reaches past its box, less the strips it keeps, along x and
/// y: negative while it fits.
pub(super) fn overflow_past_room(nodes: &[Node], i: usize) -> [f32; 2] {
    let view = nodes[i].target;
    let [end, bottom] = room_sides(nodes[i].scroll_edges.room, nodes[i].room_taken);
    let (mut w, mut h) = (0.0f32, 0.0f32);
    for c in (i + 1..nodes.len())
        .take_while(|&j| nodes[j].parent >= Some(i))
        .filter(|&j| nodes[j].parent == Some(i))
    {
        let t = nodes[c].target;
        w = w.max(match nodes[i].flow {
            crate::el::Flow::Ltr => t[2] - view[0],
            crate::el::Flow::Rtl => view[2] - t[0],
        });
        h = h.max(t[3] - view[1]);
    }
    [
        w - (view[2] - view[0] - end),
        h - (view[3] - view[1] - bottom),
    ]
}

impl Measurement<'_> {
    /// Gives each placed viewport with `El::scroll_room` the strips its content's overflow asks
    /// for and takes back the ones it no longer does, through its style. True when any changed,
    /// so layout runs again.
    pub(super) fn take_scroll_room(&self, taffy: &mut LayoutEngine, nodes: &mut [Node]) -> bool {
        let mut changed = false;
        for i in 0..nodes.len() {
            let room = nodes[i].scroll_edges.room;
            if room <= 0.0 || nodes[i].scroll.is_none() {
                continue;
            }
            let axes = nodes[i].scroll_axes;
            let [x, y] = overflow_past_room(nodes, i);
            let taken = [axes.y() && y > 0.5, axes.x() && x > 0.5];
            let was = nodes[i].room_taken;
            if taken == was {
                continue;
            }
            let mut style = taffy.style(nodes[i].taffy).clone();
            pad_room(&mut style, room, was, -1.0);
            pad_room(&mut style, room, taken, 1.0);
            taffy.set_style_and_mark_ancestors(
                nodes[i].taffy,
                style,
                std::iter::successors(nodes[i].parent, |&p| nodes[p].parent)
                    .map(|p| nodes[p].taffy),
            );
            if let Some(c) = nodes[i].cold.get_mut_if_some() {
                c.room_taken = taken;
            }
            changed = true;
        }
        changed
    }
}
