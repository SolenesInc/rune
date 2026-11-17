use super::*;

#[derive(Default)]
struct Form {
    edit: crate::TextEdit,
    polls: Vec<Option<String>>,
    blurs: usize,
}

fn view(s: &Form, _: &mut Cx<Form>) -> El {
    col()
        .size(300.0, 200.0)
        .child(el().key("heading").size(200.0, 30.0))
        .child(
            crate::text_input(&s.edit, crate::TextStyle::new(14.0, 400.0))
                .key("field")
                .size(200.0, 40.0)
                .on_edit(|s: &mut Form| &mut s.edit)
                .on_blur(|s: &mut Form, _| s.blurs += 1),
        )
        .child(el().key("next").size(200.0, 30.0).focusable(true))
}

fn form() -> Runtime<Form> {
    let mut rt = Runtime::new(Form::default(), view, (300.0, 200.0), Env::new(true, 1.0));
    rt.app_mut().poll = Some(|s: &mut Form, _, tx: &mut Tx| {
        s.polls.push(tx.focused().map(str::to_owned));
        false
    });
    rt.window_focus(true);
    rt.frame(0.0);
    rt
}

fn focused(rt: &Runtime<Form>) -> Option<&str> {
    rt.focus.path().map(|p| p.rsplit('/').next().unwrap_or(p))
}

fn click(rt: &mut Runtime<Form>, key: &str) {
    let (x, y) = rt.find(key).unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
}

#[test]
fn a_press_that_takes_focus_off_a_field_blurs_it_in_the_frame_it_lands_though_poll_never_sees_it_gone() {
    let mut rt = form();
    click(&mut rt, "field");
    rt.frame(0.0);
    rt.key(KeyEvent { key: crate::Key::Text("typed".into()), mods: Default::default() });
    rt.frame(0.0);
    assert_eq!(focused(&rt), Some("field"));
    click(&mut rt, "heading");
    rt.frame(0.0);
    assert_ne!(focused(&rt), Some("field"));
    assert_eq!(rt.state().blurs, 1);
    assert!(
        rt.state().polls.last().unwrap().as_deref().is_some_and(|f| f.ends_with("field")),
        "poll runs before the frame's input, so it alone would not see the focus leave"
    );
    click(&mut rt, "field");
    rt.frame(0.0);
    assert_eq!(rt.state().blurs, 1, "taking focus is not a blur");
}

#[test]
fn tab_and_the_window_losing_focus_each_blur_the_focused_field() {
    let mut rt = form();
    click(&mut rt, "field");
    rt.frame(0.0);
    rt.key(KeyEvent { key: crate::Key::Tab, mods: Default::default() });
    rt.frame(0.0);
    assert_eq!(focused(&rt), Some("next"));
    assert_eq!(rt.state().blurs, 1);

    click(&mut rt, "field");
    rt.frame(0.0);
    rt.window_focus(false);
    rt.frame(0.0);
    assert_eq!(rt.state().blurs, 2, "the window going inactive blurs it, as a page's input");
}
