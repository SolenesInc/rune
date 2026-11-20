//! `ElementInfo::text_ink`: the box a text's laid-out lines cover, which runs past the text's own
//! box when it overflows, which a clip check comparing boxes cannot see, and stays clear of the
//! text's padding.

use super::*;
use crate::{Cx, El, Env, TextStyle, col, text};

/// (the text's width, its words, whether it truncates, whether it wraps, its side padding)
type S = (f32, &'static str, bool, bool, f32);

fn view(s: &S, _: &mut Cx<S>) -> El {
    let (w, words, truncate, wrap, pad) = *s;
    let mut t = text(words).key("t").font(TextStyle::new(13.0, 400.0)).px(pad);
    if w > 0.0 {
        t = t.w(w);
    }
    if truncate {
        t = t.truncate();
    }
    if wrap {
        t = t.wrap();
    }
    col().w(400.0).items_start().child(t)
}

/// The text's box and its ink.
fn boxes(s: S) -> ([f32; 4], [f32; 4]) {
    let mut rt = Runtime::new(s, view, (400.0, 100.0), Env::new(true, 2.0));
    rt.frame(0.0);
    let e = rt.elements().into_iter().find(|e| e.key.as_deref() == Some("t")).unwrap();
    (e.rect, e.text_ink.expect("a text has ink"))
}

fn right(r: [f32; 4]) -> f32 {
    r[0] + r[2]
}

#[test]
fn a_line_wider_than_its_box_inks_past_its_box() {
    let (_, natural) = boxes((0.0, "Snooze this agent", false, false, 0.0));
    let (rect, ink) = boxes((40.0, "Snooze this agent", false, false, 0.0));
    assert!(natural[2] > 80.0, "the words draw about 100 pt: {natural:?}");
    assert!((ink[2] - natural[2]).abs() < 0.5, "{ink:?} keeps its {natural:?} width");
    assert!((right(ink) - right(rect) - (natural[2] - 40.0)).abs() < 1.0, "{ink:?} past {rect:?}");
}

#[test]
fn a_word_too_long_to_wrap_inks_past_its_box() {
    let (rect, ink) = boxes((40.0, "InternationalEngineeringObservability", false, true, 0.0));
    assert!(right(ink) - right(rect) > 100.0, "{ink:?} past {rect:?}");
}

#[test]
fn text_that_fits_or_shows_cut_inks_inside_its_box() {
    for s in [
        (200.0, "Snooze this agent", false, false, 0.0),
        (40.0, "Snooze this agent", true, false, 0.0),
        (70.0, "Snooze this agent wraps", false, true, 0.0),
    ] {
        let (rect, ink) = boxes(s);
        assert!(ink[0] >= rect[0] - 0.5 && right(ink) <= right(rect) + 0.5, "{s:?}: {ink:?} in {rect:?}");
    }
}

#[test]
fn ink_stays_clear_of_the_padding() {
    let (rect, ink) = boxes((0.0, "·", false, false, 6.0));
    assert!((ink[0] - rect[0] - 6.0).abs() < 0.5, "{ink:?} starts 6 pt into {rect:?}");
    assert!((right(rect) - right(ink) - 6.0).abs() < 0.5, "{ink:?} ends 6 pt before the end of {rect:?}");
}

#[test]
fn ink_scales_with_a_zoomed_ancestor() {
    fn zoomed(_: &(), _: &mut Cx<()>) -> El {
        crate::el().w(200.0).h(50.0).scale(2.0).child(text("Snooze").key("t").font(TextStyle::new(13.0, 400.0)).px(6.0))
    }
    let (_, plain) = boxes((0.0, "Snooze", false, false, 6.0));
    let mut rt = Runtime::new((), zoomed, (400.0, 100.0), Env::new(true, 2.0));
    rt.frame(0.0);
    let e = rt.elements().into_iter().find(|e| e.key.as_deref() == Some("t")).unwrap();
    let ink = e.text_ink.unwrap();
    assert!((ink[2] - 2.0 * plain[2]).abs() < 1.0, "{ink:?} is twice {plain:?}");
    assert!((ink[0] - e.rect[0] - 12.0).abs() < 1.0, "{ink:?} starts 12 pt into {:?}", e.rect);
}

#[test]
fn a_wrapped_line_inks_without_the_space_it_breaks_after() {
    let (_, line) = boxes((0.0, "Snooze this", false, false, 0.0));
    let (rect, ink) = boxes((line[2] + 1.0, "Snooze this Snooze this", false, true, 0.0));
    assert!(right(ink) <= right(rect) + 0.5, "{ink:?} in {rect:?}: the space after a line's last word draws nothing");
}
