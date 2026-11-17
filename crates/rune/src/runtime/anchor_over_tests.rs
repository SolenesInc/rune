//! `Side::Over`: an overlay centered over its anchor both ways, kept inside the window.
use super::*;
use crate::el::{Align, Side};
use crate::{Cx, El, Env, col, el};

fn view(target: &[f32; 4], _: &mut Cx<[f32; 4]>) -> El {
    let [x, y, w, h] = *target;
    col()
        .size(400.0, 300.0)
        .child(el().key("pane").absolute(x, y).size(w, h))
        .child(el().key("menu").size(100.0, 60.0).anchored("pane", Side::Over, Align::Start, 12.0))
}

fn menu(target: [f32; 4]) -> [f32; 4] {
    let mut rt = Runtime::new(target, view, (400.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.presented("menu").unwrap().0
}

#[test]
fn an_overlay_over_its_anchor_sits_on_the_anchors_middle_whatever_its_gap_and_alignment() {
    assert_eq!(menu([100.0, 50.0, 200.0, 200.0]), [150.0, 120.0, 100.0, 60.0]);
}

#[test]
fn an_overlay_over_an_anchor_by_the_windows_edge_stays_8_points_inside() {
    assert_eq!(menu([-40.0, 260.0, 60.0, 60.0]), [8.0, 232.0, 100.0, 60.0]);
}
