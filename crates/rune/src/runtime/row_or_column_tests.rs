//! `El::row_or_column`: children in a row where their natural row fits the width given, else a
//! column of children as wide as it, decided in the commit's layout from the offered width alone.
use super::*;
use crate::{Cx, El, Env, col, row, text};

const OPTIONS: [&str; 3] = ["Claude", "Codex", "Engineering observability staging review agent"];

fn options(track: El) -> El {
    track.key("track").gap(2.0).padding(2.0).children(
        OPTIONS.iter().enumerate().map(|(i, name)| {
            row().key(format!("o{i}")).shrink().min_w(0.0).px(8.0).center().child(text(*name).truncate().shrink().min_w(0.0))
        }),
    )
}

fn in_column(width: &f32, track: El) -> El {
    col().child(col().key("column").w(*width).items_start().child(track.shrink().min_w(0.0).max_w_frac(1.0)))
}

fn row_or_column(width: &f32, _: &mut Cx<f32>) -> El {
    in_column(width, options(row().row_or_column()))
}

fn plain_row(width: &f32, _: &mut Cx<f32>) -> El {
    in_column(width, options(row()))
}

fn run(width: f32, view: fn(&f32, &mut Cx<f32>) -> El) -> Runtime<f32> {
    let mut rt = Runtime::new(width, view, (1200.0, 400.0), Env::new(true, 2.0));
    rt.frame(0.0);
    rt
}

fn rects<S: 'static>(rt: &Runtime<S>) -> Vec<[f32; 4]> {
    let all = rt.elements();
    (0..OPTIONS.len())
        .map(|i| all.iter().find(|e| e.key.as_deref() == Some(&format!("o{i}"))).unwrap().target)
        .collect()
}

fn natural(rt: &mut Runtime<f32>) -> f32 {
    rt.update(|w, _| *w = 1000.0);
    rt.frame(1.0);
    let all = rt.elements();
    all.iter().find(|e| e.key.as_deref() == Some("track")).unwrap().target[2]
}

fn stacked(r: &[[f32; 4]]) -> bool {
    r.windows(2).all(|w| w[1][1] > w[0][1]) && r.iter().all(|o| o[0] == r[0][0] && o[2] == r[0][2])
}

fn one_line(r: &[[f32; 4]]) -> bool {
    r.iter().all(|o| o[1] == r[0][1])
}

#[test]
fn a_row_that_fits_lays_out_as_a_plain_row_does() {
    for width in [1000.0, 600.0] {
        let (a, b) = (run(width, row_or_column), run(width, plain_row));
        assert_eq!(rects(&a), rects(&b), "at {width}");
        assert!(one_line(&rects(&a)));
    }
}

/// The options under a heading and beside words, so a row out of place shows.
fn beside_words(width: &f32, track: El) -> El {
    col().child(
        col().w(*width).gap(6.0)
            .child(text("Default agent"))
            .child(row().gap(8.0).child(text("Used for new sessions").grow(1.0).shrink().min_w(0.0)).child(track.shrink().min_w(0.0))),
    )
}

#[test]
fn a_row_that_fits_beside_words_keeps_its_place() {
    fn flagged(w: &f32, _: &mut Cx<f32>) -> El { beside_words(w, options(row().row_or_column())) }
    fn plain(w: &f32, _: &mut Cx<f32>) -> El { beside_words(w, options(row())) }
    let (mut a, b) = (run(1000.0, flagged), run(1000.0, plain));
    assert_eq!(rects(&a), rects(&b));
    a.update(|_, _| {});
    a.frame(1.0);
    assert_eq!(rects(&a), rects(&b), "a commit whose layout comes from the cache keeps it there");
    assert!(rects(&a)[0][1] > 0.0, "under the heading");
}

#[test]
fn a_row_cut_short_becomes_a_column_of_one_width_as_wide_as_it_is_given() {
    let rt = run(160.0, row_or_column);
    let r = rects(&rt);
    assert!(stacked(&r), "{r:?}");
    assert_eq!(r[0][2], 160.0 - 4.0, "each option spans the column inside the track's padding");
}

#[test]
fn it_turns_at_its_natural_row_widening_or_narrowing_and_holds_still() {
    let mut rt = run(160.0, row_or_column);
    let natural = natural(&mut rt);
    let widths: Vec<f32> = (-8..=8).map(|i| natural + 0.25 * i as f32).collect();
    let mut at = Vec::new();
    for (pass, ws) in [widths.clone(), widths.iter().rev().copied().collect()].into_iter().enumerate() {
        for w in ws {
            rt.update(|width, _| *width = w);
            rt.frame(2.0 + pass as f64);
            let r = rects(&rt);
            rt.frame(3.0 + pass as f64);
            assert_eq!(rects(&rt), r, "at {w} the options moved on a frame with no change");
            assert!(!rt.animating, "at {w} it keeps animating");
            assert!(one_line(&r) || stacked(&r), "at {w}: {r:?}");
            at.push((pass, w, stacked(&r)));
        }
    }
    for &(_, w, is_stacked) in &at {
        assert_eq!(is_stacked, w < natural, "at {w}, natural row {natural}");
    }
}

