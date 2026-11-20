//! `El::reveal_on_arrival`. A node that carries it is revealed when
//!
//! (a) its nearest vertical scroll viewport outside a virtual list was a vertical scroll
//!     viewport on the previous frame, and
//! (b) the node itself was not in the scene on the previous frame;
//!
//! then (c) that viewport, and only that one, scrolls by the least it can to show the node, at
//! once, and after it, when the focused control sits in that same viewport, by the least it can
//! to show the focused control. No other viewport moves, and which node a focus move reveals
//! does not change.

use super::*;
use crate::{Color, Cx, El, Env, Key, KeyEvent, Mods, ScrollSpec, col, el, row};

/// A 200 point viewport over a field at the top, 600 points of rows and, while the state says
/// so, a refusal under them; `open` mounts the viewport itself and `scrolls` makes it one.
#[derive(Clone, Copy)]
struct Form {
    open: bool,
    scrolls: bool,
    refused: bool,
    words: u8,
}

impl Default for Form {
    fn default() -> Self {
        Form { open: true, scrolls: true, refused: false, words: 0 }
    }
}

fn view(form: &Form, _: &mut Cx<Form>) -> El {
    let body = col().key("body").size(300.0, 200.0);
    let body = if form.scrolls { body.scroll(ScrollSpec { stick_bottom: false }) } else { body };
    let body = body
        .child(el().key("field").w_full().h(30.0).focusable(true))
        .children((0..20).map(|i| el().key(format!("row{i}")).w_full().h(30.0).focusable(true)))
        .children(form.refused.then(|| {
            el().key("refusal").w_full().h(20.0 + f32::from(form.words)).reveal_on_arrival()
        }));
    col().w(300.0).h(400.0).children(form.open.then_some(body))
}

fn find<S: 'static>(rt: &Runtime<S>, key: &str) -> crate::ElementInfo {
    rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("no {key}"))
}

/// How far the viewport has scrolled: the field sits at its top.
fn offset(rt: &Runtime<Form>) -> f32 {
    -find(rt, "field").rect[1]
}

