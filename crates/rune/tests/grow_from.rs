//! `grow_from` starts a row's child at a basis, grows it into the free space and lets it
//! shrink below the basis: in a wrapping row the basis decides when the child takes a line of
//! its own, and there it fits the line, where a child held by `min_w` runs past it.
use rune::runtime::ElementInfo;
use rune::{Cx, El, Env, Runtime, col, el, row};

struct S {
    width: f32,
}

fn card(key: &str, words: El) -> El {
    row().key(key).w_full().flex_wrap().child(words).child(el().key(format!("{key}.control")).w(100.0).h(10.0))
}

fn view(s: &S, _: &mut Cx<S>) -> El {
    col().w_full().h_full().items_start().child(
        col()
            .w(s.width)
            .child(card("from", el().key("from.words").h(10.0).grow_from(1.0, 150.0)))
            .child(card("held", el().key("held.words").h(10.0).grow(1.0).min_w(150.0))),
    )
}

fn rect(rt: &Runtime<S>, key: &str) -> [f32; 4] {
    let e: ElementInfo = rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("{key}"));
    e.rect
}

fn laid_out(width: f32) -> Runtime<S> {
    let mut rt = Runtime::new(S { width }, view, (600.0, 400.0), Env::new(true, 2.0));
    rt.frame(0.0);
    rt
}

#[test]
fn a_child_grown_from_a_basis_fills_its_line_wraps_at_the_basis_and_then_fits_its_line() {
    let wide = laid_out(400.0);
    assert_eq!(rect(&wide, "from.words")[2], 300.0, "it takes the line beside the control");
    assert_eq!(rect(&wide, "from.control")[1], rect(&wide, "from.words")[1], "the control shares its line");

    let wraps = laid_out(240.0);
    assert_eq!(rect(&wraps, "from.words")[2], 240.0, "under 150 + 100 it takes a line of its own");
    assert!(rect(&wraps, "from.control")[1] > rect(&wraps, "from.words")[1], "the control moves under it");

    let narrow = laid_out(120.0);
    assert_eq!(rect(&narrow, "from.words")[2], 120.0, "it fits a line narrower than its basis");
    assert_eq!(rect(&narrow, "held.words")[2], 150.0, "a child held by min_w runs past the line");
}
