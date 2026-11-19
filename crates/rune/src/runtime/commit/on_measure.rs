//! `El::on_measure`: handlers that hear a commit's layout before it paints, and whose state
//! changes the commit lays out again.
use super::*;

impl RuntimeCore {
    /// Runs `El::on_measure` for the elements `measured` moved, and measures again while one
    /// ran, at most [`layout_read::PASSES`] times.
    pub(super) fn settle_measures(&mut self, mut measured: Measured, tx: &Tx) -> Measured {
        for _ in 0..layout_read::PASSES {
            if !self.run_measures(&measured.nodes) {
                break;
            }
            self.discard_measured(measured);
            measured = self.measure(tx);
            measured = self.settle_layout_reads(measured, tx);
        }
        measured
    }

    /// Calls each `El::on_measure` whose element `nodes` moved since it last heard; true when
    /// one ran. Elements gone from `nodes` are forgotten, so one that comes back hears again.
    fn run_measures(&mut self, nodes: &[Node]) -> bool {
        let mut last = std::mem::take(&mut self.measure_fired);
        let mut moved = Vec::new();
        for (i, n) in nodes.iter().enumerate() {
            let Some(f) = n.handlers.get().and_then(|h| h.on_measure.clone()) else {
                continue;
            };
            let rect = self.placed(nodes, i, false);
            let heard = last.remove(&n.path);
            if layout_read::moved(heard, Some(rect)) {
                moved.push((f, rect));
                self.measure_fired.insert(n.path.clone(), rect);
            } else if let Some(heard) = heard {
                self.measure_fired.insert(n.path.clone(), heard);
            }
        }
        for (f, rect) in &moved {
            f(self.app.states(), *rect);
        }
        !moved.is_empty()
    }
}
