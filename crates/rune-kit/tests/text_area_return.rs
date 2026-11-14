//! `TextArea` and `El::return_breaks_line`: Return breaks the line, as in a `<textarea>`, and
//! Cmd/Ctrl+Return submits.
use rune::{Cx, El, Env, Ime, Key, KeyEvent, Mods, Runtime, TextEdit};
use rune_kit::{Kit, TextArea};

struct S { area: TextEdit, sent: usize }

fn view(s: &S, _: &mut Cx<S>) -> El {
    rune::col().w_full().h_full().padding(20.0).child(
        TextArea::new(&s.area)
            .width(None)
            .on_edit(|s: &mut S| &mut s.area)
            .on_submit(|s: &mut S, _| s.sent += 1)
            .build(&Kit::neutral(true)),
    )
}

fn runtime() -> Runtime<S> {
    let mut area = TextEdit::new("");
    area.focused = true;
    let mut rt = Runtime::new(S { area, sent: 0 }, view, (400.0, 400.0), Env::new(true, 2.0));
    rt.frame(1.0 / 60.0);
    rt
}

fn key(rt: &mut Runtime<S>, mods: Mods) {
    rt.key(KeyEvent { key: Key::Enter, mods });
    rt.frame(1.0 / 60.0);
}

fn type_text(rt: &mut Runtime<S>, text: &str) {
    rt.ime(Ime::Commit(text.into()));
    rt.frame(1.0 / 60.0);
}

#[test]
fn return_in_a_text_area_breaks_the_line_and_cmd_or_ctrl_return_submits() {
    let mut rt = runtime();
    type_text(&mut rt, "first");
    key(&mut rt, Mods::default());
    type_text(&mut rt, "second");
    assert_eq!(rt.state.area.text, "first\nsecond");
    assert_eq!(rt.state.sent, 0, "plain Return does not submit");

    key(&mut rt, Mods { shift: true, ..Mods::default() });
    type_text(&mut rt, "third");
    assert_eq!(rt.state.area.text, "first\nsecond\nthird", "Shift+Return still breaks the line");

    key(&mut rt, Mods { cmd: true, ..Mods::default() });
    assert_eq!(rt.state.sent, 1);
    key(&mut rt, Mods { ctrl: true, ..Mods::default() });
    assert_eq!(rt.state.sent, 2);
    assert_eq!(rt.state.area.text, "first\nsecond\nthird", "submitting adds no line");
}
