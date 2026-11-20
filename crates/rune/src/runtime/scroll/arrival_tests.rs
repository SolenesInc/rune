//! `El::reveal_on_arrival`: an element arriving in a viewport already on screen shows on the
//! frame it arrives, then the focused control does; one that mounts with its viewport, or that
//! stays mounted, moves nothing.

use super::*;
use crate::{Cx, El, Env, ScrollSpec, col, el};

/// A 200 point viewport over a field at the top, 600 points of rows and, while the state says
/// so, a refusal under them; `open` mounts the viewport itself.
#[derive(Clone, Copy, Default)]
struct Form {
    open: bool,
    refused: bool,
    words: u8,
}

fn view(form: &Form, _: &mut Cx<Form>) -> El {
    let body = col()
        .key("body")
        .size(300.0, 200.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(el().key("field").w_full().h(30.0).focusable(true))
        .children((0..20).map(|i| el().key(format!("row{i}")).w_full().h(30.0).focusable(true)))
        .children(form.refused.then(|| {
            el().key("refusal").w_full().h(20.0 + f32::from(form.words)).reveal_on_arrival()
        }));
    col().w(300.0).h(400.0).children(form.open.then_some(body))
}

fn shown(rt: &Runtime<Form>, key: &str) -> f32 {
    let e = rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("no {key}"));
    e.visible[3]
}

/// How far the viewport has scrolled: the field sits at its top.
fn offset(rt: &Runtime<Form>) -> f32 {
    -rt.elements().into_iter().find(|e| e.key.as_deref() == Some("field")).expect("the field").rect[1]
}

fn opened(refused: bool) -> Runtime<Form> {
    let mut rt = Runtime::new(Form { open: true, refused, words: 0 }, view, (300.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

#[test]
fn an_element_arriving_below_the_fold_shows_on_the_frame_it_arrives() {
    let mut rt = opened(false);
    rt.update(|f, _| f.refused = true);
    rt.frame(1.0 / 60.0);
    assert_eq!(shown(&rt, "refusal"), 20.0, "the refusal shows whole on its first frame");
    assert_eq!(offset(&rt), 450.0, "the viewport moved just far enough");
}

#[test]
fn an_element_that_mounts_with_its_viewport_leaves_it_at_its_top() {
    let rt = opened(true);
    assert_eq!(offset(&rt), 0.0);
    let mut rt = Runtime::new(Form { open: false, refused: true, words: 0 }, view, (300.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.update(|f, _| f.open = true);
    rt.frame(1.0 / 60.0);
    assert_eq!(offset(&rt), 0.0, "a viewport that opens holding the element opens at its top");
}

#[test]
fn after_it_arrives_the_scroll_is_the_persons() {
    let mut rt = opened(false);
    rt.update(|f, _| f.refused = true);
    rt.frame(1.0 / 60.0);
    rt.wheel(150.0, 100.0, 1000.0);
    for _ in 0..60 { rt.frame(1.0 / 60.0); }
    assert_eq!(offset(&rt), 0.0, "the wheel took the viewport back to its top");
    rt.update(|f, _| f.words = 10);
    for _ in 0..60 { rt.frame(1.0 / 60.0); }
    assert_eq!(offset(&rt), 0.0, "a refusal that changes but stays mounted moves nothing");
}

#[test]
fn the_focused_control_shows_after_the_element_that_arrived() {
    for (focused, to) in [("row17", 450.0), ("row10", 330.0)] {
        let mut rt = opened(false);
        rt.update(|_, tx| tx.focus(focused, true));
        rt.frame(1.0 / 60.0);
        rt.update(|f, _| f.refused = true);
        rt.frame(1.0 / 60.0);
        assert_eq!(shown(&rt, focused), 30.0, "{focused}: the focused control shows, revealed last");
        assert_eq!(offset(&rt), to, "{focused}: each moved the viewport by the least it could");
    }
}
