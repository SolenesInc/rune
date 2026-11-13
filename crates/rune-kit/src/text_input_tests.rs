use rune::{Cx, El, Env, Ime, Runtime, TextEdit, col};
use super::{TextArea, TextField, TextInput};
use crate::Kit;

#[derive(Default)]
struct S {
    name: TextEdit,
    notes: TextEdit,
    title: TextEdit,
    focused: Vec<String>,
}

fn view(s: &S, _: &mut Cx<S>) -> El {
    let k = Kit::neutral(true);
    col()
        .gap(8.0)
        .padding(20.0)
        .child(TextInput::new(&s.name).key("name").on_edit(|s: &mut S| &mut s.name).build(&k))
        .child(TextArea::new(&s.notes).key("notes").on_edit(|s: &mut S| &mut s.notes).build(&k))
        .child(TextField::new("Title", &s.title).key("title").on_edit(|s: &mut S| &mut s.title).build(&k))
}

#[test]
fn each_field_s_editor_takes_the_key_the_app_gives_it() {
    let mut rt = Runtime::new(S::default(), view, (600.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    for key in ["name", "notes", "title"] {
        let (x, y) = rt.find(key).unwrap_or_else(|| panic!("no editor keyed {key}"));
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
        rt.ime(Ime::Commit("a".into()));
        rt.frame(0.0);
        let focused = rt.elements().into_iter().find(|e| e.focused).and_then(|e| e.key);
        rt.state.focused.push(focused.unwrap_or_default());
    }
    assert_eq!(rt.state.focused, ["name", "notes", "title"]);
    assert_eq!((rt.state.name.text.as_str(), rt.state.notes.text.as_str(), rt.state.title.text.as_str()), ("a", "a", "a"));
}
