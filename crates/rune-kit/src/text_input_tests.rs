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

const INK: rune::Color = rune::Color { r: 0.2, g: 0.6, b: 0.3, a: 1.0 };

fn disabled(s: &S, _: &mut Cx<S>) -> El {
    let k = Kit::neutral(true);
    col()
        .padding(20.0)
        .child(TextInput::new(&s.name).key("dimmed").disabled(true).build(&k).key("dimmed-box"))
        .child(TextInput::new(&s.name).key("inked").disabled(true).disabled_ink(INK).build(&k).key("inked-box"))
        .child(TextArea::new(&s.name).key("area").disabled(true).disabled_ink(INK).build(&k).key("area-box"))
}

/// The alpha the frame composites the box keyed `key` at: every layer drawn around it.
fn strength(rt: &Runtime<S>, key: &str) -> f32 {
    let e = rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("no {key}"));
    let (x, y) = (e.rect[0] + e.rect[2] / 2.0, e.rect[1] + e.rect[3] / 2.0);
    rt.instances.iter()
        .filter(|i| i.params[1] == rune::render::KIND_LAYER)
        .filter(|i| (i.rect[0]..i.rect[0] + i.rect[2]).contains(&x) && (i.rect[1]..i.rect[1] + i.rect[3]).contains(&y))
        .map(|i| i.fill[3])
        .product()
}

#[test]
fn a_disabled_field_given_an_ink_draws_its_value_in_it_at_full_strength() {
    let s = S { name: TextEdit::new("Value"), ..S::default() };
    let mut rt = Runtime::new(s, disabled, (600.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    assert_eq!(strength(&rt, "dimmed-box"), 0.5, "without an ink the kit still dims the whole field");
    let all = rt.elements();
    for (field, frame) in [("inked", "inked-box"), ("area", "area-box")] {
        let ink = all.iter().find(|e| e.key.as_deref() == Some(field)).and_then(|e| e.text_color);
        assert_eq!((ink, strength(&rt, frame)), (Some(INK), 1.0), "{field}");
    }
}

fn faded(s: &S, _: &mut Cx<S>) -> El {
    let k = Kit::neutral(true);
    col()
        .padding(20.0)
        .child(TextInput::new(&s.name).key("faded").fade_overflow().width(Some(120.0)).build(&k))
        .child(TextInput::new(&s.name).key("cut").width(Some(120.0)).build(&k))
}

#[test]
fn a_field_that_fades_its_overflow_fades_a_long_value_and_another_cuts_it() {
    let s = S { name: TextEdit::new("A value far too long for the field it was typed into"), ..S::default() };
    let mut rt = Runtime::new(s, faded, (600.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let all = rt.elements();
    let top = |key: &str| all.iter().find(|e| e.key.as_deref() == Some(key)).unwrap().rect[1];
    let right_fade = |key: &str| {
        let (y0, y1) = (top(key), top(key) + 28.0);
        rt.instances.iter()
            .filter(|i| i.params[1] == rune::render::KIND_GLYPH && i.rect[1] >= y0 && i.rect[1] < y1)
            .map(|i| rune::render::unpack_fade(i.fade).0[2])
            .fold(0.0, f32::max)
    };
    assert!(right_fade("faded") > 0.0 && right_fade("cut") == 0.0, "faded {}, cut {}", right_fade("faded"), right_fade("cut"));
}
