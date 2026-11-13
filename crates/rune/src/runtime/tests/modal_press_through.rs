use super::*;
use crate::ScrollSpec;

const DT: f64 = 1.0 / 120.0;

struct Popover { open: bool, through: bool, pressed: u32 }

/// A focusable control at the top left and, while open, a popover modal at the bottom right
/// that closes on a press outside it.
fn view(s: &Popover, _: &mut Cx<Popover>) -> El {
    let control = el().key("control").absolute(0.0, 0.0).size(40.0, 40.0).focusable(true)
        .on_press(|s: &mut Popover| s.pressed += 1);
    let popover = s.open.then(|| {
        el().key("popover").absolute(100.0, 100.0).size(40.0, 40.0).focusable(true)
            .modal(true, |s: &mut Popover| s.open = false)
            .modal_press_through(s.through)
    });
    el().size(200.0, 200.0).child(control).children(popover)
}

fn press_outside(through: bool) -> Runtime<Popover> {
    let mut rt = Runtime::new(Popover { open: true, through, pressed: 0 }, view, (200.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.pointer_move(20.0, 20.0);
    rt.pointer_down(20.0, 20.0);
    rt.pointer_up(20.0, 20.0);
    rt.frame(DT);
    rt
}

#[test]
fn a_press_outside_a_press_through_modal_closes_it_and_presses_what_is_under_it() {
    let rt = press_outside(true);
    assert!(!rt.state.open, "the press asked the modal to close");
    assert_eq!(rt.state.pressed, 1, "the press went on to the control under it");
    assert_eq!(rt.focused_key().as_deref(), Some("control"), "focus stays where the press put it");
}

#[test]
fn a_press_outside_a_modal_only_closes_it_by_default() {
    let rt = press_outside(false);
    assert!(!rt.state.open);
    assert_eq!(rt.state.pressed, 0, "the press only closed the modal");
}

struct Invoked { open: bool, toggled: u32 }

/// A toggle at the top left whose press-through popover, while open, is anchored under it.
fn invoked(s: &Invoked, _: &mut Cx<Invoked>) -> El {
    let toggle = el().key("toggle").absolute(0.0, 0.0).size(40.0, 40.0)
        .on_press(|s: &mut Invoked| { s.open = !s.open; s.toggled += 1; });
    let popover = s.open.then(|| {
        el().key("popover").size(40.0, 40.0)
            .modal(true, |s: &mut Invoked| s.open = false)
            .modal_press_through(true)
            .anchored("toggle", crate::Side::Bottom, crate::Align::Start, 4.0)
    });
    el().size(200.0, 200.0).child(toggle).children(popover)
}

#[test]
fn a_press_on_a_press_through_modals_invoker_leaves_the_modal_to_the_invoker() {
    let mut rt = Runtime::new(Invoked { open: false, toggled: 0 }, invoked, (200.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let press = |rt: &mut Runtime<Invoked>, t: f64| {
        rt.pointer_move(20.0, 20.0);
        rt.pointer_down(20.0, 20.0);
        rt.pointer_up(20.0, 20.0);
        rt.frame(t);
    };
    press(&mut rt, DT);
    assert!(rt.state.open, "the first press opens the popover");
    press(&mut rt, 2.0 * DT);
    assert!(!rt.state.open, "the second press closes it, and nothing opens it again");
    assert_eq!(rt.state.toggled, 2, "each press reached the invoker once");
}

struct Reading { through: bool }

/// A tall page in a scroll viewport, with a modal open over its top left.
fn reading(s: &Reading, _: &mut Cx<Reading>) -> El {
    let page = col().key("page").size(200.0, 200.0).scroll(ScrollSpec { stick_bottom: false })
        .child(el().size(200.0, 1000.0));
    let modal = el().key("modal").absolute(0.0, 0.0).size(40.0, 40.0).focusable(true)
        .modal(true, |_: &mut Reading| {})
        .modal_press_through(s.through);
    el().size(200.0, 200.0).child(page).child(modal)
}

fn wheel_beside_the_modal(through: bool) -> Runtime<Reading> {
    let mut rt = Runtime::new(Reading { through }, reading, (200.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.pointer_move(150.0, 150.0);
    rt.wheel(150.0, 150.0, -60.0);
    rt.frame(DT);
    rt
}

#[test]
fn the_wheel_beside_a_press_through_modal_scrolls_what_is_under_it() {
    let rt = wheel_beside_the_modal(true);
    assert_eq!(rt.scrolls.values().next().map(|s| s.offset), Some(60.0));
}

#[test]
fn the_wheel_beside_a_modal_scrolls_nothing_by_default() {
    let rt = wheel_beside_the_modal(false);
    assert_eq!(rt.scrolls.values().next().map(|s| s.offset), Some(0.0));
}
