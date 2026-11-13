use crate::{Cx, El, Env, Link, Runtime, TextStyle, col, text};

#[derive(Default)]
struct S { pressed: Vec<String> }

fn view(_: &S, _: &mut Cx<S>) -> El {
    col().w_full().padding(10.0).child(text("open this passage now").key("t").font(TextStyle::new(14.0, 400.0)).w(400.0).selectable(true)
        .link(Link::new(5..17, "mark").drag_selects())
        .on_link(|s: &mut S, id, _| s.pressed.push(id.to_owned())))
}

#[test]
fn a_drag_from_a_drag_selects_link_selects_and_a_click_on_it_presses_it() {
    let mut rt = Runtime::new(S::default(), view, (500.0, 100.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let mut glyphs: Vec<(f32, f32)> = rt.instances.iter()
        .filter(|i| i.params[1] == crate::render::KIND_GLYPH)
        .map(|i| (i.rect[0] + i.rect[2] / 2.0, i.rect[1] + i.rect[3] / 2.0))
        .collect();
    glyphs.sort_by(|a, b| a.0.total_cmp(&b.0));
    // "open" is glyphs 0..4, "this" 4..8, "passage" 8..15, "now" 15..18.
    let at = |g: usize| glyphs[g];
    let (x, y) = at(6);
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.frame(1.0 / 60.0);
    rt.pointer_up(x, y);
    rt.frame(2.0 / 60.0);
    assert_eq!(rt.state.pressed, ["mark"], "a click presses the link");
    let ((x0, y0), (x1, _)) = (at(5), at(16));
    rt.pointer_move(x0, y0);
    rt.pointer_down(x0, y0);
    rt.frame(1.0);
    for k in 1..=6 {
        rt.pointer_move(x0 + (x1 - x0) * k as f32 / 6.0, y0);
        rt.frame(1.0 + k as f64 / 60.0);
    }
    rt.pointer_up(x1, y0);
    rt.frame(2.0);
    assert!(rt.selected_text().is_some_and(|t| t.contains("passage")), "a drag from the link selects: {:?}", rt.selected_text());
    assert_eq!(rt.state.pressed, ["mark"], "and presses nothing");
}