#[test]
fn a_window_resize_turns_it_without_running_the_view() {
    fn view(_: &f32, _: &mut Cx<f32>) -> El {
        col().w_full().items_start().child(options(row().row_or_column()).shrink().min_w(0.0).max_w_frac(1.0))
    }
    let mut rt = run(0.0, view);
    assert!(one_line(&rects(&rt)));
    rt.set_viewport((160.0, 400.0), 2.0);
    rt.frame(1.0);
    assert!(stacked(&rects(&rt)), "{:?}", rects(&rt));
    rt.set_viewport((1200.0, 400.0), 2.0);
    rt.frame(2.0);
    assert!(one_line(&rects(&rt)), "{:?}", rects(&rt));
}

/// A scroll viewport that takes 16 points for its bar once its content overflows.
fn scroll(track: El, tall: bool) -> El {
    col().w_full().child(
        col().w_full().h(90.0).items_start().key("scroll")
            .scroll(crate::ScrollSpec { stick_bottom: false }).scroll_room(16.0)
            .child(track.shrink().min_w(0.0).max_w_frac(1.0))
            .child(crate::el().h(if tall { 180.0 } else { 12.0 })),
    )
}

#[test]
fn room_a_scroll_bar_takes_after_the_first_pass_is_room_it_decides_in() {
    fn view(tall: &bool, _: &mut Cx<bool>) -> El { scroll(options(row().row_or_column()), *tall) }
    let mut wide = Runtime::new(false, view, (1200.0, 400.0), Env::new(true, 2.0));
    wide.frame(0.0);
    let natural = wide.elements().iter().find(|e| e.key.as_deref() == Some("track")).unwrap().target[2];
    let mut rt = Runtime::new(true, view, (natural + 8.0, 400.0), Env::new(true, 2.0));
    rt.frame(0.0);
    let first = rects(&rt);
    assert!(stacked(&first), "a row that fits only without the bar stacks once the bar takes its room: {first:?}");
    rt.frame(1.0);
    rt.update(|_, _| {});
    rt.frame(2.0);
    assert_eq!(rects(&rt), first, "a later commit at the same width keeps the form");
    rt.update(|tall, _| *tall = false);
    rt.frame(3.0);
    let short = rects(&rt);
    assert!(one_line(&short), "with the bar gone the row fits again: {short:?}");
    rt.frame(4.0);
    assert_eq!(rects(&rt), short);
    assert!(!rt.animating);
}

#[test]
fn it_turns_at_one_width_at_every_text_zoom() {
    fn view(zoom: &f32, cx: &mut Cx<f32>) -> El {
        cx.zoom(*zoom);
        col().w_full().padding(8.0).items_start().child(options(row().row_or_column()).shrink().min_w(0.0).max_w_frac(1.0))
    }
    for zoom in [0.7, 1.0, 1.5] {
        let mut rt = Runtime::new(zoom, view, (1200.0, 400.0), Env::new(true, 2.0));
        rt.frame(0.0);
        let natural = rt.elements().iter().find(|e| e.key.as_deref() == Some("track")).unwrap().rect[2];
        let threshold = ((natural + 16.0) * zoom).round();
        let widths: Vec<f32> = (-12..=12).map(|i| threshold + i as f32).collect();
        let mut forms = Vec::new();
        for sweep in [widths.clone(), widths.iter().rev().copied().collect::<Vec<_>>()] {
            let mut at = Vec::new();
            for w in sweep {
                rt.set_viewport((w, 400.0), 2.0);
                rt.frame(1.0);
                let r = rects(&rt);
                rt.frame(1.0);
                assert_eq!(rects(&rt), r, "zoom {zoom} at {w}");
                assert!(!rt.animating);
                at.push(stacked(&r));
            }
            assert_eq!(at.windows(2).filter(|p| p[0] != p[1]).count(), 1, "zoom {zoom}: {at:?}");
            forms.push(at);
        }
        forms[1].reverse();
        assert_eq!(forms[0], forms[1], "zoom {zoom}: the same form at each width either way");
    }
}

#[test]
#[should_panic(expected = "inside another")]
fn one_inside_another_is_refused() {
    fn view(_: &f32, _: &mut Cx<f32>) -> El {
        col().w_full().child(row().row_or_column().child(options(row().row_or_column())).child(text("more")))
    }
    run(165.0, view);
}
