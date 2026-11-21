//! A checked or mixed checkbox fills with the kit's accent and draws its mark in the ink a checked
//! Switch's thumb uses on it, in light and dark; an unchecked one stays clear, and a disabled one
//! keeps the disabled surface.
use rune::{Color, Cx, El, Env, Runtime, col};
use rune_kit::{Checkbox, Kit, State, Switch};

const ACCENT: Color = Color { r: 0.1, g: 0.6, b: 0.55, a: 1.0 };
const INK: Color = Color { r: 1.0, g: 0.9, b: 0.1, a: 1.0 };

fn kit(dark: bool) -> Kit {
    let mut k = Kit::neutral(dark);
    k.c.bg_accent = ACCENT;
    k.c.knob_on = INK;
    k
}

fn view(dark: &bool, _: &mut Cx<bool>) -> El {
    {
        let k = kit(*dark);
        col()
            .child(Checkbox::new(true).label("on").build(&k).key("on"))
            .child(Checkbox::new(false).indeterminate(true).label("mixed").build(&k).key("mixed"))
            .child(Checkbox::new(false).label("off").build(&k).key("off"))
            .child(Checkbox::new(true).label("focus").state(State::Focus).build(&k).key("focus"))
            .child(Checkbox::new(true).label("hover").state(State::Hover).build(&k).key("hover"))
            .child(Switch::new(true).state(State::Hover).build(&k).key("switch-hover"))
            .child(Checkbox::new(true).label("disabled").disabled(true).build(&k).key("disabled"))
    }
}

fn paint(dark: bool, key: &str) -> (Color, Option<Color>) {
    let mut rt = Runtime::new(dark, view, (300.0, 300.0), Env::new(dark, 2.0));
    rt.frame(0.0);
    let find = |suffix: String| rt.elements().into_iter().find(|e| e.path.ends_with(&suffix));
    (find(format!("{key}/control")).unwrap().fill, find(format!("{key}/control/mark")).and_then(|m| m.text_color))
}

#[test]
fn a_checked_or_mixed_box_fills_with_the_accent_and_marks_in_the_switch_ink() {
    for dark in [false, true] {
        for key in ["on", "mixed", "focus"] {
            let (fill, mark) = paint(dark, key);
            assert_eq!(fill, ACCENT, "{key} fill, dark={dark}");
            assert_eq!(mark, Some(INK), "{key} mark, dark={dark}");
        }
    }
}

#[test]
fn a_hovered_checked_box_keeps_the_accent_as_a_hovered_switch_does() {
    for dark in [false, true] {
        let k = kit(dark);
        let (fill, mark) = paint(dark, "hover");
        assert_eq!(fill, paint(dark, "switch-hover").0, "the hovered Switch's track, dark={dark}");
        assert_ne!(fill, ACCENT);
        assert_ne!(fill, k.c.surface_hover);
        assert_eq!(mark, Some(INK));
    }
}

/// The pointer over a checked box at rest lands on the same fill the hover state draws.
#[test]
fn the_pointer_over_a_checked_box_keeps_its_accent() {
    for dark in [false, true] {
        let mut rt = Runtime::new(dark, view, (300.0, 300.0), Env::new(dark, 2.0));
        for _ in 0..10 {
            rt.frame(1.0 / 60.0);
        }
        let r = rt.elements().into_iter().find(|e| e.path.ends_with("on/control")).unwrap().rect;
        rt.pointer_move(r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
        for _ in 0..90 {
            rt.frame(1.0 / 60.0);
        }
        // The boxes' instances, top to bottom: on, mixed, off, focus, hover, disabled.
        let boxes: Vec<_> = rt.instances.iter().filter(|i| i.rect[2] == 16.0 * 2.0).map(|i| i.fill).collect();
        assert_eq!(boxes[0], boxes[4], "dark={dark}");
    }
}

#[test]
fn an_unchecked_box_stays_clear_and_a_disabled_one_keeps_its_surface() {
    for dark in [false, true] {
        let k = kit(dark);
        assert_eq!(paint(dark, "off").0, Color::TRANSPARENT);
        let (fill, mark) = paint(dark, "disabled");
        assert_eq!(fill, k.c.surface_disabled);
        assert_eq!(mark, Some(k.c.std_icon));
    }
}
