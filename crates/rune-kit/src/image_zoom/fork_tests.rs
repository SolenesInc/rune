//! Tests of the SolenesInc fork's preview patches (`FORK.md`).

use rune::{Cx, Env, Runtime, col, el};

use super::*;

#[derive(Default)]
struct App {
    zoom: ImageZoom,
    closed: bool,
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

fn with_actual_size(app: &App, _: &mut Cx<App>) -> El {
    let drawing = el().key("drawing").size(1200.0, 300.0);
    col().w_full().h_full().child(
        Preview::new(|s: &mut App| &mut s.zoom, [1200.0, 300.0], drawing)
            .scale_content()
            .actual_size("Actual size")
            .escape_returns()
            .on_close(|s: &mut App, _| s.closed = true)
            .build(&app.zoom, &Kit::neutral(true), [600.0, 800.0], true),
    )
}

fn drawing(rt: &Runtime<App>) -> [f32; 4] {
    rt.elements().iter().find(|e| e.key.as_deref() == Some("drawing")).expect("the drawing").rect
}

#[test]
fn a_preview_opened_at_actual_size_shows_its_start_at_natural_size() {
    let mut app = App::default();
    app.zoom.open_actual([1200.0, 300.0], [600.0, 800.0]);
    let mut rt = Runtime::new(app, with_actual_size, (600.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let [x, _, w, _] = drawing(&rt);
    let body = rt.elements().iter().find(|e| e.key.as_deref() == Some("image-preview-body")).expect("the body").rect;
    assert!((w - 1200.0).abs() < 0.5, "at its natural width: {w}");
    assert!((x - body[0]).abs() < 0.5, "its left edge at the body's: {x} against {}", body[0]);
    assert_eq!(rt.state().zoom.level(), 100);
}

#[test]
fn the_actual_size_button_shows_a_fitted_preview_at_natural_size() {
    let mut rt = Runtime::new(App::default(), with_actual_size, (600.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    assert!(drawing(&rt)[2] < 600.0, "it opens fitted");
    let [bx, by, bw, bh] = rt.elements().iter().find(|e| e.key.as_deref() == Some("image-preview-zoom-actual")).expect("the 100% button").rect;
    let (x, y) = (bx + bw / 2.0, by + bh / 2.0);
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.frame(1.0 / 60.0);
    rt.pointer_up(x, y);
    rt.frame(2.0 / 60.0);
    assert!((drawing(&rt)[2] - 1200.0).abs() < 0.5, "at its natural width: {}", drawing(&rt)[2]);
}

#[test]
fn a_preview_has_no_actual_size_button_unless_asked() {
    let mut rt = Runtime::new(App::default(), view, (600.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    assert!(rt.elements().iter().all(|e| e.key.as_deref() != Some("image-preview-zoom-actual")));
}

fn press(rt: &mut Runtime<App>, key: rune::Key) {
    rt.key(rune::KeyEvent { key, mods: rune::Mods::default() });
    rt.frame(1.0 / 60.0);
}

#[test]
fn the_1_key_shows_the_preview_at_natural_size() {
    let mut rt = Runtime::new(App::default(), with_actual_size, (600.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    press(&mut rt, rune::Key::Text("1".into()));
    assert!((drawing(&rt)[2] - 1200.0).abs() < 0.5, "at its natural width: {}", drawing(&rt)[2]);
}

#[test]
fn a_preview_people_return_from_closes_on_the_first_escape_however_zoomed() {
    let mut app = App::default();
    app.zoom.open_actual([1200.0, 300.0], [600.0, 800.0]);
    let mut rt = Runtime::new(app, with_actual_size, (600.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    press(&mut rt, rune::Key::Escape);
    assert!(rt.state().closed, "one Escape closes it at 100%");
}
