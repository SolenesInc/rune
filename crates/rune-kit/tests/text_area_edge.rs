//! `TextArea::edge` draws an area as a web `<textarea>` with a 1 point border: one colour at
//! rest, another while it has focus, and none of the kit's ring or focus outline.

use rune::{Color, Cx, El, Env, Key, KeyEvent, Mods, Runtime, TextEdit, col};
use rune_kit::*;

const REST: Color = Color { r: 0.2, g: 0.6, b: 0.2, a: 1.0 };
const FOCUSED: Color = Color { r: 0.9, g: 0.1, b: 0.5, a: 1.0 };

fn view(s: &TextEdit, cx: &mut Cx<TextEdit>) -> El {
    let k = Kit::neutral(cx.system_dark());
    col().padding(20.0).child(
        TextArea::new(s).edge(REST, FOCUSED).resizable().on_edit(|s: &mut TextEdit| s).build(&k),
    )
}

fn borders(rt: &Runtime<TextEdit>) -> Vec<[f32; 4]> {
    rt.instances.iter().filter(|i| i.border[3] > 0.0).map(|i| i.border).collect()
}

#[test]
fn an_edged_area_draws_its_rest_border_then_its_focus_border_and_no_kit_ring() {
    let mut rt = Runtime::new(TextEdit::new(""), view, (400.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    assert_eq!(borders(&rt), [REST.premul(1.0)], "one border, the rest edge");

    rt.key(KeyEvent { key: Key::Tab, mods: Mods::default() });
    for _ in 0..60 { rt.frame(1.0 / 60.0); }
    assert!(rt.state().focused, "Tab focuses the area");
    assert_eq!(borders(&rt), [FOCUSED.premul(1.0)], "focused, the one border takes the focus edge");
}
