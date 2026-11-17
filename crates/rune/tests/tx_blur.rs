//! `Tx::blur` drops keyboard focus from a handler.
use rune::tok::TextStyle;
use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, TextEdit, col, text_input};

#[derive(Default)]
struct S {
    field: TextEdit,
    submits: u32,
    keys: u32,
}

fn view(s: &S, _: &mut Cx<S>) -> El {
    col().size(200.0, 100.0).child(
        text_input(&s.field, TextStyle::new(14.0, 400.0)).key("field").w(150.0)
            .on_edit(|s: &mut S| &mut s.field)
            .on_submit(|s: &mut S, tx| { s.submits += 1; tx.blur(); }),
    )
}

fn focused(rt: &Runtime<S>) -> bool {
    rt.elements().iter().any(|e| e.key.as_deref() == Some("field") && e.focused)
}

fn runtime() -> Runtime<S> {
    let mut s = S::default();
    s.field.focused = true;
    let mut rt = Runtime::new(s, view, (200.0, 100.0), Env::default());
    rt.app_mut().on_key = Some(|s, _, _| { s.keys += 1; true });
    rt.frame(0.0);
    assert!(focused(&rt), "the model asked for focus");
    rt
}

fn press(rt: &mut Runtime<S>, key: Key) {
    rt.key(KeyEvent { key, mods: Mods::default() });
    rt.frame(1.0 / 60.0);
}

#[test]
fn a_submit_handler_blurs_its_field() {
    let mut rt = runtime();
    press(&mut rt, Key::Enter);
    assert_eq!(rt.state().submits, 1);
    assert!(!focused(&rt), "nothing has focus");
    assert!(!rt.state().field.focused, "the model hears it through on_edit");
    rt.frame(1.0 / 60.0);
    assert!(!focused(&rt), "and focus stays dropped");
    press(&mut rt, Key::Text("z".into()));
    assert_eq!(rt.state().field.text.as_str(), "", "typing no longer reaches the field");
    assert_eq!(rt.state().keys, 1, "keys go to the window");
}

#[test]
fn an_update_blurs_and_asks_for_a_frame() {
    let mut rt = runtime();
    assert!(rt.update_if(|_, tx| { tx.blur(); false }), "a blur that drops focus is frame work");
    rt.frame(1.0 / 60.0);
    assert!(!focused(&rt));
    assert!(!rt.update_if(|_, tx| { tx.blur(); false }), "with nothing focused it's nothing");
}

#[derive(Default)]
struct Naming {
    name: Option<TextEdit>,
    named: Vec<String>,
}

fn naming_view(s: &Naming, _: &mut Cx<Naming>) -> El {
    col().size(200.0, 100.0).children(s.name.as_ref().map(|name| {
        text_input(name, TextStyle::new(14.0, 400.0)).key("name").w(150.0)
            .on_edit(|s: &mut Naming| s.name.as_mut().expect("the name field shows only while naming"))
            .on_submit(|s: &mut Naming, tx| {
                let name = s.name.take().expect("submitted from the field");
                s.named.push(name.text.as_str().to_owned());
                tx.blur();
            })
    }))
}

#[test]
fn a_handler_that_ends_a_field_and_blurs_never_reaches_its_edit() {
    let mut name = TextEdit::new("draft");
    name.focused = true;
    let mut rt = Runtime::new(Naming { name: Some(name), named: Vec::new() }, naming_view, (200.0, 100.0), Env::default());
    rt.frame(0.0);
    rt.key(KeyEvent { key: Key::Enter, mods: Mods::default() });
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.state().named, ["draft"]);
    assert!(rt.elements().iter().all(|e| !e.focused), "nothing has focus");
    rt.update(|s, _| {
        let mut name = TextEdit::new("");
        name.focused = true;
        s.name = Some(name);
    });
    rt.frame(2.0 / 60.0);
    assert!(rt.elements().iter().any(|e| e.key.as_deref() == Some("name") && e.focused), "naming again");
    rt.update(|s, tx| { s.name = None; tx.blur(); });
    rt.frame(3.0 / 60.0);
    assert!(rt.elements().iter().all(|e| !e.focused), "an update that ends the field and blurs drops focus");
}
