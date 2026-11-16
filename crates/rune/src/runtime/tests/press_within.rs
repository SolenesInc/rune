use super::*;
use crate::PointerButton;
use crate::el::{col, el, row, text};

#[derive(Default)]
struct S { heard: Vec<&'static str> }

fn view(_: &S, _: &mut Cx<S>) -> El {
    col().w_full().h_full().on_press(|s: &mut S| s.heard.push("window"))
        .child(col().key("pane").size(100.0, 100.0).on_press_within(|s: &mut S, _| s.heard.push("pane"))
            .child(el().key("button").size(40.0, 40.0).on_press(|s: &mut S| s.heard.push("button"))
                .pointer_buttons([PointerButton::Middle]).on_secondary_press(|s: &mut S, _, _| s.heard.push("menu")))
            .child(text("plain words")))
        .child(el().key("over").size(30.0, 30.0).pin(Some(60.0), Some(60.0), None, None).on_press(|s: &mut S| s.heard.push("over")))
}

fn rt() -> Runtime<S> {
    let mut rt = Runtime::new(S::default(), view, (200.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn press(rt: &mut Runtime<S>, x: f32, y: f32, button: PointerButton) -> Vec<&'static str> {
    rt.pointer_move(x, y);
    rt.button_down(x, y, button);
    rt.button_up(x, y, button);
    rt.frame(1.0);
    std::mem::take(&mut rt.state_mut().heard)
}

#[test]
fn a_press_on_a_control_inside_tells_the_element_first_and_still_presses_the_control() {
    let mut rt = rt();
    assert_eq!(press(&mut rt, 20.0, 20.0, PointerButton::Primary), ["pane", "button"]);
}

#[test]
fn a_press_on_the_elements_own_surface_stops_on_it() {
    let mut rt = rt();
    assert_eq!(press(&mut rt, 80.0, 20.0, PointerButton::Primary), ["pane"], "the window under the pane doesn't hear it");
    assert_eq!(press(&mut rt, 10.0, 45.0, PointerButton::Primary), ["pane"], "nor a press on plain text inside");
}

#[test]
fn every_button_tells_the_element() {
    let mut rt = rt();
    assert_eq!(press(&mut rt, 20.0, 20.0, PointerButton::Secondary), ["pane", "menu"]);
    assert_eq!(press(&mut rt, 20.0, 20.0, PointerButton::Middle), ["pane", "button"]);
}

#[test]
fn a_press_outside_or_on_something_drawn_over_it_does_not_tell_it() {
    let mut rt = rt();
    assert_eq!(press(&mut rt, 150.0, 150.0, PointerButton::Primary), ["window"]);
    assert_eq!(press(&mut rt, 75.0, 75.0, PointerButton::Primary), ["over"]);
}

#[test]
fn a_primary_press_on_a_surface_that_keeps_its_press_does_not_tell_the_element_but_a_control_on_it_does() {
    #[derive(Default)]
    struct H { heard: Vec<&'static str> }
    fn view(_: &H, _: &mut Cx<H>) -> El {
        col().key("pane").size(100.0, 100.0).on_press_within(|s: &mut H, _| s.heard.push("pane"))
            .child(row().key("header").w_full().h(40.0).keeps_press_from_within()
                .on_drag(|s: &mut H, d: &mut crate::Drag, _| if d.phase == crate::Phase::Down { s.heard.push("drag") })
                .on_secondary_press(|s: &mut H, _, _| s.heard.push("menu"))
                .child(el().key("close").size(20.0, 20.0).on_press(|s: &mut H| s.heard.push("close"))))
    }
    let mut rt = Runtime::new(H::default(), view, (200.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let mut press = |x: f32, y: f32, button: PointerButton| {
        rt.pointer_move(x, y);
        rt.button_down(x, y, button);
        rt.button_up(x, y, button);
        rt.frame(1.0);
        std::mem::take(&mut rt.state_mut().heard)
    };
    assert_eq!(press(60.0, 20.0, PointerButton::Primary), ["drag"]);
    assert_eq!(press(10.0, 10.0, PointerButton::Primary), ["pane", "close"]);
    assert_eq!(press(60.0, 20.0, PointerButton::Secondary), ["pane", "menu"]);
}
