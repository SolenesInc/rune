//! A wrapping row as wide as its items keeps them on one line at every zoom and display scale,
//! though its text measures in thirds, fifths or ninths of a point there.
use super::*;
use crate::{Cx, El, Env, TextStyle, row, text};

#[derive(Clone)]
struct S {
    zoom: f32,
    labels: [&'static str; 2],
}

/// A notice's shape: a sentence that grows from a basis too wide to leave its actions room, so
/// the actions' row takes a line of its own at its own max-content width.
fn view(s: &S, cx: &mut Cx<S>) -> El {
    cx.zoom(s.zoom);
    let actions = row().key("actions").flex_wrap().items_center().gap(8.0).max_w_frac(1.0).children(
        s.labels.iter().enumerate().map(|(i, l)| row().padding(6.0).child(text(*l).key(format!("label{i}")).font(TextStyle::new(14.0, 500.0)))),
    );
    let said = text("Something went wrong and this sentence says what").wrap().grow_from(1.0, 260.0);
    let line = row().w_full().flex_wrap().items_start().gap(8.0).child(said).child(actions);
    col().size(800.0, 600.0).child(col().w(300.0).child(line))
}

#[test]
fn a_wrapping_row_as_wide_as_its_items_keeps_them_on_one_line_at_every_zoom() {
    let mut dropped = Vec::new();
    for labels in [["Undo", "Cancel"], ["Keep both", "Cancel"], ["Show in folder", "Reload from disk"]] {
        for scale in [1.0, 2.0] {
            for tenths in 7..=15 {
                let zoom = tenths as f32 / 10.0;
                let mut rt = Runtime::new(S { zoom, labels }, view, (800.0, 600.0), Env::new(true, scale));
                rt.frame(0.0);
                let first = rt.presented("label0").unwrap().0;
                let last = rt.presented("label1").unwrap().0;
                if last[1] != first[1] {
                    dropped.push((labels, scale, zoom));
                }
            }
        }
    }
    assert!(dropped.is_empty(), "rows that put their last item on a line of its own (labels, scale, zoom): {dropped:?}");
}
