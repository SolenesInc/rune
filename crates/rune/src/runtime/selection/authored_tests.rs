use std::ops::Range;

use crate::{Cx, Direction, El, Env, Runtime, TextStyle, col, text};

#[derive(Default)]
struct S { heard: Option<(Range<usize>, Range<usize>)> }

fn view(_: &S, _: &mut Cx<S>) -> El {
    col().w_full().selection_group(true)
        .on_selection_change(|s: &mut S, c, _| s.heard = c.selection().and_then(|t| t.texts().first().map(|p| (p.range(), p.authored()))))
        .child(text("ab xyz cd").key("t").font(TextStyle::new(14.0, 400.0)).w(300.0).selectable(true).isolate(3..6, Direction::Ltr))
}

fn selected(range: Range<usize>) -> (Range<usize>, Range<usize>) {
    let mut rt = Runtime::new(S::default(), view, (400.0, 100.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.update(|_, tx| tx.select_text("t", range));
    rt.frame(1.0 / 60.0);
    rt.frame(2.0 / 60.0);
    rt.state().heard.clone().expect("the selection was heard")
}

#[test]
fn a_selection_after_an_isolate_is_its_authored_range_without_the_marks() {
    let (drawn, authored) = selected(13..15);
    assert_eq!(drawn, 13..15, "the drawn text holds the two 3-byte marks");
    assert_eq!(authored, 7..9, "\"cd\" as written");
}

#[test]
fn a_selection_inside_an_isolate_drops_only_the_opening_mark() {
    let (_, authored) = selected(7..8);
    assert_eq!(authored, 4..5, "\"y\" as written");
}
