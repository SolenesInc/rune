//! `Tx::scroll_into_view` and `Tx::scroll_center`: revealing or centring a keyed element,
//! virtual row or `TextHighlight` inside its scroll viewport.
use super::*;

/// Where a reveal lands its target in the viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Landing {
    /// Just into view, moving as little as possible: `Tx::scroll_into_view`.
    Nearest,
    /// Just into view, and a target taller than the viewport from its start:
    /// `Tx::scroll_row_into_view_in`.
    Start,
    /// Centred vertically: `Tx::scroll_center`.
    Center,
}

/// One `Tx::scroll_into_view`, `Tx::scroll_row_into_view_in` or `Tx::scroll_center` request.
#[derive(Clone, Debug)]
pub(crate) struct ScrollReveal {
    pub scope: Option<String>,
    pub key: String,
    pub landing: Landing,
    /// Glide there on the scroll spring rather than jump.
    pub animated: bool,
    /// The least space to leave between the target and the viewport's edge, such as the reach of
    /// its focus ring; the viewport's own scroll margin counts toward it.
    pub reach: f32,
    /// The viewport to scroll, when it is an outer one: the target then lands where the
    /// viewports between them have scrolled it.
    pub through: Option<String>,
}

impl ScrollReveal {
    pub fn into_view(scope: Option<String>, key: String) -> Self {
        ScrollReveal { scope, key, landing: Landing::Nearest, animated: false, reach: 0.0, through: None }
    }

    pub fn row_into_view(scope: Option<String>, key: String) -> Self {
        ScrollReveal { scope, key, landing: Landing::Start, animated: false, reach: 0.0, through: None }
    }

    pub fn center(scope: Option<String>, key: String, animated: bool) -> Self {
        ScrollReveal { scope, key, landing: Landing::Center, animated, reach: 0.0, through: None }
    }
}

/// The offset that lands `[top, top + height)` in the viewport as `landing` says, from
/// `offset`, before clamping.
pub(super) fn place(landing: Landing, offset: f64, top: f64, height: f64, viewport: f64, margin: f64) -> f64 {
    match landing {
        Landing::Nearest => scroll_edge::reveal(offset, top, height, viewport, margin),
        Landing::Start => scroll_edge::reveal_start(offset, top, height, viewport, margin),
        Landing::Center => (top - ((viewport - height) * 0.5).max(0.0)).max(0.0),
    }
}

impl RuntimeCore {
    /// Scrolls so row or highlight `r.key` shows, landing as `r.landing` says. A virtual row
    /// placed by estimates may measure differently once mounted, along with the rows around it,
    /// so this places it again from what the commit measured until it stays put. Those commits
    /// are `instant` when the request's was, or when it glides: the glide then starts back
    /// where the viewport stood and runs to that place, keeping the speed of one already running.
    pub(super) fn reveal_row(&mut self, r: &ScrollReveal, instant: bool) {
        let (scope, key) = (r.scope.as_deref(), r.key.as_str());
        let glide = r.animated && !instant && !self.reduced_motion && !self.settled;
        let through = r.through.as_deref();
        let (mut window, was) = self.reveal_row_once(scope, key, through, r.landing, r.reach, true, instant || glide);
        for _ in 0..8 {
            if !window { break; }
            window = self.reveal_row_once(scope, key, through, r.landing, r.reach, false, instant || glide).0;
        }
        if glide && let Some(Was { id, offset: start, glide: v, viewport }) = was
            && let Some(st) = self.scrolls.get_mut(&id)
            && (st.offset - start).abs() > 0.5
        {
            // A long trip cuts to a couple of viewports out first, as `Tx::scroll_to` does.
            let end = st.offset;
            let reach = scroll_bottom::GLIDE_REACH * viewport;
            st.offset = start.clamp(end - reach, end + reach);
            st.mode = st.mode.to_glide(scroll_offset::retarget_v(motion::GLIDE, st.offset, v.unwrap_or(0.0), end), GlideTo::Offset(end));
            st.spring = None;
            st.moved();
            if self.virtual_window_changed() {
                self.commit(&Tx { commit: Commit::Instant, ..Tx::default() });
            }
        }
        self.reveal_row_x(scope, key);
    }

