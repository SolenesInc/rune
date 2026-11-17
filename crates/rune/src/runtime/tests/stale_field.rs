use super::*;
use crate::PointerButton;
use crate::el::{col, el};

#[derive(Default)]
struct S { field: Option<crate::TextEdit>, menu: bool }

fn view(s: &S, _: &mut Cx<S>) -> El {
    let close = el().key("close").size(100.0, 40.0).on_press(|s: &mut S| s.field = None);
    let below = match &s.field {
        Some(field) => crate::text_input(field, crate::TextStyle::new(14.0, 400.0)).key("field").size(100.0, 40.0)
            .on_edit(|s: &mut S| s.field.as_mut().expect("a field's state lives as long as the field")),
        None => el().key("blank").size(100.0, 40.0),
    };
    let menu = el().key("menu").size(100.0, 40.0).focusable(true).on_press(|s: &mut S| s.menu = true);
    col().size(200.0, 200.0).child(close).child(below).child(menu)
}

fn rt() -> Runtime<S> {
    let mut rt = Runtime::new(S { field: Some(crate::TextEdit::new("hello")), menu: false }, view, (200.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn click(rt: &mut Runtime<S>, (x, y): (f32, f32), button: PointerButton) {
    rt.pointer_move(x, y);
    rt.button_down(x, y, button);
    rt.button_up(x, y, button);
}

const CLOSE: (f32, f32) = (20.0, 20.0);
const FIELD: (f32, f32) = (20.0, 60.0);
const MENU: (f32, f32) = (20.0, 100.0);

#[test]
fn a_press_on_a_field_an_earlier_press_in_the_frame_closed_reaches_no_field() {
    let mut rt = rt();
    click(&mut rt, CLOSE, PointerButton::Primary);
    click(&mut rt, FIELD, PointerButton::Primary);
    rt.frame(1.0);
    assert!(rt.state().field.is_none());
    assert!(rt.nodes.iter().all(|n| n.key.as_deref() != Some("field")));
}

#[test]
fn a_field_menu_asked_after_an_earlier_press_closed_the_field_opens_on_no_field() {
    let mut rt = rt();
    click(&mut rt, CLOSE, PointerButton::Primary);
    click(&mut rt, FIELD, PointerButton::Secondary);
    rt.frame(1.0);
    assert!(rt.state().field.is_none());
}

#[test]
fn a_focus_change_after_an_earlier_press_closed_a_field_does_not_reach_its_state() {
    let mut rt = rt();
    click(&mut rt, CLOSE, PointerButton::Primary);
    click(&mut rt, MENU, PointerButton::Primary);
    rt.frame(1.0);
    assert!(rt.state().field.is_none());
    assert!(rt.state().menu, "the later press still lands");
    assert!(rt.focus.path().is_some_and(|p| rt.node(p).is_some_and(|n| n.key.as_deref() == Some("menu"))));
}
