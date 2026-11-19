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
    assert_eq!(rt.state().closes, 1);
    assert!(rt.state().open);
    let focused = rt.elements().into_iter().find(|e| e.focused).and_then(|e| e.key);
    assert_eq!(focused.as_deref(), Some("field"));

    let (x, y) = rt.find("close").expect("the close button");
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state().closes, 2);
    assert!(!rt.state().open);
}

#[test]
fn a_tall_dialog_fills_the_window_up_to_its_height_and_carries_header_content() {
    assert_eq!(rect(&settled((1200.0, 900.0)), "settings-dialog")[3], 720.0);
    let short = settled((1200.0, 500.0));
    assert_eq!(short.elements().into_iter().find(|e| e.key.as_deref() == Some("settings-dialog")).unwrap().rect[3], 500.0 - 32.0);

    let (search, close) = (rect(&short, "search"), rect(&short, "close"));
    assert!(search[0] + search[2] <= close[0], "the search sits before the close button: {search:?} {close:?}");
}

fn led(_: &S, _: &mut Cx<S>) -> El {
    let k = Kit::neutral(true);
    let mut layer = Vec::new();
    Dialog::new("crew", "Manage crew")
        .description("Crew")
        .open(true)
        .lead(rune::text("CREW · Manage crew").key("kicker"))
        .on_close(|_: &mut S| {})
        .build(&k, &mut layer);
    root(rune::el().w_full().h_full(), layer)
}

#[test]
fn a_lead_takes_the_place_of_the_title_and_the_title_still_names_the_dialog() {
    let mut rt = Runtime::new(S { open: true, ..S::default() }, led, (1200.0, 900.0), Env::new(true, 1.0));
    rt.frame(1.0 / 60.0);
    let elements = rt.elements();
    let dialog = elements.iter().find(|e| e.key.as_deref() == Some("crew-dialog")).expect("the dialog");
    assert_eq!(dialog.label.as_deref(), Some("Manage crew"));
    assert!(elements.iter().any(|e| e.key.as_deref() == Some("kicker")), "the lead shows");
    assert!(!elements.iter().any(|e| e.text.as_deref() == Some("Manage crew")), "the kit's title is not drawn");
    let (lead, close) = (rect(&rt, "lead"), rect(&rt, "close"));
    assert!(lead[0] + lead[2] <= close[0], "the lead sits before the close button: {lead:?} {close:?}");
}

fn backed(s: &S, _: &mut Cx<S>) -> El {
    let k = Kit::neutral(true);
    let mut layer = Vec::new();
    Dialog::new("plain", "Plain").open(s.open).on_close(|_: &mut S| {}).build(&k, &mut layer);
    Dialog::new("whats-new", "What's new")
        .open(s.open)
        .backdrop(rune::Color::BLACK.alpha(0.7), 4.0)
        .on_close(|_: &mut S| {})
        .build(&k, &mut layer);
    root(rune::el().w_full().h_full(), layer)
}

#[test]
fn a_dialog_draws_its_own_backdrop_in_place_of_the_kits_scrim() {
    let mut rt = Runtime::new(S { open: true, ..S::default() }, backed, (1200.0, 900.0), Env::new(true, 1.0));
    for _ in 0..60 {
        rt.frame(1.0 / 60.0);
    }
    let scrim = |key: &str| rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("no {key}"));
    let (plain, own) = (scrim("plain-scrim"), scrim("whats-new-scrim"));
    assert_eq!((plain.fill, plain.backdrop_blur), (Kit::neutral(true).c.scrim, 0.0));
    assert_eq!((own.fill, own.backdrop_blur), (rune::Color::BLACK.alpha(0.7), 4.0));
}
