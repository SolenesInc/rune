//! `ScrollBarStyle::room`: a bar that isn't an overlay draws in a strip its viewport keeps clear
//! only while the content overflows, as a browser's classic scroll bar does; and
//! `ScrollBarStyle::both_ways` puts a vertical and a horizontal bar on one viewport.
use rune::{Cx, El, Env, Runtime, ScrollBars, ScrollSpec, col, el, row};
use rune_kit::{ScrollBar, ScrollBarStyle};

#[derive(Clone, Copy)]
struct S {
    rows: usize,
    look: Look,
}

#[derive(Clone, Copy)]
enum Look {
    /// An 8 point styled bar shown whenever the content overflows, as a `::-webkit-scrollbar`.
    Always,
    /// The kit's bar, as the system "Show scroll bars" setting says.
    System,
    /// The 8 point bar on a 6 by 6 grid of 100 pt cells scrolling both ways.
    BothWays,
}

fn style(look: Look) -> ScrollBarStyle {
    let styled = ScrollBarStyle::native(rune::Color::WHITE, rune::Color::WHITE).webkit_parts(8.0, 0.0, 0.0).always().room();
    match look {
        Look::Always => styled,
        Look::System => ScrollBarStyle::neutral(true).room(),
        Look::BothWays => styled.both_ways(),
    }
}

fn view(s: &S, _: &mut Cx<S>) -> El {
    let viewport = match s.look {
        Look::BothWays => {
            let grid = (0..6).fold(col(), |c, r| c.child((0..6).fold(row(), |row, i| row.child(el().key(format!("cell{r}-{i}")).w(100.0).h(100.0)))));
            col().key("view").w(200.0).h(200.0).scroll_xy().child(grid)
        }
        _ => (0..s.rows).fold(col().key("view").w(200.0).h(200.0).scroll(ScrollSpec { stick_bottom: false }), |c, i| {
            c.child(el().key(format!("row{i}")).w_full().h(40.0))
        }),
    };
    col().size(300.0, 300.0).child(viewport.scroll_bar_styled(style(s.look)))
}

fn app(s: S, bars: ScrollBars) -> Runtime<S> {
    let mut rt = Runtime::new(s, view, (300.0, 300.0), Env::new(true, 1.0));
    rt.set_system_scroll_bars(bars);
    rt.set_system_scroller_overlay(bars != ScrollBars::Always);
    settle(&mut rt);
    rt
}

fn settle(rt: &mut Runtime<S>) {
    for _ in 0..600 {
        if !rt.frame(1.0 / 60.0) {
            return;
        }
    }
    panic!("never settled");
}

fn rect(rt: &Runtime<S>, key: &str) -> [f32; 4] {
    rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("{key}")).rect
}

#[test]
fn a_styled_bar_takes_its_strip_from_the_rows_only_while_they_overflow() {
    let mut rt = app(S { rows: 3, look: Look::Always }, ScrollBars::WhenScrolling);
    assert_eq!(rect(&rt, "row0")[2], 200.0, "rows that fit keep the whole width");

    rt.update(|s, _| s.rows = 12);
    settle(&mut rt);
    assert_eq!(rect(&rt, "row0")[2], 192.0, "overflowing rows end where the bar's strip starts");
    let [x, _, w, _] = rect(&rt, "scroll-bar");
    assert_eq!((x, x + w), (192.0, 200.0), "the bar draws in the strip, over none of the rows");

    rt.update(|s, _| s.rows = 2);
    settle(&mut rt);
    assert_eq!(rect(&rt, "row0")[2], 200.0, "once they fit again, the rows take the strip back");
}

#[test]
fn a_bar_that_follows_the_system_takes_room_only_while_the_system_shows_classic_bars() {
    let classic = app(S { rows: 12, look: Look::System }, ScrollBars::Always);
    let [x, _, w, _] = rect(&classic, "scroll-bar");
    assert_eq!(rect(&classic, "row0")[2], x, "classic bars take their strip from the rows");
    assert_eq!(x + w, 200.0);

    let overlay = app(S { rows: 12, look: Look::System }, ScrollBars::WhenScrolling);
    assert_eq!(rect(&overlay, "row0")[2], 200.0, "an overlay bar draws over the rows' end edge");
}

#[test]
fn a_viewport_scrolling_both_ways_carries_both_bars_and_leaves_the_corner_between_them() {
    let rt = app(S { rows: 0, look: Look::BothWays }, ScrollBars::WhenScrolling);
    let [x, y, w, h] = rect(&rt, "scroll-bar");
    assert_eq!((x, y, x + w, y + h), (192.0, 0.0, 200.0, 192.0), "the vertical bar runs down the end strip to the corner");
    let [x, y, w, h] = rect(&rt, "scroll-bar-x");
    assert_eq!((x, y, x + w, y + h), (0.0, 192.0, 192.0, 200.0), "the horizontal bar runs along the bottom strip to the corner");
}
