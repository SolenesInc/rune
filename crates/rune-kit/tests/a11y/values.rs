//! Values and toggles a tool panel sets: a selected icon button, a slider, a number stepper and
//! colour swatches, by assistive action, pointer and keyboard.

use rune::accesskit::Action;
use rune::{Color, Cx, El, Key, col};
use rune_kit::*;

use crate::support::*;

const fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color { r, g, b, a: 1.0 }
}

const COLOURS: [(Color, &str); 7] = [
    (rgb(0.9, 0.2, 0.2), "Red"),
    (rgb(0.95, 0.6, 0.1), "Orange"),
    (rgb(0.95, 0.85, 0.2), "Yellow"),
    (rgb(0.2, 0.7, 0.3), "Green"),
    (rgb(0.2, 0.5, 0.9), "Blue"),
    (rgb(0.0, 0.0, 0.0), "Black"),
    (rgb(1.0, 1.0, 1.0), "White"),
];

fn panel(s: &App, _: &mut Cx<App>) -> El {
    let k = kit();
    let level = f64::from(s.level);
    col()
        .gap(12.0)
        .items_start()
        .child(
            IconButton::new(k.glyph.copy)
                .label("Pen")
                .variant(IconButtonVariant::Ghost)
                .selected(s.on)
                .on_press(|s: &mut App| s.on = !s.on)
                .build(&k)
                .key("pen"),
        )
        .child(
            IconButton::new(k.glyph.close)
                .label("Eraser")
                .variant(IconButtonVariant::Ghost)
                .selected(false)
                .build(&k)
                .key("eraser"),
        )
        .child(
            Slider::new("Size", level, 0.0..=1.0)
                .step(0.1)
                .reading(format!("{:.0}%", level * 100.0))
                .width(200.0)
                .on_change(|s: &mut App, ch: SliderChange, _: &mut rune::Tx| {
                    s.level = ch.value as f32;
                    s.continues.push(ch.continues);
                })
                .build(&k)
                .key("size"),
        )
        .child(
            NumberStepper::new("Width", s.count)
                .range(1.0, 5.0)
                .on_change(|s: &mut App, v, _: &mut rune::Tx| s.count = v)
                .build(&k)
                .key("width"),
        )
        .child(
            Swatches::new("Colour", &COLOURS)
                .selected(s.picked)
                .width(6.0 * 30.0 - 8.0)
                .on_pick(|s: &mut App, i, _: &mut rune::Tx| s.picked = Some(i))
                .build(&k)
                .key("colour"),
        )
        .child(Slider::new("Opacity", 0.5, 0.0..=1.0).disabled(true).build(&k).key("opacity"))
}

#[test]
fn a_selected_icon_button_reads_as_a_pressed_toggle() {
    let mut rt = mount(App::default(), panel);
    let tree = rt.accessibility_json();
    let pen = node(&tree, "pen");
    assert_eq!(said(pen), ("button", "Pen"));
    assert!(has(pen, "unchecked") && !has(pen, "collapsed"), "a toggle, not a menu trigger: {pen}");
    assert!(has(node(&tree, "eraser"), "unchecked"));
    golden("values", &tree);
    assert!(act(&mut rt, "pen", Action::Click));
    assert!(has(node(&rt.accessibility_json(), "pen"), "checked"), "selected reads as pressed");
}

