//! Swatches: a choice of colours, one round swatch each, as a radio group. Arrows move between
//! swatches and Space picks one. Given a width, the swatches wrap onto balanced rows, so no
//! row holds a swatch alone.

use rune::{Color, El, Tx, col, el, row};

use crate::Kit;

/// A swatch's diameter and the space between swatches, in points.
const SWATCH: f32 = 22.0;
const GAP: f32 = 8.0;

type Pick = Box<dyn Fn(usize) -> Box<dyn FnOnce(El) -> El>>;

/// `Swatches::new("Colour", &[(red, "Red"), (blue, "Blue")]).selected(Some(0)).build(&kit)`.
/// Each swatch is keyed `swatch{i}`, as SegmentedControl keys its segments.
pub struct Swatches {
    label: String,
    swatches: Vec<(Color, String)>,
    selected: Option<usize>,
    width: Option<f32>,
    pick: Option<Pick>,
}

impl Swatches {
    /// `label` names the choice ("Colour"); each swatch's name is what a screen reader reads
    /// for it ("Red") and its tooltip.
    pub fn new<S: AsRef<str>>(label: impl Into<String>, swatches: &[(Color, S)]) -> Swatches {
        Swatches {
            label: label.into(),
            swatches: swatches.iter().map(|(c, n)| (*c, n.as_ref().to_owned())).collect(),
            selected: None,
            width: None,
            pick: None,
        }
    }
    pub fn selected(mut self, i: Option<usize>) -> Self {
        self.selected = i;
        self
    }
    /// The width the swatches wrap in. Unset: one row.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    pub fn on_pick<S: 'static>(mut self, f: impl Fn(&mut S, usize, &mut Tx) + Clone + 'static) -> Self {
        self.pick = Some(Box::new(move |i| {
            let f = f.clone();
            Box::new(move |e: El| e.on_press_tx(move |s: &mut S, tx: &mut Tx| f(s, i, tx)))
        }));
        self
    }

    pub fn build(self, k: &Kit) -> El {
        let c = &k.c;
        let n = self.swatches.len();
        let mut lengths = balanced(n, self.width).into_iter();
        let mut left = lengths.next().unwrap_or(n);
        let mut root = col().role("radiogroup").label(self.label).focus_group().gap(GAP);
        if let Some(w) = self.width {
            root = root.w(w);
        }
        let mut line = row().gap(GAP);
        for (i, (color, name)) in self.swatches.into_iter().enumerate() {
            let on = self.selected == Some(i);
            let mut swatch = el()
                .key(format!("swatch{i}"))
                .role("radio")
                .label(name.clone())
                .help(name)
                .checked(on)
                .position_in_set(i + 1, n)
                .focusable(true)
                .focus_selected(on)
                .focus_ring(2.0, 2.0, c.border_focus)
                .size(SWATCH, SWATCH)
                .radius(SWATCH / 2.0)
                .fill(color)
                // A hairline keeps a swatch the colour of the page visible.
                .border(0.5, c.border_strong);
            if on {
                // A ring with a gap of the page between it and the colour.
                swatch = swatch.border(2.0, c.bg_base).box_shadow(rune::Shadow {
                    spread: 2.0,
                    color: c.text_base,
                    ..Default::default()
                });
            }
            if let Some(p) = &self.pick {
                swatch = p(i)(swatch);
            }
            line = line.child(swatch);
            left -= 1;
            if left == 0 {
                root = root.child(std::mem::replace(&mut line, row().gap(GAP)));
                left = lengths.next().unwrap_or(n);
            }
        }
        root
    }
}

/// How many swatches go on each row: as few rows as fit `width`, their lengths at most one
/// apart, longer rows first. With room for three or more, no row holds a swatch alone.
fn balanced(n: usize, width: Option<f32>) -> Vec<usize> {
    let fits = width.map_or(n, |w| (((w + GAP) / (SWATCH + GAP)).floor() as usize).max(1));
    let rows = n.div_ceil(fits.max(1)).max(1);
    (0..rows).map(|r| n / rows + usize::from(r < n % rows)).collect()
}

#[cfg(test)]
mod tests {
    use super::{GAP, SWATCH, balanced};

    #[test]
    fn rows_are_balanced_and_never_leave_one_swatch_alone() {
        let w = |fit: usize| Some(fit as f32 * (SWATCH + GAP) - GAP);
        for n in 1..=24 {
            for fit in 1..=12 {
                let rows = balanced(n, w(fit));
                assert_eq!(rows.iter().sum::<usize>(), n);
                assert_eq!(rows.len(), n.div_ceil(fit), "{n} in room for {fit}: as few rows as fit");
                let (lo, hi) = (rows.iter().min().unwrap(), rows.iter().max().unwrap());
                assert!(*hi <= fit && hi - lo <= 1, "{n} in room for {fit}: {rows:?}");
                if fit >= 3 && n >= 2 {
                    assert!(*lo >= 2, "{n} in room for {fit}: one swatch alone in {rows:?}");
                }
            }
        }
        assert_eq!(balanced(7, None), [7], "no width: one row");
        assert_eq!(balanced(7, w(6)), [4, 3], "Lens's seven presets in room for six");
    }
}
