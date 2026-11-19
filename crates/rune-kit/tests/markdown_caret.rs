//! A [`Markdown`] still streaming, in a style with a caret ([`MarkdownStyle::caret`]), ends in
//! one: after the last word of its last text, moving as the reply grows and gone when it ends.
#![cfg(feature = "markdown")]

use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, col};
use rune_kit::{Kit, Markdown, MarkdownStyle};

/// What the caret adds to the text it follows.
const CARET: &str = "\u{a0}\u{a0}";

fn view((src, streaming): &(String, bool), _: &mut Cx<(String, bool)>) -> El {
    let k = Kit::neutral(true);
    let style = MarkdownStyle::from_kit(&k).caret(k.c.text_weak);
    col().w_full().padding(10.0).child(Markdown::new(src.clone()).streaming(*streaming).reveal(0.18).selectable(true).style(style).build(&k))
}

fn runtime(src: &str) -> Runtime<(String, bool)> {
    let mut rt = Runtime::new((src.to_owned(), true), view, (480.0, 2000.0), Env::new(true, 2.0));
    rt.frame(0.0);
    rt
}

/// The texts that end in the caret, without it.
fn careted(rt: &Runtime<(String, bool)>) -> Vec<String> {
    rt.elements().into_iter().filter_map(|e| e.text).filter_map(|t| t.strip_suffix(CARET).map(str::to_owned)).collect()
}

#[test]
fn the_caret_follows_the_last_word_as_a_reply_grows_and_goes_when_it_ends() {
    let mut rt = runtime("First paragraph.\n\nSecond **par");
    assert_eq!(careted(&rt), ["Second par"]);
    rt.update(|s, _| s.0.push_str("agraph** grows"));
    rt.frame(1.0 / 60.0);
    assert_eq!(careted(&rt), ["Second paragraph grows"]);
    rt.update(|s, _| s.0.push_str(".\n\n- one\n- two"));
    rt.frame(1.0 / 60.0);
    assert_eq!(careted(&rt), ["two"], "in a list it follows the last item");
    rt.update(|s, _| s.0.push_str("\n\n```rust\nlet a = 1;\nlet b = 2;"));
    rt.frame(1.0 / 60.0);
    assert_eq!(careted(&rt), ["let a = 1;\nlet b = 2;"], "in a code block, its last line");
    rt.update(|s, _| s.1 = false);
    rt.frame(1.0 / 60.0);
    assert_eq!(careted(&rt), [] as [&str; 0], "a finished reply has none");
}

#[test]
fn a_reply_that_ends_in_a_table_or_a_rule_has_no_caret() {
    for src in ["Intro.\n\n| a | b |\n|---|---|\n| 1 | 2 |", "Intro.\n\n---\n"] {
        assert_eq!(careted(&runtime(src)), [] as [&str; 0], "{src:?}");
    }
}

#[test]
fn the_caret_is_not_copied() {
    let mut rt = runtime("Copy **this** text");
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    let at = rt.elements().into_iter().find(|e| e.text.as_deref().is_some_and(|t| t.starts_with("Copy"))).unwrap().rect;
    let (x, y) = (at[0] + 20.0, at[1] + 5.0);
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    rt.frame(1.0 / 60.0);
    rt.pointer_move(x + 30.0, y);
    rt.frame(1.0 / 60.0);
    rt.pointer_up(x + 30.0, y);
    rt.frame(1.0 / 60.0);
    for c in ['a', 'c'] {
        rt.key(KeyEvent { key: Key::Command(c), mods: Mods { cmd: true, ..Mods::default() } });
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(clipboard.text().as_deref().map(String::as_str), Some("Copy this text"));
}
