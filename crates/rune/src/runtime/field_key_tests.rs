//! `AppSpec::on_field_key`: an app's shortcuts reach it from inside a focused text field.
use super::*;
use crate::{Cx, El, Env, Key, KeyEvent, Mods, TextEdit, TextStyle, col, el, text_input};

#[derive(Default)]
struct Shortcuts {
    field: TextEdit,
    log: Vec<String>,
}

fn view(s: &Shortcuts, _: &mut Cx<Shortcuts>) -> El {
    col()
        .size(300.0, 100.0)
        .child(el().key("pad").size(300.0, 10.0).focusable(true))
        .child(
            text_input(&s.field, TextStyle::new(14.0, 400.0))
                .key("field")
                .size(200.0, 30.0)
                .on_edit(|s: &mut Shortcuts| &mut s.field)
                .on_input_key(|s: &mut Shortcuts, ev: &KeyEvent, _: &mut Tx| {
                    s.log.push(format!("field {:?}", ev.key));
                    false
                }),
        )
}

/// ⌘Z (Ctrl+Z off macOS) is the app's shortcut; anything else it leaves.
fn shortcut(s: &mut Shortcuts, ev: &KeyEvent, tx: &mut Tx) -> bool {
    s.log.push(format!("app {:?} in {:?} field {}", ev.key, tx.focused(), tx.in_text_field()));
    ev.is_command('z')
}

fn runtime() -> Runtime<Shortcuts> {
    let spec = crate::AppSpec::new("shortcuts", (300.0, 100.0), Shortcuts::default, view);
    let mut rt = crate::App::new(spec).on_field_key(shortcut).headless_runtime((300.0, 100.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn press(rt: &mut Runtime<Shortcuts>, key: Key, mods: Mods) {
    rt.key(KeyEvent { key, mods });
    rt.frame(0.0);
}

fn command() -> Mods {
    if cfg!(target_os = "macos") { Mods { cmd: true, ..Mods::default() } } else { Mods { ctrl: true, ..Mods::default() } }
}

#[test]
fn a_focused_field_s_keys_reach_the_app_first_and_what_it_leaves_reaches_the_field() {
    let mut rt = runtime();
    rt.update(|_, tx| tx.focus("field", false));
    rt.frame(0.0);
    press(&mut rt, Key::Text("a".into()), Mods::default());
    press(&mut rt, Key::Command('z'), command());
    assert_eq!(
        rt.state().log,
        [r#"app Text("a") in Some("field") field true"#, r#"field Text("a")"#, r#"app Command('z') in Some("field") field true"#],
        "the app hears each key first; the field never hears the one the app took"
    );
    assert_eq!(rt.state().field.text, "a", "the field typed what the app left");
}

#[test]
fn keys_with_no_text_field_focused_never_reach_the_field_hook() {
    let mut rt = runtime();
    press(&mut rt, Key::Command('z'), command());
    rt.update(|_, tx| tx.focus("pad", false));
    rt.frame(0.0);
    press(&mut rt, Key::Command('z'), command());
    assert!(rt.state().log.is_empty(), "{:?}", rt.state().log);
}
