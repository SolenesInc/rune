use super::*;
use crate::{Cx, El, Env, Ime, PhysicalInput, PhysicalKeyCode, Runtime, TextEdit, TextStyle, text_input};

#[derive(Default)]
struct Field { edit: TextEdit, opened: bool }

fn field(s: &Field, _: &mut Cx<Field>) -> El {
    text_input(&s.edit, TextStyle::new(14.0, 400.0)).key("reply").size(200.0, 40.0)
        .on_edit(|s: &mut Field| &mut s.edit)
}

#[test]
fn dvorak_shift_command_o_reaches_the_app_with_a_field_focused() {
    let mut rt = Runtime::new(Field::default(), field, (200.0, 40.0), Env::new(true, 1.0));
    rt.app_mut().on_key = Some(|s, ev, _| {
        if ev.key == Key::Command('o') && ev.mods.shift {
            s.opened = true;
            return true;
        }
        false
    });
    rt.frame(0.0);
    let (x, y) = rt.find("reply").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    // Dvorak O lives on ANSI S; the logical chord must win over that position.
    let command = if cfg!(target_os = "macos") { ModifiersState::SUPER } else { ModifiersState::CONTROL };
    let logical = map_key(&WKey::Character("O".into()), command | ModifiersState::SHIFT).unwrap();
    rt.physical_with_fallback(PhysicalInput::Key {
        key: PhysicalKeyCode::KeyS, mods: logical.mods, pressed: true, repeat: false,
    }, Some(logical));
    rt.frame(0.0);
    assert!(rt.state().opened);
    assert!(rt.state().edit.text.is_empty(), "the chord did not type into the field");
}

#[test]
fn dvorak_paste_uses_the_logical_character_not_the_physical_period() {
    let mut rt = Runtime::new(Field::default(), field, (200.0, 40.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let (x, y) = rt.find("reply").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    // macOS supplies charactersIgnoringModifiers="v" at ANSI Period (47)
    // on Dvorak, including a synthesized Command-V from a dictation app.
    let command = if cfg!(target_os = "macos") { ModifiersState::SUPER } else { ModifiersState::CONTROL };
    let logical = map_key(&WKey::Character("v".into()), command).unwrap();
    assert_eq!(logical.key, Key::Command('v'));
    rt.physical_with_fallback(PhysicalInput::Key {
        key: PhysicalKeyCode::Period, mods: logical.mods, pressed: true, repeat: false,
    }, Some(logical));
    rt.frame(0.0);
    let target = rt.paste_request.take().expect("Dvorak Command-V requests the clipboard");
    rt.paste_result(target, crate::ClipboardContent::new("Dictated reply"));
    rt.frame(0.0);
    assert_eq!(rt.state().edit.text, "Dictated reply");
}

fn focused_field(bound: bool) -> Runtime<Field> {
    let mut rt = Runtime::new(Field::default(), field, (200.0, 40.0), Env::new(true, 1.0));
    if bound {
        rt.app_mut().on_key = Some(|s, ev, tx| {
            s.opened |= crate::Chord::parse("alt+e").unwrap().matches_tx(ev, tx);
            s.opened
        });
    }
    rt.frame(0.0);
    let (x, y) = rt.find("reply").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    let _synced_by_the_window = std::mem::take(&mut rt.ime_reset);
    rt
}

const OPTION: crate::Mods = crate::Mods { alt: true, cmd: false, ctrl: false, shift: false, logo: false };

#[test]
fn an_option_dead_key_runs_the_shortcut_on_its_key_and_types_nothing() {
    let mut rt = focused_field(true);
    let accent = map_key(&WKey::Dead(Some('´')), ModifiersState::ALT).unwrap();
    rt.physical_then(PhysicalInput::Key { key: PhysicalKeyCode::KeyE, mods: accent.mods, pressed: true, repeat: false },
        Some(accent), Some('e'), None);
    rt.frame(0.0);
    assert!(rt.state().opened, "⌥E's dead key ran the alt+e binding");
    assert!(rt.state().edit.text.is_empty(), "the accent did not type into the field");
}

#[test]
fn a_dead_key_that_opens_a_composition_runs_its_shortcut_first_and_drops_the_accent() {
    let mut rt = focused_field(true);
    rt.dead_key(PhysicalKeyCode::KeyE, OPTION, Some('e'), Ime::Preedit("´".into(), Some((0, 2))));
    rt.frame(0.0);
    assert!(rt.state().opened, "the binding heard the press the input method composed");
    assert!(rt.state().edit.preedit().is_none() && rt.state().edit.text.is_empty(), "the field shows no accent");
    assert!(rt.ime_reset, "the input method drops its accent too");
}

#[test]
fn a_dead_key_no_handler_takes_composes_its_accent_into_the_next_character() {
    let mut rt = focused_field(false);
    rt.dead_key(PhysicalKeyCode::KeyE, OPTION, Some('e'), Ime::Preedit("´".into(), Some((0, 2))));
    rt.frame(0.0);
    assert_eq!(rt.state().edit.preedit().map(|(s, _)| s), Some("´"), "the field shows the accent composing");
    assert!(!rt.ime_reset);
    rt.ime(Ime::Commit("é".into()));
    rt.frame(0.0);
    assert_eq!(rt.state().edit.text, "é");
}
