//! `Tx::scroll_by`, `Tx::scroll_fraction` and `Tx::scroll_adjacent_key`: moving a viewport from
//! where it stands by points, by a share of its height, or to the next of a set of elements.
use rune::{Cx, El, Env, Runtime, ScrollSpec, col, el};

struct S { rows: usize, stick: bool }

// 40 pt rows in a 200 pt viewport.
fn view(s: &S, _: &mut Cx<S>) -> El {
    let rows = (0..s.rows).fold(col().w_full(), |c, i| c.child(el().key(format!("row{i}")).w_full().h(40.0)));
    col().w_full().h_full().child(col().key("list").w_full().h(200.0).scroll(ScrollSpec { stick_bottom: s.stick }).child(rows))
}

fn app(rows: usize, stick: bool) -> Runtime<S> {
    let mut rt = Runtime::new(S { rows, stick }, view, (300.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn top(rt: &Runtime<S>) -> f32 {
    rt.elements().into_iter().find(|e| e.key.as_deref() == Some("row0")).expect("row0").rect[1]
}

fn step(rt: &mut Runtime<S>, f: impl FnOnce(&mut rune::Tx)) -> f32 {
    rt.update(|_, tx| f(tx));
    rt.frame(1.0 / 60.0);
    -top(rt)
}

fn settle(rt: &mut Runtime<S>) {
    for _ in 0..600 { if !rt.frame(1.0 / 60.0) { return; } }
    panic!("never settled");
}

#[test]
fn scroll_by_moves_from_where_it_stands_and_clamps_to_both_ends() {
    let mut rt = app(20, false);
    assert_eq!(step(&mut rt, |tx| tx.scroll_by("list", 50.0)), 50.0);
    assert_eq!(step(&mut rt, |tx| tx.scroll_by("row3", 24.0)), 74.0, "a key inside finds its viewport");
    assert_eq!(step(&mut rt, |tx| tx.scroll_by("list", -500.0)), 0.0, "clamped to the top");
    assert_eq!(step(&mut rt, |tx| tx.scroll_by("list", 10_000.0)), 600.0, "clamped to the end");
}

#[test]
fn scroll_fraction_moves_by_shares_of_the_viewport_height() {
    let mut rt = app(20, false);
    assert_eq!(step(&mut rt, |tx| tx.scroll_fraction("list", 0.5)), 100.0);
    assert_eq!(step(&mut rt, |tx| tx.scroll_fraction("list", 1.5)), 400.0);
    assert_eq!(step(&mut rt, |tx| tx.scroll_fraction("list", -0.5)), 300.0);
    assert_eq!(step(&mut rt, |tx| tx.scroll_fraction("list", 9.0)), 600.0, "clamped to the end");
}

#[test]
fn a_relative_scroll_stops_a_glide_and_a_fling() {
    let mut rt = app(20, false);
    rt.update(|_, tx| tx.scroll_to("list", 0.0, 600.0));
    for _ in 0..3 { rt.frame(1.0 / 60.0); }
    let gliding = -top(&rt);
    assert!(gliding > 0.0 && gliding < 600.0, "on its way: {gliding}");
    let stepped = step(&mut rt, |tx| tx.scroll_by("list", 10.0));
    assert!((stepped - (gliding + 10.0)).abs() < 1e-3, "{gliding} + 10 = {stepped}");
    settle(&mut rt);
    assert_eq!(-top(&rt), stepped, "the glide stopped");

    rt.fling(150.0, 100.0, -2_000.0);
    rt.frame(1.0 / 60.0);
    let flung = -top(&rt);
    let stepped = step(&mut rt, |tx| tx.scroll_fraction("list", -0.5));
    assert!((stepped - (flung - 100.0)).abs() < 1e-3, "{flung} - 100 = {stepped}");
    settle(&mut rt);
    assert_eq!(-top(&rt), stepped, "the fling stopped");
}

#[test]
fn a_stick_bottom_viewport_leaves_its_end_and_follows_it_again_once_back() {
    let mut rt = app(20, true);
    assert_eq!(-top(&rt), 600.0, "opens at the end");
    assert_eq!(step(&mut rt, |tx| tx.scroll_by("list", -40.0)), 560.0);
    rt.update(|s, _| s.rows = 21);
    rt.frame(1.0 / 60.0);
    assert_eq!(-top(&rt), 560.0, "off the end it stays put as rows arrive");
    assert_eq!(step(&mut rt, |tx| tx.scroll_fraction("list", 1.0)), 640.0);
    rt.update(|s, _| s.rows = 22);
    rt.frame(1.0 / 60.0);
    assert_eq!(-top(&rt), 680.0, "back on the end it follows it");
}

#[test]
fn scroll_adjacent_key_steps_through_keys_from_wherever_the_viewport_stands() {
    let mut rt = app(20, false);
    let keys = || ["row0", "row5", "row10", "row15", "missing"].map(String::from).to_vec();
    let next = |rt: &mut Runtime<S>, margin| step(rt, |tx| tx.scroll_adjacent_key("list", keys(), true, margin));
    let prev = |rt: &mut Runtime<S>, margin| step(rt, |tx| tx.scroll_adjacent_key("list", keys(), false, margin));

    assert_eq!(next(&mut rt, 0.0), 200.0, "the key at the top doesn't count");
    assert_eq!(next(&mut rt, 0.0), 400.0, "repeated requests step on");
    assert_eq!(prev(&mut rt, 0.0), 200.0);
    assert_eq!(step(&mut rt, |tx| tx.scroll_to_y("list", 450.0)), 450.0);
    assert_eq!(next(&mut rt, 0.0), 600.0, "from a hand-moved offset, the next key below it");
    assert_eq!(next(&mut rt, 0.0), 600.0, "none below: it stays");
    assert_eq!(step(&mut rt, |tx| tx.scroll_to_y("list", 450.0)), 450.0);
    assert_eq!(prev(&mut rt, 0.0), 400.0, "from a hand-moved offset, the nearest key above it");
    assert_eq!(prev(&mut rt, 0.0), 200.0);
    assert_eq!(prev(&mut rt, 0.0), 0.0);
    assert_eq!(prev(&mut rt, 0.0), 0.0, "none above: it stays");

    assert_eq!(next(&mut rt, 12.0), 188.0, "the margin lands the key below the top");
    assert_eq!(next(&mut rt, 12.0), 388.0, "and the key it landed doesn't count");
    assert_eq!(prev(&mut rt, 12.0), 188.0);
}
