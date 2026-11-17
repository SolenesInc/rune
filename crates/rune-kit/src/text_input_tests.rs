use rune::{Cx, El, Env, Ime, Runtime, TextEdit, col};
use super::{TextField, TextInput};
use crate::TextArea;
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
        rt.state_mut().focused.push(focused.unwrap_or_default());
    }
    assert_eq!(rt.state().focused, ["name", "notes", "title"]);
    assert_eq!((rt.state().name.text.as_str(), rt.state().notes.text.as_str(), rt.state().title.text.as_str()), ("a", "a", "a"));
}

fn reply(s: &S, _: &mut Cx<S>) -> El {
    let k = Kit::neutral(true);
    col()
        .child(TextInput::new(&s.name).key("sending").read_only(true).on_edit(|s: &mut S| &mut s.name).build(&k))
        .child(TextInput::new(&s.notes).key("typing").on_edit(|s: &mut S| &mut s.notes).build(&k))
}

#[test]
fn a_read_only_field_s_editor_reads_as_read_only_and_an_editable_one_does_not() {
    let mut rt = Runtime::new(S::default(), reply, (600.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let tree = rt.accessibility_json();
    fn read_only(n: &serde_json::Value, key: &str) -> Option<bool> {
        if n["key"] == key {
            return Some(n["states"].as_array().is_some_and(|s| s.iter().any(|s| s == "read-only")));
        }
        n["children"].as_array()?.iter().find_map(|c| read_only(c, key))
    }
    assert_eq!((read_only(&tree, "sending"), read_only(&tree, "typing")), (Some(true), Some(false)), "{tree}");
}

fn inset(s: &S, _: &mut Cx<S>) -> El {
    let k = Kit::neutral(true);
    col()
        .padding(20.0)
        .child(TextInput::new(&s.name).key("plain").plain().on_edit(|s: &mut S| &mut s.name).build(&k))
        .child(TextInput::new(&s.notes).key("boxed").on_edit(|s: &mut S| &mut s.notes).build(&k))
}

#[test]
fn a_plain_field_s_text_starts_two_points_in_and_a_boxed_field_s_eight() {
    let mut rt = Runtime::new(S::default(), inset, (600.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let tree = rt.accessibility_json();
    fn left(n: &serde_json::Value, key: &str) -> Option<f64> {
        if n["key"] == key {
            return n["bounds"][0].as_f64();
        }
        n["children"].as_array()?.iter().find_map(|c| left(c, key))
    }
    assert_eq!((left(&tree, "plain"), left(&tree, "boxed")), (Some(22.0), Some(28.0)), "{tree}");
}
