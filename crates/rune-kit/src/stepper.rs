//! NumberStepper: a number between a step down and a step up. One tab stop: the stepper is a
//! spin button, and its arrows, Page Up and Page Down step it; the − and + buttons are for the
//! pointer and for assistive technology.

use rune::{El, TextStyle, Tx, row, text};

use crate::Kit;

/// The width kept for the value, so steppers in a column line up.
const VALUE_W: f32 = 48.0;
/// A regular control's height, and each button's side.
const SIDE: f32 = 24.0;

/// Gives the parts their handlers, typed on the app's state: (root, down, up, place).
type Attach = Box<dyn FnOnce(El, El, El, Place) -> (El, El, El)>;

#[derive(Clone, Copy)]
struct Place {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
}

impl Place {
    fn moved(self, steps: i32) -> f64 {
        (self.value + f64::from(steps) * self.step).clamp(self.min, self.max)
    }
}

/// `NumberStepper::new("Width", 4.0).range(1.0, 40.0).on_change(..).build(&kit)`. The parts
/// are keyed `down`, `value` and `up`, as attn's own stepper keys them, and read to a screen
/// reader as "Decrease Width" and "Increase Width".
pub struct NumberStepper {
    label: String,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    reading: Option<String>,
    width: Option<f32>,
    disabled: bool,
    attach: Option<Attach>,
}

impl NumberStepper {
    /// `label` names the value for a screen reader; draw a visible one beside it yourself.
    pub fn new(label: impl Into<String>, value: f64) -> NumberStepper {
        NumberStepper {
            label: label.into(),
            value,
            min: f64::MIN,
            max: f64::MAX,
            step: 1.0,
            reading: None,
            width: None,
            disabled: false,
            attach: None,
        }
    }
    /// The smallest and largest value. Unset: unbounded.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        (self.min, self.max) = (min, max.max(min));
        self
    }
    /// How far one press or arrow moves the value (default 1).
    pub fn step(mut self, step: f64) -> Self {
        if step > 0.0 {
            self.step = step;
        }
        self
    }
    /// The value as shown and read, as "4 px". Unset: the number, with as many decimals as the
    /// step has.
    pub fn reading(mut self, r: impl Into<String>) -> Self {
        self.reading = Some(r.into());
        self
    }
    /// Width in points. Unset: as wide as its buttons and value need.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    /// Called with the new value; a step that would not move it (at an end) calls nothing.
    pub fn on_change<S: 'static>(mut self, f: impl Fn(&mut S, f64, &mut Tx) + 'static) -> Self {
        self.attach = Some(Box::new(move |root: El, down: El, up: El, p: Place| {
            let f = std::rc::Rc::new(f);
            let go = move |s: &mut S, next: f64, tx: &mut Tx| {
                if next != p.value {
                    f(s, next, tx);
                }
            };
            let (d, u, l) = (go.clone(), go.clone(), go.clone());
            (
                root.on_step(move |s: &mut S, delta, tx: &mut Tx| go(s, p.moved(delta), tx))
                    .on_step_limit(move |s: &mut S, end, tx: &mut Tx| l(s, if end { p.max } else { p.min }, tx)),
                down.on_press_tx(move |s: &mut S, tx: &mut Tx| d(s, p.moved(-1), tx)),
                up.on_press_tx(move |s: &mut S, tx: &mut Tx| u(s, p.moved(1), tx)),
            )
        }));
        self
    }

    pub fn build(self, k: &Kit) -> El {
        let c = &k.c;
        let value = self.value.clamp(self.min, self.max);
        let shown = self.reading.unwrap_or_else(|| format_step(value, self.step));
        let enabled = !self.disabled;
        let font = TextStyle { line_height: 16.0, ..k.ty.ui };
        let button = |key: &'static str, said: &str, mark: &'static str, can: bool| {
            let mut b = row()
                .key(key)
                .role("button")
                .label(format!("{said} {}", self.label))
                .focusable(false)
                .size(SIDE, SIDE)
                .center()
                .radius(k.radius.md)
                .disabled(!can)
                .child(text(mark).font(font).color(if can { c.icon_base } else { c.icon_faint }).a11y_hidden());
            if can {
                b = k.interactive(b);
            }
            b
        };
        let down = button("down", "Decrease", "\u{2212}", enabled && value > self.min);
        let up = button("up", "Increase", "+", enabled && value < self.max);
        let mut middle = text(shown.clone())
            .key("value")
            .font(font)
            .color(c.text_base)
            .truncate()
            .a11y_hidden()
            .grow(1.0)
            .shrink()
            .align(rune::Align::Center);
        middle = match self.width {
            Some(_) => middle,
            None => middle.w(VALUE_W),
        };
        let mut root = row()
            .role("spinbutton")
            .label(self.label.clone())
            .a11y_value(shown)
            .h(SIDE)
            .center_y()
            .radius(k.radius.md)
            .fill(c.layer_01)
            .box_shadow(rune::Shadow { spread: 0.5, color: c.border_base, ..Default::default() })
            .focus_ring(2.0, 1.0, c.border_focus)
            .disabled(!enabled);
        if self.min > f64::MIN && self.max < f64::MAX {
            root = root.range_value(value, self.min, self.max).range_step(self.step);
        }
        if let Some(w) = self.width {
            root = root.w(w);
        }
        let place = Place { value, min: self.min, max: self.max, step: self.step };
        let (root, down, up) = match (self.attach, enabled) {
            (Some(attach), true) => attach(root, down, up, place),
            _ => (root, down, up),
        };
        let root = root.child(down).child(middle).child(up);
        if enabled { root } else { root.layer_opacity(0.5) }
    }
}

/// `v` with as many decimals as `step` has (up to four): 4, 0.5, 0.25.
fn format_step(v: f64, step: f64) -> String {
    let decimals = (0..=4usize).find(|d| {
        let scaled = step * 10f64.powi(*d as i32);
        (scaled - scaled.round()).abs() < 1e-9
    });
    format!("{v:.*}", decimals.unwrap_or(4))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_value_shows_as_many_decimals_as_its_step() {
        assert_eq!(super::format_step(4.0, 1.0), "4");
        assert_eq!(super::format_step(0.5, 0.5), "0.5");
        assert_eq!(super::format_step(1.25, 0.25), "1.25");
        assert_eq!(super::format_step(0.1 + 0.2, 0.1), "0.3");
    }
}