#[test]
fn a_slider_steps_by_action_and_key_and_a_drag_is_one_change_then_continuations() {
    let mut rt = mount(App::default(), panel);
    let tree = rt.accessibility_json();
    let size = node(&tree, "size");
    assert_eq!(said(size), ("slider", "Size"));
    assert_eq!(size["value"], "40%");
    assert!(can(size, "increment") && can(size, "decrement"));
    assert!(find(size, "knob").is_none(), "the knob and track are drawing, not controls");

    assert!(act(&mut rt, "size", Action::Increment));
    assert!((rt.state().level - 0.5).abs() < 1e-6, "{}", rt.state().level);
    assert!(act(&mut rt, "size", Action::Focus));
    press_key(&mut rt, Key::Right);
    assert!((rt.state().level - 0.6).abs() < 1e-6);
    press_key(&mut rt, Key::End);
    assert_eq!(rt.state().level, 1.0);
    press_key(&mut rt, Key::Home);
    assert_eq!(rt.state().level, 0.0);
    assert_eq!(rt.state().continues, [false; 4], "a step by action or key starts its own change");

    // A drag from the start to the end: its first change starts, the rest continue.
    rt.update(|s, _| s.continues.clear());
    let b: Vec<f32> = node(&rt.accessibility_json(), "size")["bounds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();
    let y = b[1] + b[3] / 2.0;
    rt.pointer_move(b[0] + 8.0, y);
    rt.pointer_down(b[0] + 8.0, y);
    settle(&mut rt);
    for x in [0.3, 0.55, 0.8, 1.0] {
        rt.pointer_move(b[0] + b[2] * x, y);
        settle(&mut rt);
    }
    rt.pointer_up(b[0] + b[2], y);
    settle(&mut rt);
    assert_eq!(rt.state().level, 1.0);
    let c = &rt.state().continues;
    assert!(c.len() >= 2 && !c[0] && c[1..].iter().all(|&x| x), "one change, then continuations: {c:?}");

    let off = node(&rt.accessibility_json(), "opacity").clone();
    assert!(has(&off, "disabled") && !can(&off, "increment"), "{off}");
}

#[test]
fn a_number_stepper_is_one_spin_button_with_named_step_buttons() {
    let mut rt = mount(App::default(), panel);
    let tree = rt.accessibility_json();
    let width = node(&tree, "width");
    assert_eq!(said(width), ("spin-button", "Width"));
    assert_eq!(width["value"], "3");
    assert_eq!(said(node(&tree, "down")), ("button", "Decrease Width"));
    assert_eq!(said(node(&tree, "up")), ("button", "Increase Width"));
    no_echo(width);

    tap(&mut rt, "up");
    assert_eq!(rt.state().count, 4.0);
    assert!(act(&mut rt, "down", Action::Click));
    assert_eq!(rt.state().count, 3.0);
    assert!(act(&mut rt, "width", Action::Focus));
    press_key(&mut rt, Key::Up);
    press_key(&mut rt, Key::Up);
    assert_eq!(rt.state().count, 5.0);
    let up = node(&rt.accessibility_json(), "up").clone();
    assert!(has(&up, "disabled"), "at the maximum Increase is off: {up}");
    assert_eq!(focused(&rt.accessibility_json()).as_deref(), Some("width"), "the buttons take no focus");
}

#[test]
fn swatches_are_a_radio_group_that_wraps_evenly_and_moves_by_arrow() {
    let mut rt = mount(App { picked: Some(0), ..App::default() }, panel);
    let tree = rt.accessibility_json();
    let colour = node(&tree, "colour");
    assert_eq!(said(colour), ("radio-group", "Colour"));
    let radios = with_role(colour, "radio-button");
    assert_eq!(radios.iter().map(|r| r["name"].as_str().unwrap()).collect::<Vec<_>>(), COLOURS.map(|(_, n)| n));
    assert!(has(radios[0], "checked") && has(radios[1], "unchecked"));
    let top = |key: &str| node(&tree, key)["bounds"][1].as_f64().unwrap();
    assert_eq!(top("swatch3"), top("swatch0"), "seven in room for six: four on the first row");
    assert!(top("swatch4") > top("swatch3") && top("swatch6") == top("swatch4"), "and three on the second");

    tap(&mut rt, "swatch4");
    assert_eq!(rt.state().picked, Some(4));
    press_key(&mut rt, Key::Right);
    assert_eq!(focused(&rt.accessibility_json()).as_deref(), Some("swatch5"), "arrows move between swatches");
    press_key(&mut rt, Key::Text(" ".into()));
    assert_eq!(rt.state().picked, Some(5));
    assert!(act(&mut rt, "swatch6", Action::Click));
    assert_eq!(rt.state().picked, Some(6));
}
