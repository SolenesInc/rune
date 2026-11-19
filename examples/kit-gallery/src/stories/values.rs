//! A tool panel's controls: a selected IconButton, Slider, NumberStepper and Swatches.

use super::*;

fn number(s: &G, key: &str, default: f64) -> f64 {
    s.picked(key, "").parse().unwrap_or(default)
}

fn set(key: &'static str) -> impl Fn(&mut G, f64, &mut Tx) + Clone {
    move |s: &mut G, v: f64, _: &mut Tx| {
        s.pick.insert(key.into(), v.to_string());
    }
}

/// A caption over a control, as a tool panel labels one: name on the left, reading on the right.
fn labelled(k: &Kit, name: &str, reading: Option<String>, control: El) -> El {
    let caption = TextStyle { line_height: 16.0, ..k.ty.small };
    let mut head = row().w_full().justify_between().child(text(name).font(caption).color(k.c.text_muted));
    if let Some(r) = reading {
        head = head.child(text(r).font(caption).color(k.c.text_base));
    }
    col().w(220.0).gap(6.0).child(head).child(control)
}

/// A tool picker: ghost icon buttons, the active tool selected.
pub(super) fn icon_button_selected(s: &G, k: &Kit, _: &mut Vec<El>) -> El {
    let tools: [(&'static rune::Icon, &str); 5] = [
        (&ic::PENCIL_LINE, "Pen"),
        (&ic::OUTLINE_TEXT, "Text"),
        (&ic::SELECT_ELEMENT, "Select"),
        (&ic::MAGNIFYING_GLASS, "Magnifier"),
        (&ic::DASH, "Line"),
    ];
    let on: usize = s.picked("tool", "0").parse().unwrap_or(0);
    let mut picker = row().gap(4.0).padding(4.0).radius(k.radius.lg).fill(k.c.layer_01);
    for (i, (icon, name)) in tools.into_iter().enumerate() {
        picker = picker.child(
            IconButton::new(icon)
                .label(name)
                .variant(IconButtonVariant::Ghost)
                .control_size(ControlSize::Large)
                .selected(i == on)
                .on_press(move |s: &mut G| {
                    s.pick.insert("tool".into(), i.to_string());
                })
                .build(k)
                .key(format!("tool-{i}")),
        );
    }
    col().gap(12.0).items_start().child(picker).child(
        flex(12.0)
            .child(IconButton::new(&ic::PLUS).label("Add").selected(true).build(k))
            .child(
                IconButton::new(&ic::PLUS).label("Add").variant(IconButtonVariant::GhostMuted).selected(true).build(k),
            )
            .child(
                IconButton::new(&ic::PLUS)
                    .label("Add")
                    .variant(IconButtonVariant::Ghost)
                    .selected(true)
                    .disabled(true)
                    .build(k),
            ),
    )
}

pub(super) fn slider_basic(s: &G, k: &Kit, _: &mut Vec<El>) -> El {
    let size = number(s, "size", 12.0);
    let opacity = number(s, "opacity", 0.8);
    col()
        .gap(16.0)
        .child(labelled(
            k,
            "Size",
            Some(format!("{size:.0} px")),
            Slider::new("Size", size, 1.0..=64.0)
                .step(1.0)
                .reading(format!("{size:.0} px"))
                .on_change(|s: &mut G, ch: SliderChange, tx: &mut Tx| set("size")(s, ch.value, tx))
                .build(k)
                .key("size"),
        ))
        .child(labelled(
            k,
            "Opacity",
            Some(format!("{:.0}%", opacity * 100.0)),
            Slider::new("Opacity", opacity, 0.0..=1.0)
                .step(0.05)
                .on_change(|s: &mut G, ch: SliderChange, tx: &mut Tx| set("opacity")(s, ch.value, tx))
                .build(k)
                .key("opacity"),
        ))
        .child(labelled(
            k,
            "Disabled",
            None,
            Slider::new("Disabled", 0.3, 0.0..=1.0).disabled(true).build(k).key("disabled"),
        ))
}

pub(super) fn number_stepper_basic(s: &G, k: &Kit, _: &mut Vec<El>) -> El {
    let width = number(s, "width", 4.0);
    let blur = number(s, "blur", 0.0);
    let zoom = number(s, "zoom", 1.5);
    col()
        .gap(16.0)
        .child(labelled(
            k,
            "Width",
            None,
            NumberStepper::new("Width", width)
                .range(1.0, 40.0)
                .reading(format!("{width:.0} px"))
                .on_change(set("width"))
                .build(k)
                .key("width"),
        ))
        .child(labelled(
            k,
            "Blur, at its minimum",
            None,
            NumberStepper::new("Blur", blur).range(0.0, 20.0).on_change(set("blur")).build(k).key("blur"),
        ))
        .child(labelled(
            k,
            "Zoom",
            None,
            NumberStepper::new("Zoom", zoom)
                .range(1.0, 4.0)
                .step(0.25)
                .width(220.0)
                .on_change(set("zoom"))
                .build(k)
                .key("zoom"),
        ))
        .child(labelled(
            k,
            "Disabled",
            None,
            NumberStepper::new("Disabled", 2.0).disabled(true).build(k).key("disabled"),
        ))
}

pub(super) fn swatches_basic(s: &G, k: &Kit, _: &mut Vec<El>) -> El {
    let hex = |h: &str| rune::theme::hex(h).unwrap_or_default();
    let colours = [
        (hex("#ff3b30"), "Red"),
        (hex("#ff9500"), "Orange"),
        (hex("#ffcc00"), "Yellow"),
        (hex("#34c759"), "Green"),
        (hex("#007aff"), "Blue"),
        (hex("#000000"), "Black"),
        (hex("#ffffff"), "White"),
    ];
    let picked = |key: &str| s.picked(key, "0").parse().ok();
    let pick = |key: &'static str| {
        move |s: &mut G, i: usize, _: &mut Tx| {
            s.pick.insert(key.into(), i.to_string());
        }
    };
    col()
        .gap(16.0)
        .child(labelled(
            k,
            "Colour",
            None,
            Swatches::new("Colour", &colours)
                .selected(picked("colour"))
                .width(220.0)
                .on_pick(pick("colour"))
                .build(k)
                .key("colour"),
        ))
        // Room for six of seven: rows of four and three, not six and one.
        .child(labelled(
            k,
            "Border colour",
            None,
            Swatches::new("Border colour", &colours)
                .selected(picked("border"))
                .width(172.0)
                .on_pick(pick("border"))
                .build(k)
                .key("border"),
        ))
}
