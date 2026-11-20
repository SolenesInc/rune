use super::*;
use crate::el::ScrollSpec;

/// Twenty 30 pt rows in a viewport under a band `band` points tall, in a 300 pt window.
fn under_a_band(band: &f32, _: &mut Cx<f32>) -> El {
    let mut rows = col().key("rows").w_full();
    for i in 0..20 { rows = rows.child(el().key(format!("r{i}")).h(30.0).w_full()); }
    col().key("root").w_full().h(300.0)
        .child(el().key("band").h(*band).w_full())
        .child(col().key("scroll").w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).child(rows))
}

/// [`under_a_band`] with its viewport anchored (`El::scroll_anchor`).
fn anchored_under_a_band(band: &f32, _: &mut Cx<f32>) -> El {
    let mut rows = col().key("rows").w_full();
    for i in 0..20 { rows = rows.child(el().key(format!("r{i}")).h(30.0).w_full()); }
    col().key("root").w_full().h(300.0)
        .child(el().key("band").h(*band).w_full())
        .child(col().key("scroll").w_full().grow(1.0).min_h(0.0).scroll(ScrollSpec { stick_bottom: false }).scroll_anchor().child(rows))
}

fn top(rt: &Runtime<f32>, key: &str) -> f32 {
    rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap().rect[1]
}

fn offset(rt: &Runtime<f32>) -> f64 { rt.scrolls.values().next().unwrap().offset }

/// The rows scrolled 200 pt down, under no band.
fn scrolled(view: fn(&f32, &mut Cx<f32>) -> El) -> Runtime<f32> {
    let mut rt = Runtime::new(0.0f32, view, (200.0, 300.0), Env::new(true, 2.0));
    rt.frame(0.0);
    rt.wheel(100.0, 150.0, -200.0);
    for _ in 0..30 { rt.frame(1.0 / 60.0); }
    assert_eq!(offset(&rt), 200.0);
    rt
}

/// A band that arrives over a scrolled viewport moves the viewport's top down by its height; a
/// row kept on screen stays where it shows on the screen, on every frame, because the viewport
/// scrolls by the same height, and so on the band's way back.
#[test]
fn a_kept_row_holds_its_place_on_screen_when_its_viewport_s_top_moves() {
    let mut rt = scrolled(under_a_band);
    let was = top(&rt, "r10");
    for (band, scrolled) in [(60.0, 260.0), (0.0, 200.0)] {
        rt.update(|b, tx| { *b = band; tx.keep_on_screen("r10"); });
        for f in 0..30 {
            rt.frame(1.0 / 60.0);
            assert!((top(&rt, "r10") - was).abs() < 0.5, "band {band}, frame {f}: r10 at {} left {was}", top(&rt, "r10"));
        }
        assert!((offset(&rt) - scrolled).abs() < 0.5, "band {band}: offset {}", offset(&rt));
    }
}

/// A kept row that showed where a band landing at once now stands shows just under it, at the
/// viewport's new top, rather than under the band.
#[test]
fn a_kept_row_the_band_would_cover_lands_at_its_viewport_s_new_top() {
    let mut rt = scrolled(under_a_band);
    let row = "r7";
    assert!(top(&rt, row) < 60.0, "{row} shows in the band's room: {}", top(&rt, row));
    rt.update(|b, tx| { *b = 60.0; tx.keep_on_screen(row); tx.instant(); });
    for f in 0..30 {
        rt.frame(1.0 / 60.0);
        assert!((top(&rt, row) - 60.0).abs() < 0.5, "frame {f}: {row} at {}", top(&rt, row));
    }
}

/// A scroll anchor holds its row in its viewport, as CSS `overflow-anchor` does: a band that
/// moves the viewport's top takes the rows with it, and the viewport keeps its offset.
#[test]
fn an_anchored_viewport_whose_top_moves_keeps_its_offset() {
    let mut rt = scrolled(anchored_under_a_band);
    let was = top(&rt, "r10");
    rt.update(|b, _| *b = 60.0);
    for _ in 0..30 { rt.frame(1.0 / 60.0); }
    assert_eq!(offset(&rt), 200.0);
    assert!((top(&rt, "r10") - was - 60.0).abs() < 0.5, "r10 at {} left {was}", top(&rt, "r10"));
}
