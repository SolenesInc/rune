//! `El::on_measure` hears a commit's layout before the commit paints, and what its handler
//! changes lays the view out again in that commit.

use super::*;
use crate::el::row;

const BAR: f32 = 200.0;
const CHIP: f32 = 60.0;

/// A bar of chips that shows only the first when they overflow, as attn's queue bar does: it
/// remembers how wide the whole set was, from a layout that showed it, for as long as the set
/// is the same.
#[derive(Default)]
struct Bar {
    chips: usize,
    needs: Option<(usize, f32)>,
    heard: usize,
}

fn crowded(s: &Bar) -> bool {
    s.needs
        .is_some_and(|(chips, right)| chips == s.chips && right > BAR)
}

fn view(s: &Bar, _: &mut Cx<Bar>) -> El {
    let shown = if crowded(s) { 1 } else { s.chips };
    row().w(BAR).h(20.0).child(
        row()
            .key("chips")
            .children((0..shown).map(|i| el().key(format!("chip.{i}")).w(CHIP).h(20.0)))
            .on_measure(|s: &mut Bar, [x, _, w, _]| {
                s.heard += 1;
                if !crowded(s) {
                    s.needs = Some((s.chips, x + w));
                }
            }),
    )
}

fn shown(rt: &Runtime<Bar>) -> usize {
    rt.elements()
        .iter()
        .filter(|e| !e.exiting && e.key.as_deref().is_some_and(|k| k.starts_with("chip.")))
        .count()
}

#[test]
fn a_view_that_hides_what_overflows_never_draws_the_overflow() {
    let mut rt = Runtime::new(
        Bar {
            chips: 5,
            ..Bar::default()
        },
        view,
        (400.0, 100.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    assert_eq!(shown(&rt), 1, "the first frame draws the crowded bar");
    assert_eq!(
        rt.stats.commits, 1,
        "in the commit that measured the overflow"
    );
    let heard = rt.state().heard;
    rt.frame(1.0 / 60.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(
        rt.state().heard,
        heard,
        "a layout that holds still is not heard again"
    );

    rt.update(|s, _| s.chips = 3);
    rt.frame(1.0 / 60.0);
    assert_eq!(
        shown(&rt),
        3,
        "a set that fits shows whole in the frame it changed"
    );

    rt.update(|s, _| s.chips = 4);
    rt.frame(1.0 / 60.0);
    assert_eq!(
        shown(&rt),
        1,
        "one that overflows crowds in the frame it changed"
    );
}

#[test]
fn an_element_that_comes_back_is_heard_again() {
    fn view(s: &(bool, usize), _: &mut Cx<(bool, usize)>) -> El {
        col().w(100.0).h(100.0).children(s.0.then(|| {
            el().key("watched")
                .w(10.0)
                .h(10.0)
                .on_measure(|s: &mut (bool, usize), _| s.1 += 1)
        }))
    }
    let mut rt = Runtime::new((true, 0), view, (100.0, 100.0), Env::new(true, 1.0));
    rt.frame(0.0);
    assert_eq!(rt.state().1, 1);
    rt.update(|s, _| s.0 = false);
    rt.frame(1.0 / 60.0);
    rt.update(|s, _| s.0 = true);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state().1, 2);
}
