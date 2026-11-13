use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, TextEdit};
use super::Dialog;
use crate::{Kit, TextInput, root};

#[derive(Default)]
struct S {
    open: bool,
    field: TextEdit,
    closes: usize,
}

fn view(s: &S, _: &mut Cx<S>) -> El {
    let k = Kit::neutral(true);
    let mut layer = Vec::new();
    Dialog::new("settings", "Settings")
        .open(s.open)
        .max_height(720.0)
        .width(960.0)
        .header(rune::el().key("search").size(200.0, 28.0))
        .child(TextInput::new(&s.field).key("field").on_edit(|s: &mut S| &mut s.field).build(&k))
        .on_close_tx(|s: &mut S, tx| {
            s.closes += 1;
            if s.closes > 1 {
                s.open = false;
            } else {
                tx.focus("field", false);
            }
        })
        .build(&k, &mut layer);
    root(rune::el().w_full().h_full(), layer)
}

fn settled(size: (f32, f32)) -> Runtime<S> {
    let mut rt = Runtime::new(S { open: true, ..S::default() }, view, size, Env::new(true, 1.0));
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
    rt
}

fn rect(rt: &Runtime<S>, key: &str) -> [f32; 4] {
    rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("no {key}")).rect
}

#[test]
fn a_close_handler_with_a_tx_can_keep_the_dialog_open_and_move_focus() {
    let mut rt = settled((1200.0, 900.0));
    rt.key(KeyEvent { key: Key::Escape, mods: Mods::default() });
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.closes, 1);
    assert!(rt.state.open);
    let focused = rt.elements().into_iter().find(|e| e.focused).and_then(|e| e.key);
    assert_eq!(focused.as_deref(), Some("field"));

    let (x, y) = rt.find("close").expect("the close button");
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state.closes, 2);
    assert!(!rt.state.open);
}

#[test]
fn a_tall_dialog_fills_the_window_up_to_its_height_and_carries_header_content() {
    assert_eq!(rect(&settled((1200.0, 900.0)), "settings-dialog")[3], 720.0);
    let short = settled((1200.0, 500.0));
    assert_eq!(short.elements().into_iter().find(|e| e.key.as_deref() == Some("settings-dialog")).unwrap().rect[3], 500.0 - 32.0);

    let (search, close) = (rect(&short, "search"), rect(&short, "close"));
    assert!(search[0] + search[2] <= close[0], "the search sits before the close button: {search:?} {close:?}");
}
