use super::*;
use crate::{Key, KeyEvent, Mods, TextEdit, TextStyle, col, text_input};

struct Sheet {
    open: bool,
    search: TextEdit,
    on_itself: bool,
}

/// An opener and, while open, a modal sheet whose first control is a text field.
fn view(s: &Sheet, _: &mut Cx<Sheet>) -> El {
    let opener = el()
        .key("opener")
        .size(30.0, 30.0)
        .focusable(true)
        .role("button")
        .on_press(|s: &mut Sheet| s.open = true);
    let sheet = s.open.then(|| {
        let sheet = col()
            .key("sheet")
            .absolute(100.0, 0.0)
            .size(150.0, 100.0)
            .modal(true, |s: &mut Sheet| s.open = false)
            .child(
                text_input(&s.search, TextStyle::new(14.0, 400.0))
                    .key("search")
                    .size(120.0, 30.0)
                    .on_edit(|s: &mut Sheet| &mut s.search),
            )
            .child(
                el().key("close")
                    .size(30.0, 30.0)
                    .focusable(true)
                    .role("button"),
            );
        if s.on_itself {
            sheet.autofocus()
        } else {
            sheet
        }
    });
    col()
        .key("root")
        .size(300.0, 300.0)
        .child(opener)
        .children(sheet)
}

fn opened(on_itself: bool) -> Runtime<Sheet> {
    let sheet = Sheet {
        open: false,
        search: TextEdit::new(""),
        on_itself,
    };
    let mut rt = Runtime::new(sheet, view, (300.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let (x, y) = rt.find("opener").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    assert!(rt.state().open);
    rt
}

#[test]
fn a_modal_marked_autofocus_opens_focused_on_itself_and_tab_enters_its_field() {
    let mut rt = opened(true);
    assert_eq!(
        rt.focused_key().as_deref(),
        Some("sheet"),
        "the modal holds focus, not its field"
    );
    assert!(!rt.state().search.focused, "no caret in the field");
    rt.key(KeyEvent {
        key: Key::Tab,
        mods: Mods::default(),
    });
    rt.frame(0.0);
    assert_eq!(
        rt.focused_key().as_deref(),
        Some("search"),
        "Tab enters at the first control"
    );
}

#[test]
fn a_modal_without_autofocus_opens_on_its_field() {
    let rt = opened(false);
    assert_eq!(rt.focused_key().as_deref(), Some("search"));
}
