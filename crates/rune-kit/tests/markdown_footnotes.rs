//! Footnotes in a [`Markdown`]: a mark (`[^label]`) draws as its number in superscript, numbered
//! by where each footnote is first referred to, and the definitions (`[^label]: text`) stand
//! as a numbered list under a rule at the end of the document, wherever the source has them.
#![cfg(feature = "markdown")]

use rune::{Cx, El, Env, Runtime, col};
use rune_kit::markdown::{Block, parse};
use rune_kit::{Kit, Markdown};

fn para(b: &Block) -> &str {
    match b {
        Block::Para(i) => &i.text,
        other => panic!("not a paragraph: {other:?}"),
    }
}

#[test]
fn marks_are_numbered_by_first_reference_and_definitions_stand_at_the_end() {
    let blocks = parse("First[^b] and second[^a], first again[^b].\n\n[^a]: About a.\n\n[^b]: About b.\n\nLast paragraph.");
    assert_eq!(para(&blocks[0]), "First¹ and second², first again¹.");
    assert_eq!(para(&blocks[1]), "Last paragraph.");
    assert_eq!(blocks[2], Block::Rule);
    let Block::List { start: Some(1), items } = &blocks[3] else { panic!("the definitions are a numbered list: {:?}", blocks[3]) };
    assert_eq!(items.iter().map(|i| para(&i[0])).collect::<Vec<_>>(), ["About b.", "About a."], "in the order of their marks");
    assert_eq!(blocks.len(), 4);
}

#[test]
fn a_mark_without_a_definition_is_text_and_a_definition_without_a_mark_comes_last() {
    assert_eq!(para(&parse("See [^missing] here.")[0]), "See [^missing] here.");
    let blocks = parse("[^unused]: Nobody points here.\n\nText[^1].\n\n[^1]: Pointed at.");
    let Some(Block::List { items, .. }) = blocks.last() else { panic!("{blocks:?}") };
    assert_eq!(items.iter().map(|i| para(&i[0])).collect::<Vec<_>>(), ["Pointed at.", "Nobody points here."]);
    let marks = (1..=12).map(|i| format!("w[^{i}]")).collect::<Vec<_>>().join(" ");
    let notes: String = (1..=12).map(|i| format!("\n\n[^{i}]: Note {i}.")).collect();
    assert_eq!(para(&parse(&format!("{marks}{notes}"))[0]).split(' ').next_back(), Some("w¹²"));
}

const NOTE: &str = "Intro[^code] and a list:\n\n- [x] done\n- [ ] todo\n\n[^code]: Run this:\n\n    ```rust\n    fn in_note() {}\n    ```\n\n    - [ ] a task in the note\n\n```python\ndef in_doc(): pass\n```\n\nEnd[^two].\n\n[^two]: Second note.\n";

fn view(src: &String, _: &mut Cx<String>) -> El {
    col().w_full().padding(10.0).child(Markdown::new(src.clone()).streaming(true).build(&Kit::neutral(true)))
}

fn drawn(rt: &Runtime<String>) -> Vec<String> {
    rt.elements().into_iter().map(|e| format!("{} {} {:?} {:?} {:?} {:?}", e.path, e.role, e.text, e.rect, e.fill, e.text_color)).collect()
}

/// A footnote's code and tasks are drawn after the document's own, where the note stands:
/// each keeps its own language and its own box, while the reply streams in and whole.
#[test]
fn a_stream_with_footnotes_shows_what_the_same_text_shows_built_fresh() {
    let mut streamed = Runtime::new(String::new(), view, (480.0, 2000.0), Env::new(true, 2.0));
    streamed.frame(0.0);
    let mut at = 0;
    while at < NOTE.len() {
        let end = (at + 5).min(NOTE.len());
        streamed.update(|s, tx| {
            tx.instant();
            s.push_str(&NOTE[at..end]);
        });
        streamed.frame(1.0 / 60.0);
        let mut fresh = Runtime::new(NOTE[..end].to_owned(), view, (480.0, 2000.0), Env::new(true, 2.0));
        fresh.frame(0.0);
        assert_eq!(drawn(&streamed), drawn(&fresh), "after {:?}", &NOTE[..end]);
        at = end;
    }
    let texts: Vec<String> = streamed.elements().into_iter().filter_map(|e| e.text).collect();
    let at = |needle: &str| texts.iter().position(|t| t.contains(needle)).unwrap_or_else(|| panic!("{needle:?} in {texts:?}"));
    assert!(at("def in_doc") < at("Run this:") && at("Run this:") < at("fn in_note") && at("fn in_note") < at("Second note."), "{texts:?}");
    assert!(texts.iter().any(|t| t.starts_with("Intro¹")) && texts.iter().any(|t| t.starts_with("End²")), "{texts:?}");
}
