//! `El::reveal_on_arrival`: an element that arrives in a scroll viewport already on screen
//! shows on the frame it arrives, and the focused control in that viewport after it.
use super::*;

impl RuntimeCore {
    /// Notes node `i` of the commit's `nodes`, new to the scene, when its nearest vertical
    /// viewport outside any virtual list was a vertical viewport on the previous frame too.
    pub(super) fn note_arrival(&mut self, nodes: &[Node], i: usize) {
        let vertical = |n: &Node| n.scroll.is_some() && n.scroll_axes != crate::el::ScrollAxes::X;
        let mut at = nodes[i].parent;
        while let Some(p) = at {
            if self.virtuals.contains_key(&nodes[p].path) {
                return;
            }
            if vertical(&nodes[p]) {
                let id = nodes[p].id;
                if self.nodes.iter().any(|was| was.id == id && vertical(was)) {
                    self.requests.note_arrived(nodes[i].path.clone());
                }
                return;
            }
            at = nodes[p].parent;
        }
    }

    /// One reveal for each element that arrived, each moving only its own viewport, then the
    /// focused control's when it sits in a viewport an element arrived in.
    pub(super) fn arrival_reveals(&mut self, into: &mut Vec<ScrollReveal>) {
        let arrived = self.requests.take_arrived();
        let viewports: Vec<usize> = arrived.iter().filter_map(|path| self.viewport_of(self.index_of_path(path)?)).collect();
        into.extend(arrived.into_iter().map(|path| ScrollReveal::into_view(None, path)));
        if let Some(focused) = self.focused_index()
            && self.viewport_of(focused).is_some_and(|v| viewports.contains(&v))
        {
            into.push(ScrollReveal::into_view(None, self.nodes[focused].path.clone()));
        }
    }

    fn index_of_path(&self, path: &str) -> Option<usize> {
        self.node_index.get(self.path_ids.get(path)?).copied()
    }
}

#[cfg(test)]
#[path = "arrival_tests.rs"]
mod tests;
