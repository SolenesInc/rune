//! `El::reveal_on_arrival`: an element that arrives in a scroll viewport already on screen
//! shows on the frame it arrives, and then the focused control does.
use super::*;

impl RuntimeCore {
    /// Notes node `i` of the commit's `nodes`, which mounts into a parent already on screen,
    /// when a vertical viewport holds it outside any virtual list.
    pub(super) fn note_arrival(&mut self, nodes: &[Node], i: usize) {
        let mut at = nodes[i].parent;
        while let Some(p) = at {
            if self.virtuals.contains_key(&nodes[p].path) {
                return;
            }
            if nodes[p].scroll.is_some() && nodes[p].scroll_axes != crate::el::ScrollAxes::X {
                self.requests.note_arrived(nodes[i].path.clone());
                return;
            }
            at = nodes[p].parent;
        }
    }

    /// The reveals the elements that arrived are owed: each in turn, then the focused element.
    pub(super) fn arrival_reveals(&mut self, into: &mut Vec<ScrollReveal>) {
        let arrived = self.requests.take_arrived();
        if arrived.is_empty() {
            return;
        }
        into.extend(arrived.into_iter().map(|path| ScrollReveal::into_view(None, path)));
        if let Some(focused) = self.focused_index() {
            into.push(self.reveal_focus(focused));
        }
    }
}

#[cfg(test)]
#[path = "arrival_tests.rs"]
mod tests;
