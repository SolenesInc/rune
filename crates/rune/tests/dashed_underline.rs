//! `Span::underline_dashed` underlines text in dashes, as a CSS `1px dashed` border draws:
//! dashes and gaps three widths long, starting and ending on a dash, where a plain underline
//! draws one line.
use rune::render::KIND_RECT;
use rune::{Color, Cx, El, Env, Runtime, Span, el};

const TEXT: &str = "an area note";

/// The underline rects drawn at 2× for `TEXT`, device px `(left, width)`, left to right.
fn underline(dashed: bool) -> Vec<(f32, f32)> {
    fn view(dashed: &bool, _: &mut Cx<bool>) -> El {
        let line = Color::WHITE.alpha(0.5);
        let span = Span::new(0, TEXT.len());
        el::rich(TEXT, vec![if *dashed { span.underline_dashed(line) } else { span.underline(line) }])
    }
    let mut rt = Runtime::new(dashed, view, (400.0, 60.0), Env::new(true, 2.0));
    rt.frame(0.0);
    let mut out: Vec<(f32, f32)> =
        rt.instances.iter().filter(|i| i.params[1] == KIND_RECT && i.rect[3] == 2.0).map(|i| (i.rect[0], i.rect[2])).collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

#[test]
fn a_dashed_underline_runs_the_plain_ones_length_in_three_pixel_dashes_and_gaps() {
    let solid = underline(false);
    assert_eq!(solid.len(), 1, "a plain underline is one line: {solid:?}");
    let (left, width) = solid[0];
    let dashes = underline(true);
    assert!(dashes.len() > 5, "the words take many dashes: {dashes:?}");
    assert_eq!(dashes[0].0, left, "the first dash starts where the line does");
    let last = dashes[dashes.len() - 1];
    assert_eq!(last.0 + last.1, left + width, "the last dash ends where the line does");
    for d in &dashes {
        assert_eq!(d.1, 6.0, "each dash is three points, six pixels at 2×: {dashes:?}");
    }
    for pair in dashes.windows(2) {
        let gap = pair[1].0 - (pair[0].0 + pair[0].1);
        assert!((5.0..=8.0).contains(&gap), "gaps stay near a dash's length: {dashes:?}");
    }
}