    /// One placement of [`Self::reveal_row`]: whether it committed, and on the first, where the
    /// viewport stood before. After the first, it moves only when the row would land somewhere
    /// else.
    #[allow(clippy::too_many_arguments)]
    fn reveal_row_once(&mut self, scope: Option<&str>, key: &str, through: Option<&str>, landing: Landing, reach: f32, first: bool, instant: bool) -> (bool, Option<Was>) {
        let target_node = self.nodes.iter().enumerate().find_map(|(i, row)| {
            if !row.named(key) || self.subtree_exiting(i) || !row.parent.is_none_or(|p| in_scope(&self.nodes[p].path, scope)) {
                return None;
            }
            Some((i, row.target[1], row.local.h, true))
        }).or_else(|| {
            let (i, [_, y0, _, y1]) = self.highlight_target_in(&self.nodes, scope, key)?;
            (!self.subtree_exiting(i)).then_some((i, y0, (y1 - y0).max(0.0), false))
        });
        let geometry = target_node.and_then(|(i, target_top, target_h, is_whole_node)| {
            let s = match through {
                Some(path) => self.nodes.iter().position(|n| n.path == path)?,
                None => self.viewport_of(i)?,
            };
            let parent = &self.nodes[s];
            let scrolled_between = std::iter::successors(self.viewport_of(i), |&v| self.viewport_of(v))
                .take_while(|&v| v != s)
                .filter_map(|v| self.scrolls.get(&self.nodes[v].id))
                .map(|st| st.offset as f32)
                .sum::<f32>();
            let (laid_top, laid_h) = (target_top - parent.target[1] - scrolled_between, target_h);
            let (top, height) = std::iter::successors(Some(i), |&p| self.nodes[p].parent)
                .take_while(|&p| p != s)
                .find_map(|r| {
                    let c = self.nodes[r].parent?;
                    self.nodes[c].growth.as_ref()?;
                    let stack = self.stacks.get(&self.nodes[c].id)?;
                    let rid = self.nodes[r].id;
                    let mut count = 0usize;
                    let before_r: f32 = stack.order.iter()
                        .take_while(|&&id| id != rid)
                        .filter_map(|id| self.grows.get(id))
                        .map(|g| { count += 1; g.h })
                        .sum::<f32>() + stack.gap * count as f32;
                    let r_top = (self.nodes[c].target[1] - parent.target[1]) + stack.pad_top + before_r;
                    let r_h = self.grows.get(&rid).map_or(self.nodes[r].local.h, |g| g.h);
                    if i == r && is_whole_node {
                        Some((r_top, r_h))
                    } else {
                        let in_row = target_top - self.nodes[r].target[1];
                        Some((r_top + in_row, target_h))
                    }
                })
                .map(|(top, height)| (top - scrolled_between, height))
                .unwrap_or((laid_top, laid_h));
            let laid_end = self.scroll_children(s).map(|c| self.nodes[c].target[3] - parent.target[1]).reduce(f32::max).unwrap_or(0.0);
            Some((parent.path.clone(), f64::from(top), f64::from(height), f64::from(parent.local.h), f64::from(laid_top), f64::from(laid_h), f64::from(laid_end)))
        }).or_else(|| {
            self.nodes.iter().filter(|n| in_scope(&n.path, scope)).filter_map(|n| {
                let v = self.virtuals.get(&n.path)?;
                let index = v.index_of(key).or_else(|| {
                    key.rmatch_indices('/').find_map(|(at, _)| {
                        let (scope, row_key) = (&key[..at], &key[at + 1..]);
                        let mut keyed = n.path.rsplit('/').filter(|seg| !seg.starts_with('#'));
                        scope.rsplit('/').all(|seg| keyed.next() == Some(seg)).then(|| v.index_of(row_key))?
                    })
                }).or_else(|| {
                    // `"row/mark"` names an element or highlight inside unmounted row `row`.
                    key.match_indices('/').find_map(|(at, _)| v.index_of(&key[..at]))
                })?;
                let row_key = &v.keys[index];
                let top = f64::from(v.top + v.pad_top + v.prefix(index));
                let height = f64::from(v.height(row_key));
                let end = f64::from(v.top + v.pad_top + v.prefix(v.keys.len()) + v.pad_bottom);
                Some((v.scroll_path.clone(), top, height, f64::from(v.viewport), top, height, end))
            }).next()
        });
        let Some((path, top, height, viewport, laid_top, laid_h, laid_end)) = geometry else { return (false, None) };
        // Stuck headers cover the viewport's top: land the row below them.
        let Some(cover) = self.stuck_cover(&path, key) else { return (false, None) };
        let (top, height) = (top - cover, height + cover);
        let (laid_top, laid_h) = (laid_top - cover, laid_h + cover);
        let margin = self.node(&path).map_or(0.0, |n| f64::from(n.scroll_edges.margin.max(reach)));
        let Some(&id) = self.path_ids.get(&path) else { return (false, None) };
        let Some(&s_idx) = self.node_index.get(&id) else { return (false, None) };
        let stick = self.nodes[s_idx].scroll.is_some_and(|s| s.stick_bottom);
        let max = self.scroll_overflow(s_idx).max(0.0);
        let Some(scroll) = self.scrolls.get_mut(&id) else { return (false, None) };
        let was = first.then_some(Was { id, offset: scroll.offset, glide: scroll.glide(), viewport });
        scroll.max = max;
        if first { scroll.halt(); }
        let offset = place(landing, scroll.offset, top, height, viewport, margin).clamp(0.0, scroll.max);
        let laid_max = (laid_end - viewport).max(0.0);
        let laid_to = place(landing, scroll.offset, laid_top, laid_h, viewport, margin).clamp(0.0, laid_max);
        let at_end = stick && (scroll.pinned() || offset >= scroll.max - 0.5) && laid_to >= laid_max - 0.5;
        let effects = self.requests.step(requests::Event::Place { first, scope, key, viewport: &path,
            landing, at_end, from: scroll.offset, to: offset });
        let Some(effect) = effects.into_iter().next() else { return (false, was) };
        match effect {
            requests::Effect::Move(to) => {
                scroll.offset = to;
                scroll.mode = scroll.mode.to_rest();
                scroll.moved();
            }
            requests::Effect::Rest => {
                scroll.mode = scroll.mode.to_rest();
                return (false, was);
            }
        }
        let window = self.virtual_window_changed();
        if window {
            self.commit(&Tx { commit: if instant { Commit::Instant } else { Commit::Animate }, ..Tx::default() });
            // The commit measured rows that were estimates: the next placement clamps to the end
            // they make.
            if let Some(i) = self.nodes.iter().position(|n| n.id == id) {
                let max = self.scroll_overflow(i).max(0.0);
                if let Some(scroll) = self.scrolls.get_mut(&id) {
                    scroll.max = max;
                    scroll.mode = scroll.mode.to_rest();
                }
            }
        }
        (window, was)
    }
}

/// Where a reveal's viewport stood before it moved, for the glide that follows.
struct Was {
    id: NodeId,
    offset: f64,
    glide: Option<f32>,
    viewport: f64,
}
