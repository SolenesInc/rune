//! Moving the keyboard's focus shows the element that draws the focused control's ring, so a
//! field whose box rings its editor shows whole, ring and all, inside the viewport's margin.

use super::*;
use crate::{Color, Cx, El, Env, Key, KeyEvent, Mods, ScrollSpec, col, el};

/// A 200 point viewport with a 4 point scroll margin over twenty 40 point boxes, each ringing
/// the 16 point editor 8 points inside it while focus is in it.
fn view(_: &(), _: &mut Cx<()>) -> El {
    col()
        .key("body")
        .size(300.0, 200.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .scroll_margin(4.0)
        .child(el().w_full().h(30.0))
        .children((0..20).map(|i| {
            col()
                .key(format!("box{i}"))
                .w_full()
                .h(40.0)
                .pt(8.0)
                .focus_within_ring(2.0, 2.0, Color::WHITE)
                .child(el().key(format!("editor{i}")).w_full().h(16.0).focusable(true))
        }))
}

fn top(rt: &Runtime<()>, key: &str) -> f32 {
    rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("no {key}")).rect[1]
}

#[test]
fn tab_shows_the_box_that_rings_the_focused_editor_inside_the_margin() {
    let mut rt = Runtime::new((), view, (300.0, 200.0), Env::new(true, 1.0));
    rt.set_full_keyboard_access(true);
    rt.frame(0.0);
    let tab = |rt: &mut Runtime<()>, shift: bool| {
        rt.key(KeyEvent { key: Key::Tab, mods: Mods { shift, ..Mods::default() } });
        rt.settle_all();
        rt.frame(0.0);
    };
    for _ in 0..20 {
        tab(&mut rt, false);
    }
    assert_eq!(top(&rt, "box19") + 40.0, 200.0, "the last box, the content's end, ends flush with the viewport");
    for _ in 0..19 {
        tab(&mut rt, true);
    }
    assert_eq!(top(&rt, "box0"), 4.0, "the first box, not only its editor, starts the margin below the top");
}
