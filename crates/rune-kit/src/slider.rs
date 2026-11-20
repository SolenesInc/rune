//! Slider: a value along a track, as `NSSlider` draws one. The pointer drags or presses the
//! knob anywhere on the track; arrows, Page Up and Down step it, and Home and End go to its ends.

use std::ops::RangeInclusive;

use rune::{El, Phase, Tx, el, row};

use crate::{Kit, shadows};

/// The knob's diameter and the track's thickness, in points.
const KNOB: f32 = 16.0;
const TRACK: f32 = 4.0;

/// A slider's new value. `continues` is true for the changes of a drag after its first, so an
/// app can keep a whole drag as one undo step; a press, a key and the first change of a drag
/// start a new one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderChange {
    pub value: f64,
    pub continues: bool,
}

/// The slider's place and step: value, minimum, maximum, step.
#[derive(Clone, Copy)]
struct Place {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
}

impl Place {
    fn snap(self, v: f64) -> f64 {
        (self.min + ((v - self.min) / self.step).round() * self.step).clamp(self.min, self.max)
    }
}

/// Gives the root its pointer and key handlers, typed on the app's state.
type Attach = Box<dyn FnOnce(El, Place) -> El>;

/// `Slider::new("Size", 12.0, 1.0..=64.0).step(1.0).on_change(..).build(&kit)`. The parts
/// are keyed `fill` (the track up to the knob), `knob` and `track` (the rest).
pub struct Slider {
    label: String,
    value: f64,
    min: f64,
    max: f64,
    step: Option<f64>,
    reading: Option<String>,
    width: Option<f32>,
    disabled: bool,
    attach: Option<Attach>,
}

impl Slider {
    /// `label` is the accessible name ("Brush size"); draw a visible one beside it yourself.
    pub fn new(label: impl Into<String>, value: f64, range: RangeInclusive<f64>) -> Slider {
        Slider {
            label: label.into(),
            value,
            min: *range.start(),
            max: range.end().max(*range.start()),
            step: None,
            reading: None,
            width: None,
            disabled: false,
            attach: None,
        }
    }
    /// The values the slider stops at, from its minimum. Unset: a hundredth of the range.
    pub fn step(mut self, step: f64) -> Self {
        self.step = (step > 0.0).then_some(step);
        self
    }
    /// What a screen reader reads for the value, as "12 px" or "40%". Unset: the number.
    pub fn reading(mut self, r: impl Into<String>) -> Self {
        self.reading = Some(r.into());
        self
    }
    /// Width in points. Unset: the parent's width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    pub fn disabled(mut self, d: bool) -> Self {
        self.disabled = d;
        self
    }
    /// Called with each new value; a value the slider already shows calls nothing.
    pub fn on_change<S: 'static>(mut self, f: impl Fn(&mut S, SliderChange, &mut Tx) + 'static) -> Self {
        self.attach = Some(Box::new(move |root: El, p: Place| {
            let f = std::rc::Rc::new(f);
            let emit = move |s: &mut S, value: f64, continues: bool, tx: &mut Tx| {
                let changed = value != p.value;
                if changed {
                    f(s, SliderChange { value, continues }, tx);
                }
                changed
            };
            let (keys, ends) = (emit.clone(), emit.clone());
            root.on_drag(move |s: &mut S, d: &mut rune::Drag, tx: &mut Tx| {
                if d.phase == Phase::Up && d.cancelled() {
                    return;
                }
                let at = f64::from(((d.x - KNOB / 2.0) / (d.w - KNOB).max(1.0)).clamp(0.0, 1.0));
                // `data` is 1 once this gesture has changed the value.
                if emit(s, p.snap(p.min + at * (p.max - p.min)), d.data != 0.0, tx) {
                    d.data = 1.0;
                }
            })
            .on_step(move |s: &mut S, delta, tx: &mut Tx| {
                keys(s, p.snap(p.value + f64::from(delta) * p.step), false, tx);
            })
            .on_step_limit(move |s: &mut S, end, tx: &mut Tx| {
                ends(s, if end { p.max } else { p.min }, false, tx);
            })
        }));
        self
    }

    pub fn build(self, k: &Kit) -> El {
        let c = &k.c;
        let (min, max) = (self.min, self.max);
        let step = self.step.unwrap_or((max - min) / 100.0).max(f64::EPSILON);
        let value = self.value.clamp(min, max);
        let fraction = if max > min { ((value - min) / (max - min)) as f32 } else { 0.0 };
        let enabled = !self.disabled;
        let bar =
            |key: &'static str, grow: f32, color| el().key(key).w(0.0).grow(grow).h(TRACK).fill(color).a11y_hidden();
        let knob = shadows(
            k,
            el().key("knob")
                .size(KNOB, KNOB)
                .radius(KNOB / 2.0)
                .fill(c.knob)
                .gradient(k.on(c.depth_top, c.knob), k.on(c.depth_bottom, c.knob))
                .border(0.5, c.border_base)
                .a11y_hidden(),
            &k.elevation.elements,
        );
        // The knob sits between the filled and empty parts of the track, so its left edge is
        // `fraction` of the width the knob does not take.
        let mut root = row()
            .role("slider")
            .label(self.label)
            .range_value(value, min, max)
            .range_step(step)
            .h(KNOB + 4.0)
            .center_y()
            .radius(KNOB / 2.0)
            .focus_ring(2.0, 1.0, c.border_focus)
            .disabled(!enabled)
            .child(bar("fill", fraction, if enabled { c.bg_accent } else { c.layer_04 }).corners([
                TRACK / 2.0,
                0.0,
                0.0,
                TRACK / 2.0,
            ]))
            .child(knob)
            .child(bar("track", 1.0 - fraction, c.layer_04).corners([0.0, TRACK / 2.0, TRACK / 2.0, 0.0]));
        root = match self.width {
            Some(w) => root.w(w),
            None => root.w_full(),
        };
        if let Some(r) = self.reading {
            root = root.a11y_value(r);
        }
        if !enabled {
            return root.layer_opacity(0.5);
        }
        match self.attach {
            Some(attach) => attach(root, Place { value, min, max, step }),
            None => root,
        }
    }
}
