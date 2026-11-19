//! A long code block in a [`Markdown`] is laid out as texts of 32 lines each, so a block still
//! streaming in shapes its newest lines, not every line again. It still reads as one block: its
//! lines keep one pitch, it copies as its source, and what has arrived never fades in twice.
#![cfg(feature = "markdown")]

use rune::runtime::ElementInfo;
use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, col};
use rune_kit::{Kit, Markdown};

fn view(src: &String, _: &mut Cx<String>) -> El {
    col().w_full().padding(10.0).child(Markdown::new(src.clone()).streaming(true).reveal(0.18).selectable(true).build(&Kit::neutral(true)))
}

fn runtime(src: &str) -> Runtime<String> {
    let mut rt = Runtime::new(src.to_owned(), view, (480.0, 4000.0), Env::new(true, 2.0));
    rt.frame(0.0);
    rt
}

fn lines(n: std::ops::Range<usize>) -> String {
    n.map(|i| format!("let value_{i} = compute({i});\n")).collect()
}

/// The code block's texts, top to bottom.
fn code_texts(rt: &Runtime<String>) -> Vec<ElementInfo> {
    rt.elements().into_iter().filter(|e| e.text.as_ref().is_some_and(|t| t.contains("let value_"))).collect()
}

fn tick(rt: &mut Runtime<String>) { rt.frame(1.0 / 60.0); }

fn command(rt: &mut Runtime<String>, c: char) {
    rt.key(KeyEvent { key: Key::Command(c), mods: Mods { cmd: true, ..Mods::default() } });
    tick(rt);
}

#[test]
fn a_line_arriving_in_a_long_fence_changes_only_its_last_text() {
    let mut rt = runtime(&format!("```rust\n{}", lines(0..100)));
    let before = code_texts(&rt);
    assert!(before.len() > 1, "100 lines are more than one text");
    assert!(before.iter().all(|e| e.text.as_ref().unwrap().lines().count() <= 32), "no text holds more than 32 lines");
    let shaped = rt.text.shaped;
    rt.update(|s, _| s.push_str("let value_100 = compute(100);\n"));
    tick(&mut rt);
    let after = code_texts(&rt);
    assert_eq!(rt.text.shaped - shaped, 1, "one text is shaped for one new line");
    assert_eq!(after.len(), before.len());
    for (a, b) in after.iter().zip(&before).take(before.len() - 1) {
        assert_eq!((&a.text, a.rect), (&b.text, b.rect), "the texts above the new line stay as they were");
    }
}

#[test]
fn a_long_block_keeps_one_line_pitch_and_a_short_one_is_one_text() {
    let short = runtime(&format!("```rust\n{}```\n", lines(0..32)));
    let one = code_texts(&short);
    assert_eq!(one.len(), 1, "32 lines are one text, as before");
    let long = runtime(&format!("```rust\n{}```\n", lines(0..96)));
    let three = code_texts(&long);
    assert_eq!(three.len(), 3);
    for (k, e) in three.iter().enumerate() {
        assert_eq!(e.rect[3], one[0].rect[3], "each text of 32 lines is as tall as a block of 32");
        assert_eq!(e.rect[1], three[0].rect[1] + k as f32 * one[0].rect[3], "text {k} starts where the one above ends");
        assert_eq!(e.rect[0], three[0].rect[0]);
    }
}

#[test]
fn a_long_block_copies_as_its_source() {
    let code = lines(0..100);
    let mut rt = runtime(&format!("Intro.\n\n```rust\n{code}```\n"));
    let clipboard = rune::clipboard::memory();
    rt.use_clipboard(&clipboard);
    let first = code_texts(&rt)[0].rect;
    let (x, y) = (first[0] + 20.0, first[1] + 5.0);
    rt.pointer_move(x, y);
    rt.pointer_down(x, y);
    tick(&mut rt);
    rt.pointer_move(x + 30.0, y);
    tick(&mut rt);
    rt.pointer_up(x + 30.0, y);
    tick(&mut rt);
    command(&mut rt, 'a');
    command(&mut rt, 'c');
    assert_eq!(clipboard.text().as_deref().map(String::as_str), Some(format!("Intro.\n\n{}", code.trim_end()).as_str()));
}

#[test]
fn lines_past_the_first_text_fade_in_and_the_ones_above_do_not_fade_again() {
    let mut rt = runtime(&format!("```rust\n{}", lines(0..30)));
    // Long enough for anything still fading to finish.
    for _ in 0..60 { tick(&mut rt); }
    assert!(code_texts(&rt).iter().all(|e| !e.revealing));
    rt.update(|s, _| s.push_str(&lines(30..36)));
    tick(&mut rt);
    let texts = code_texts(&rt);
    assert_eq!(texts.len(), 2);
    assert!(texts[1].revealing, "the lines that start a new text fade in as any new lines do");
    for _ in 0..60 { tick(&mut rt); }
    let texts = code_texts(&rt);
    assert!(texts.iter().all(|e| !e.revealing));
    assert_eq!(texts[0].text.as_ref().unwrap().lines().count(), 32);
}
