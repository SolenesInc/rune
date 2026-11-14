//! A runtime's work doesn't depend on the app's state type: code written against `RuntimeCore`
//! drives the runtime of any state, and the typed `Runtime<S>` reads that state back.
use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, RuntimeCore, col, el};

#[derive(Default)]
struct Count {
    presses: u32,
    polls: u32,
    keys: u32,
}

#[derive(Default)]
struct Names(Vec<&'static str>);

fn count_view(_: &Count, _: &mut Cx<Count>) -> El {
    col().size(200.0, 100.0).child(el().key("button").size(50.0, 50.0).on_press(|s: &mut Count| s.presses += 1))
}

fn names_view(_: &Names, _: &mut Cx<Names>) -> El {
    col().size(200.0, 100.0).child(el().key("button").size(50.0, 50.0).on_press(|s: &mut Names| s.0.push("press")))
}

/// Presses the element keyed `button`, then types Escape, without knowing whose state it changes.
fn press_and_key(core: &mut RuntimeCore) {
    core.frame(0.0);
    let (x, y) = core.find("button").expect("the view keyed a button");
    core.pointer_down(x, y);
    core.pointer_up(x, y);
    core.key(KeyEvent { key: Key::Escape, mods: Mods::default() });
    core.frame(1.0 / 60.0);
}

#[test]
fn one_core_drives_runtimes_of_any_state_type() {
    let mut count = Runtime::new(Count::default(), count_view, (200.0, 100.0), Env::default());
    count.app_mut().poll = Some(|s, _, _| { s.polls += 1; false });
    count.app_mut().on_key = Some(|s, _, _| { s.keys += 1; true });
    let mut names = Runtime::new(Names::default(), names_view, (200.0, 100.0), Env::default());
    names.app_mut().on_key = Some(|s, ev, _| { s.0.push(if ev.key == Key::Escape { "escape" } else { "other" }); true });

    press_and_key(&mut count);
    press_and_key(&mut names);

    assert_eq!(count.state().presses, 1, "the press reached the Count handler");
    assert_eq!(count.state().keys, 1, "the unhandled key reached on_key");
    assert!(count.state().polls >= 2, "each frame polled, got {}", count.state().polls);
    assert_eq!(names.state().0, ["press", "escape"]);

    count.state_mut().presses = 10;
    count.update(|s, _| s.presses += 1);
    assert_eq!(count.state().presses, 11, "state_mut and update change the same state");
}
