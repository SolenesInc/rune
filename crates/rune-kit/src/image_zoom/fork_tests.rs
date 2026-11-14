//! Tests of the SolenesInc fork's preview patches (`FORK.md`).

use rune::{Cx, Env, Runtime, col, el};

use super::*;

#[derive(Default)]
struct App {
    zoom: ImageZoom,
}

fn view(app: &App, _: &mut Cx<App>) -> El {
    let drawing = el().key("drawing").size(1200.0, 300.0).child(el().key("node").size(100.0, 40.0));
    col().w_full().h_full().child(
        Preview::new(|s: &mut App| &mut s.zoom, [1200.0, 300.0], drawing)
            .scale_content()
            .build(&app.zoom, &Kit::neutral(true), [600.0, 800.0], true),
    )
}

#[test]
fn a_preview_of_scaled_content_draws_its_own_layout_fitted_to_the_window() {
    let mut rt = Runtime::new(App::default(), view, (600.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let elements = rt.elements();
    let drawing = elements.iter().find(|e| e.key.as_deref() == Some("drawing")).expect("the drawing");
    let [_, _, w, h] = drawing.rect;
    assert!(w < 600.0 && (w / h - 4.0).abs() < 0.01, "drawn fitted to the window, its shape kept: {w}x{h}");
    assert!((drawing.scale - w / 1200.0).abs() < 0.001, "drawn by scaling: {}", drawing.scale);
    let node = elements.iter().find(|e| e.key.as_deref() == Some("node")).expect("its content");
    assert!((node.rect[2] / w - 100.0 / 1200.0).abs() < 0.001, "its content keeps its own layout, scaled with it: {}", node.rect[2]);
}
