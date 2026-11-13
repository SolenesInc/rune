//! A focusable control that keeps focus on a press, by `El::preserve_focus_on_press` or
//! `Cx::controls_keep_focus`: Tab reaches it, and a press on it leaves focus where it was, as a
//! macOS window's push button does.
use rune::tok::TextStyle;
use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, TextEdit, col, row, text};

#[derive(Default)]
struct S {
    field: TextEdit,
    presses: u32,
    window_wide: bool,
}

fn view(s: &S, cx: &mut Cx<S>) -> El {
    if s.window_wide {
        cx.controls_keep_focus();
    }
    col()
        .size(300.0, 100.0)
        .child(
            rune::text_input(&s.field, TextStyle::new(14.0, 400.0))
                .key("field")
                .w(150.0)
                .on_edit(|s: &mut S| &mut s.field),
        )
        .child(
            row()
                .key("button")
                .role("button")
                .label("Go")
                .size(60.0, 24.0)
                .focusable(true)
                .preserve_focus_on_press(!s.window_wide)
                .on_press(|s: &mut S| s.presses += 1),
        )
        .child(text("Plain words").key("plain"))
}

fn focused(rt: &Runtime<S>) -> Option<String> {
    rt.elements().iter().find(|e| e.focused).and_then(|e| e.key.clone())
}

fn runtime(field_focused: bool) -> Runtime<S> {
    runtime_with(field_focused, false)
}

fn runtime_with(field_focused: bool, window_wide: bool) -> Runtime<S> {
    let mut s = S { window_wide, ..S::default() };
    s.field.focused = field_focused;
    let mut rt = Runtime::new(s, view, (300.0, 100.0), Env::default());
    rt.frame(0.0);
    rt
}

fn click(rt: &mut Runtime<S>, key: &str) {
    let (x, y) = rt.find(key).expect("laid out");
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
}

#[test]
fn a_press_on_a_focusable_control_that_keeps_focus_leaves_the_field_focused() {
    let mut rt = runtime(true);
    assert_eq!(focused(&rt).as_deref(), Some("field"));

    click(&mut rt, "button");

    assert_eq!(rt.state.presses, 1);
    assert_eq!(focused(&rt).as_deref(), Some("field"));
}

#[test]
fn a_press_on_a_focusable_control_that_keeps_focus_leaves_nothing_focused_when_nothing_was() {
    let mut rt = runtime(false);

    click(&mut rt, "button");

    assert_eq!(rt.state.presses, 1);
    assert_eq!(focused(&rt), None);
}

#[test]
fn tab_still_reaches_a_focusable_control_that_keeps_focus_on_a_press() {
    let mut rt = runtime(true);

    rt.key(KeyEvent { key: Key::Tab, mods: Mods::default() });
    rt.frame(1.0 / 60.0);

    assert_eq!(focused(&rt).as_deref(), Some("button"));
}

#[test]
fn under_controls_keep_focus_a_press_on_a_button_leaves_the_field_focused_and_tab_still_reaches_it() {
    let mut rt = runtime_with(true, true);

    click(&mut rt, "button");
    assert_eq!(rt.state.presses, 1);
    assert_eq!(focused(&rt).as_deref(), Some("field"));

    rt.key(KeyEvent { key: Key::Tab, mods: Mods::default() });
    rt.frame(1.0 / 60.0);
    assert_eq!(focused(&rt).as_deref(), Some("button"));
}

#[test]
fn under_controls_keep_focus_a_press_on_plain_content_still_blurs_the_field() {
    let mut rt = runtime_with(true, true);

    click(&mut rt, "plain");

    assert_eq!(focused(&rt), None);
}
