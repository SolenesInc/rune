//! An app that draws its own editor lines moves its caret and hit-tests on the very layout Rune
//! lays them out with: `rune::text::layout` hands it out, and Parley's editing types work on it.
use rune::TextStyle;
use rune::parley::editing::{Cursor, Selection};
use rune::parley::layout::Affinity;
use rune::text::{Span, advance, layout, measure};

fn style() -> TextStyle {
    TextStyle::new(15.0, 400.0).lh(22.0)
}

#[test]
fn a_line_s_layout_places_the_caret_after_the_text_before_it_and_a_press_finds_that_byte() {
    let text = "Hello, world";
    let l = layout(text, &[], style(), None);
    let caret = Cursor::from_byte_index(&l, 5, Affinity::Downstream).geometry(&l, 1.0);
    assert!(
        (caret.x0 as f32 - advance("Hello", style())).abs() < 0.5,
        "{caret:?}"
    );
    let pressed = Selection::from_point(&l, caret.x0 as f32 + 0.5, 5.0);
    assert_eq!(pressed.focus().index(), 5);
    let end = Selection::from_point(&l, 10_000.0, 5.0);
    assert_eq!(end.focus().index(), text.len());
}

#[test]
fn a_wrapped_line_s_layout_moves_by_visual_lines_and_matches_what_measure_says() {
    let text = "amber lantern orchard quiet harbor copper meadow signal drift pebble willow ember";
    let width = Some(160.0);
    let l = layout(text, &[], style(), width);
    let (_, h) = measure(text, &[], style(), width);
    assert!(
        l.len() >= 3,
        "it wraps onto three lines or more: {}",
        l.len()
    );
    assert!((l.height() - h).abs() < 1.0, "{} vs {h}", l.height());
    let start = Selection::from_byte_index(&l, 0, Affinity::Downstream);
    let down = start.move_lines(&l, 1, false);
    assert!(down.focus().index() > 0, "Down leaves the first line");
    let boxes = Selection::new(
        Cursor::from_byte_index(&l, 0, Affinity::Downstream),
        Cursor::from_byte_index(&l, text.len(), Affinity::Upstream),
    )
    .geometry(&l);
    assert_eq!(boxes.len(), l.len());
}

#[test]
fn spans_lay_out_as_rich_draws_them() {
    let text = "# Heading";
    let big = [Span::new(2, text.len()).size(30.0)];
    let plain = layout(text, &[], style(), None);
    let sized = layout(text, &big, style(), None);
    assert!(
        sized.width() > plain.width() * 1.5,
        "{} {}",
        sized.width(),
        plain.width()
    );
}