fn started(form: Form) -> Runtime<Form> {
    let mut rt = Runtime::new(form, view, (300.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn refuse(rt: &mut Runtime<Form>) {
    rt.update(|f, _| f.refused = true);
    rt.frame(1.0 / 60.0);
}

#[test]
fn a_an_element_arriving_below_the_fold_shows_on_the_frame_it_arrives() {
    let mut rt = started(Form::default());
    refuse(&mut rt);
    assert_eq!(find(&rt, "refusal").visible[3], 20.0, "the refusal shows whole on its first frame");
    assert_eq!(offset(&rt), 450.0, "the viewport moved just far enough");
}

#[test]
fn a_an_element_that_mounts_with_its_viewport_leaves_it_at_its_top() {
    let rt = started(Form { refused: true, ..Form::default() });
    assert_eq!(offset(&rt), 0.0);
    let mut rt = started(Form { open: false, refused: true, ..Form::default() });
    rt.update(|f, _| f.open = true);
    rt.frame(1.0 / 60.0);
    assert_eq!(offset(&rt), 0.0, "a viewport that opens holding the element opens at its top");
}

#[test]
fn a_an_element_arriving_as_its_parent_becomes_a_viewport_leaves_it_at_its_top() {
    let mut rt = started(Form { scrolls: false, ..Form::default() });
    rt.update(|f, _| {
        f.scrolls = true;
        f.refused = true;
    });
    rt.frame(1.0 / 60.0);
    assert_eq!(find(&rt, "field").scrolled[1], 0.0, "a body that only now scrolls starts at its top");
}

#[test]
fn b_after_it_arrives_the_scroll_is_the_persons_until_it_arrives_again() {
    let mut rt = started(Form::default());
    for _ in 0..2 {
        refuse(&mut rt);
        assert_eq!(offset(&rt), 450.0, "it shows each time it arrives");
        rt.wheel(150.0, 100.0, 1000.0);
        rt.settle_all();
        rt.frame(0.0);
        assert_eq!(offset(&rt), 0.0, "the wheel took the viewport back to its top");
        rt.update(|f, _| f.words = 10);
        rt.frame(1.0 / 60.0);
        assert_eq!(offset(&rt), 0.0, "a refusal that changes but stays mounted moves nothing");
        rt.settle_all();
        rt.frame(0.0);
        assert_eq!([rt.frame(1.0 / 60.0), rt.frame(1.0 / 60.0)], [false; 2], "nothing repaints at rest");
        rt.update(|f, _| {
            f.refused = false;
            f.words = 0;
        });
        rt.settle_all();
        rt.frame(0.0);
    }
}

#[test]
fn c_the_focused_control_in_the_same_viewport_shows_after_the_arrival() {
    for (focused, to) in [("row17", 450.0), ("row10", 330.0)] {
        let mut rt = started(Form::default());
        rt.update(|_, tx| tx.focus(focused, true));
        rt.frame(1.0 / 60.0);
        refuse(&mut rt);
        assert_eq!(find(&rt, focused).visible[3], 30.0, "{focused}: the focused control shows, revealed last");
        assert_eq!(offset(&rt), to, "{focused}: each moved the viewport by the least it could");
    }
}

#[test]
fn c_an_arrival_in_one_viewport_leaves_another_where_the_wheel_put_it() {
    fn view(refused: &bool, _: &mut Cx<bool>) -> El {
        let left = col().key("left").size(300.0, 200.0).scroll(ScrollSpec { stick_bottom: false })
            .children((0..20).map(|i| el().key(format!("left{i}")).w_full().h(30.0).focusable(true)));
        let right = col().key("right").size(300.0, 200.0).scroll(ScrollSpec { stick_bottom: false })
            .children((0..20).map(|i| el().key(format!("right{i}")).w_full().h(30.0)))
            .children(refused.then(|| el().key("refusal").w_full().h(20.0).reveal_on_arrival()));
        row().size(600.0, 200.0).child(left).child(right)
    }
    let mut rt = Runtime::new(false, view, (600.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.update(|_, tx| tx.focus("left0", true));
    rt.frame(0.0);
    rt.wheel(150.0, 100.0, -1000.0);
    rt.settle_all();
    rt.frame(0.0);
    let wheeled = find(&rt, "left0").scrolled[1];
    assert!(wheeled > 0.0);
    rt.update(|refused, _| *refused = true);
    rt.frame(0.0);
    assert_eq!(find(&rt, "refusal").visible[3], 20.0, "the refusal shows in its own viewport");
    assert_eq!(find(&rt, "left0").scrolled[1], wheeled, "the viewport holding the focus stays put");
}

#[test]
fn c_tab_into_a_viewport_inside_a_ringed_container_shows_each_row() {
    fn view(_: &(), _: &mut Cx<()>) -> El {
        col().key("composite").w(320.0).h(200.0).focus_within_ring(2.0, 0.0, Color::WHITE).child(
            col().key("body").w_full().h_full().scroll(ScrollSpec { stick_bottom: false }).children((0..8).map(|i| {
                el().key(format!("row{i}")).label(format!("Row {i}")).role("button").focusable(true).w_full().h(40.0)
            })),
        )
    }
    let mut rt = Runtime::new((), view, (320.0, 200.0), Env::new(true, 1.0));
    rt.set_full_keyboard_access(true);
    rt.frame(0.0);
    for i in 0..8 {
        rt.key(KeyEvent { key: Key::Tab, mods: Mods::default() });
        rt.frame(1.0 / 60.0);
        let row = find(&rt, &format!("row{i}"));
        assert!(row.focused, "Tab reached row{i}");
        assert_eq!(row.visible[3], row.rect[3], "row{i} shows whole");
    }
}
