//! `Tx::focus` that moves focus shows what it focused on that frame, as Tab does and as a web
//! page's `element.focus()` scrolls it into view; a request for the node already focused leaves
//! the scroll where the person put it.
use crate::{Cx, El, Env, Runtime, ScrollSpec, col, el};

#[derive(Default)]
struct S;

fn button(key: &str) -> El { el().key(key).size(200.0, 30.0).focusable(true).role("button") }

fn rows(prefix: &str, n: usize) -> impl Iterator<Item = El> + '_ {
    (0..n).map(move |i| button(&format!("{prefix}{i}")))
}

fn one_viewport(_: &S, _: &mut Cx<S>) -> El {
    col().key("root").size(200.0, 200.0)
        .child(col().key("viewport").size(200.0, 100.0).scroll(ScrollSpec { stick_bottom: false }).children(rows("row", 10)))
}

fn nested(_: &S, _: &mut Cx<S>) -> El {
    col().key("root").size(200.0, 200.0)
        .child(col().key("outer").size(200.0, 100.0).scroll(ScrollSpec { stick_bottom: false })
            .children(rows("lead", 5))
            .child(col().key("inner").size(200.0, 60.0).scroll(ScrollSpec { stick_bottom: false }).children(rows("deep", 8))))
}

fn start(view: fn(&S, &mut Cx<S>) -> El) -> Runtime<S> {
    let mut rt = Runtime::new(S, view, (200.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn rect(rt: &Runtime<S>, key: &str) -> [f32; 4] {
    rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("no {key}")).rect
}

fn whole_in(rt: &Runtime<S>, key: &str, viewport: &str) -> bool {
    let [_, y, _, h] = rect(rt, key);
    let [_, vy, _, vh] = rect(rt, viewport);
    y >= vy - 0.5 && y + h <= vy + vh + 0.5
}

fn focused(rt: &Runtime<S>) -> Option<String> {
    rt.elements().into_iter().find(|e| e.focused).and_then(|e| e.key)
}

fn focus(rt: &mut Runtime<S>, key: &'static str) {
    rt.update(|_, tx| tx.focus(key, true));
    rt.frame(0.0);
}

#[test]
fn a_focus_that_moves_shows_the_control_on_the_same_frame() {
    let mut rt = start(one_viewport);
    assert!(!whole_in(&rt, "row9", "viewport"));
    focus(&mut rt, "row9");
    assert_eq!(focused(&rt).as_deref(), Some("row9"));
    assert!(whole_in(&rt, "row9", "viewport"), "row9 at {:?}, viewport {:?}", rect(&rt, "row9"), rect(&rt, "viewport"));
}

#[test]
fn a_focus_on_the_control_already_focused_leaves_the_scroll_where_it_was_put() {
    let mut rt = start(one_viewport);
    focus(&mut rt, "row9");
    rt.update(|_, tx| { tx.instant(); tx.scroll_to_y("viewport", 0.0) });
    rt.frame(0.0);
    let away = rect(&rt, "row9");
    assert!(!whole_in(&rt, "row9", "viewport"), "the scroll took row9 out of view");
    focus(&mut rt, "row9");
    assert_eq!(rect(&rt, "row9"), away, "asking for the focus it already has scrolled");
}

#[test]
fn a_focus_on_a_control_already_in_view_scrolls_nothing() {
    let mut rt = start(one_viewport);
    let before = rect(&rt, "row1");
    focus(&mut rt, "row1");
    assert_eq!(focused(&rt).as_deref(), Some("row1"));
    assert_eq!(rect(&rt, "row1"), before);
}

#[test]
fn a_focus_inside_a_nested_viewport_shows_through_each() {
    let mut rt = start(nested);
    assert!(!whole_in(&rt, "deep7", "outer"));
    focus(&mut rt, "deep7");
    assert!(whole_in(&rt, "deep7", "inner"), "deep7 at {:?}, inner {:?}", rect(&rt, "deep7"), rect(&rt, "inner"));
    assert!(whole_in(&rt, "deep7", "outer"), "deep7 at {:?}, outer {:?}", rect(&rt, "deep7"), rect(&rt, "outer"));
}

fn ringed(_: &S, _: &mut Cx<S>) -> El {
    let color = crate::color::oklch(0.7, 0.15, 60.0);
    col().key("root").size(200.0, 200.0)
        .child(col().key("viewport").size(200.0, 100.0).scroll(ScrollSpec { stick_bottom: false })
            .children(rows("row", 5))
            .child(col().key("box").padding(8.0).focus_within_ring(2.0, 2.0, color).child(button("field")))
            .child(button("after")))
}

#[test]
fn a_focus_shows_the_whole_ring_the_control_s_box_draws_around_it() {
    let mut rt = start(ringed);
    focus(&mut rt, "field");
    let [_, y, _, h] = rect(&rt, "box");
    let [_, vy, _, vh] = rect(&rt, "viewport");
    assert!(y - 4.0 >= vy - 0.5 && y + h + 4.0 <= vy + vh + 0.5, "the box at {:?} with its ring is cut by the viewport {:?}", rect(&rt, "box"), rect(&rt, "viewport"));
}

#[test]
fn tab_shows_the_whole_ring_the_control_s_box_draws_around_it() {
    let mut rt = start(ringed);
    for _ in 0..6 {
        rt.key(crate::KeyEvent { key: crate::Key::Tab, mods: crate::Mods::default() });
        rt.frame(0.0);
    }
    assert_eq!(focused(&rt).as_deref(), Some("field"));
    let [_, y, _, h] = rect(&rt, "box");
    let [_, vy, _, vh] = rect(&rt, "viewport");
    assert!(y - 4.0 >= vy - 0.5 && y + h + 4.0 <= vy + vh + 0.5, "the box at {:?} with its ring is cut by the viewport {:?}", rect(&rt, "box"), rect(&rt, "viewport"));
}

fn ringed_around_the_viewport(_: &S, _: &mut Cx<S>) -> El {
    col().key("composite").w(320.0).h(200.0)
        .focus_within_ring(2.0, 0.0, crate::color::oklch(1.0, 0.0, 0.0))
        .child(col().key("body").w_full().h_full().scroll(ScrollSpec { stick_bottom: false })
            .children((0..8).map(|i| {
                el().key(format!("row{i}")).label(format!("Row {i}")).role("button").focusable(true).w_full().h(40.0)
            })))
}

#[test]
fn tab_shows_each_row_of_a_viewport_whose_ring_owner_sits_outside_it() {
    let mut rt = Runtime::new(S, ringed_around_the_viewport, (320.0, 200.0), Env::new(true, 1.0));
    rt.set_full_keyboard_access(true);
    rt.frame(0.0);
    for i in 0..8 {
        rt.key(crate::KeyEvent { key: crate::Key::Tab, mods: crate::Mods::default() });
        rt.frame(1.0 / 60.0);
        let row = format!("row{i}");
        assert_eq!(focused(&rt).as_deref(), Some(row.as_str()));
        assert!(whole_in(&rt, &row, "body"), "{row} at {:?}, body {:?}", rect(&rt, &row), rect(&rt, "body"));
    }
}

fn taller_inner(_: &S, _: &mut Cx<S>) -> El {
    let scroll = ScrollSpec { stick_bottom: false };
    col().key("root").size(200.0, 200.0)
        .child(col().key("outer").size(200.0, 100.0).scroll(scroll)
            .child(el().w_full().h(150.0))
            .child(col().key("inner").size(200.0, 160.0).scroll(scroll).children(rows("deep", 12)))
            .child(el().w_full().h(100.0)))
}

#[test]
fn a_focus_inside_a_viewport_taller_than_its_own_shows_through_both() {
    let mut rt = start(taller_inner);
    focus(&mut rt, "deep11");
    assert!(whole_in(&rt, "deep11", "inner") && whole_in(&rt, "deep11", "outer"), "deep11 at {:?}, inner {:?}, outer {:?}", rect(&rt, "deep11"), rect(&rt, "inner"), rect(&rt, "outer"));
    focus(&mut rt, "deep0");
    assert!(whole_in(&rt, "deep0", "inner") && whole_in(&rt, "deep0", "outer"), "deep0 at {:?}, inner {:?}, outer {:?}", rect(&rt, "deep0"), rect(&rt, "inner"), rect(&rt, "outer"));
}

#[test]
fn tab_inside_a_viewport_taller_than_its_own_shows_through_both() {
    let mut rt = start(taller_inner);
    focus(&mut rt, "deep11");
    rt.key(crate::KeyEvent { key: crate::Key::Tab, mods: crate::Mods::default() });
    rt.frame(0.0);
    assert_eq!(focused(&rt).as_deref(), Some("deep0"));
    assert!(whole_in(&rt, "deep0", "inner") && whole_in(&rt, "deep0", "outer"), "deep0 at {:?}, inner {:?}, outer {:?}", rect(&rt, "deep0"), rect(&rt, "inner"), rect(&rt, "outer"));
}

fn three_deep(_: &S, _: &mut Cx<S>) -> El {
    let scroll = ScrollSpec { stick_bottom: false };
    col().key("root").size(200.0, 200.0)
        .child(col().key("outer").size(200.0, 100.0).scroll(scroll)
            .child(el().w_full().h(120.0))
            .child(col().key("middle").size(200.0, 140.0).scroll(scroll)
                .child(el().w_full().h(90.0))
                .child(col().key("inner").size(200.0, 120.0).scroll(scroll).children(rows("deep", 10)))
                .child(el().w_full().h(90.0)))
            .child(el().w_full().h(120.0)))
}

#[test]
fn a_focus_three_viewports_deep_shows_through_each() {
    let mut rt = start(three_deep);
    for key in ["deep9", "deep0", "deep5"] {
        focus(&mut rt, key);
        for viewport in ["inner", "middle", "outer"] {
            assert!(whole_in(&rt, key, viewport), "{key} at {:?} is cut by {viewport} at {:?}", rect(&rt, key), rect(&rt, viewport));
        }
    }
}

fn sideways_in_vertical(_: &S, _: &mut Cx<S>) -> El {
    col().key("root").size(200.0, 200.0)
        .child(col().key("outer").size(200.0, 100.0).scroll(ScrollSpec { stick_bottom: false })
            .children(rows("lead", 5))
            .child(crate::row().key("strip").size(200.0, 30.0).scroll_x()
                .children((0..6).map(|i| el().key(format!("chip{i}")).size(80.0, 30.0).focusable(true).role("button")))))
}

fn left(rt: &Runtime<S>, key: &str) -> f32 { rect(rt, key)[0] }

#[test]
fn a_focus_in_a_strip_inside_a_vertical_viewport_moves_sideways_only_when_cut_sideways() {
    let mut rt = start(sideways_in_vertical);
    focus(&mut rt, "chip1");
    assert!(whole_in(&rt, "chip1", "outer"), "chip1 at {:?}, outer {:?}", rect(&rt, "chip1"), rect(&rt, "outer"));
    assert_eq!(left(&rt, "chip0"), left(&rt, "strip"), "the strip moved sideways for a chip already whole on X");
    focus(&mut rt, "chip5");
    let [x, _, w, _] = rect(&rt, "chip5");
    let [sx, _, sw, _] = rect(&rt, "strip");
    assert!(x >= sx - 0.5 && x + w <= sx + sw + 0.5, "chip5 at {:?} is cut by the strip {:?}", rect(&rt, "chip5"), rect(&rt, "strip"));
    assert!(whole_in(&rt, "chip5", "outer"));
}
