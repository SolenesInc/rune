//! A [`Markdown`] still streaming keeps the elements of the blocks that can't change and
//! builds its newest one. What it shows at every length is what the same text shows built
//! from nothing: no block keeps a stale margin, checkbox, colour or place.
#![cfg(feature = "markdown")]

use rune::{Cx, El, Env, Runtime, col};
use rune_kit::{Kit, Markdown};

const REPLY: &str = "# A reply\n\nFirst paragraph with **bold**, `code` and a [link](https://example.com).\n\n## Steps\n\n1. one\n2. two\n   - [x] nested done\n   - [ ] nested todo\n\n```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n\n> A quote\n> over two lines.\n\n| File | Change |\n|---|---|\n| a.rs | x |\n| b.rs | y |\n\n- [ ] a task\n- [x] another\n\n```ts\nconst seen = { [key]: 1 };\n```\n\n---\n\nLast words, with *emphasis* that ends.\n";

fn view(src: &String, _: &mut Cx<String>) -> El {
    col().w_full().padding(10.0).child(Markdown::new(src.clone()).streaming(true).selectable(true).build(&Kit::neutral(true)))
}

/// Every element drawn: where it is in the tree, what it says, where it sits and its colours.
fn drawn(rt: &Runtime<String>) -> Vec<String> {
    rt.elements().into_iter().map(|e| format!("{} {} {:?} {:?} {:?} {:?}", e.path, e.role, e.text, e.rect, e.fill, e.text_color)).collect()
}

#[test]
fn a_stream_shows_at_every_length_what_the_same_text_shows_built_fresh() {
    let mut streamed = Runtime::new(String::new(), view, (480.0, 2000.0), Env::new(true, 2.0));
    streamed.frame(0.0);
    let mut at = 0;
    while at < REPLY.len() {
        let end = (at + 7).min(REPLY.len());
        streamed.update(|s, tx| {
            tx.instant();
            s.push_str(&REPLY[at..end]);
        });
        streamed.frame(1.0 / 60.0);
        at = end;
        let mut fresh = Runtime::new(REPLY[..end].to_owned(), view, (480.0, 2000.0), Env::new(true, 2.0));
        fresh.frame(0.0);
        assert_eq!(drawn(&streamed), drawn(&fresh), "after {:?}", &REPLY[..end]);
    }
}

/// A kept block's sideways scroll bar stays its own: the block built under it, which is the
/// only one built, takes a bar of its own and not the kept one's.
#[test]
fn a_block_built_under_a_kept_one_scrolls_under_its_own_bar() {
    fn view(src: &String, _: &mut Cx<String>) -> El {
        let k = Kit::neutral(true);
        let style = rune_kit::MarkdownStyle::from_kit(&k).code_scroll_bar(rune_kit::ScrollBarStyle::from_kit(&k).always());
        col().w_full().child(Markdown::new(src.clone()).streaming(true).scroll_wide(true).style(style).build(&k))
    }
    let wide = |name: &str| format!("let {name} = [{}];", (0..60).map(|i| i.to_string()).collect::<Vec<_>>().join(", "));
    let src = format!("```rust\n{}\n```\n\n```rust\n{}\n", wide("first"), wide("second"));
    let mut rt = Runtime::new(src, view, (480.0, 800.0), Env::new(true, 1.0));
    rt.frame(0.0);
    // The second block grows, so it alone is built again.
    rt.update(|s, tx| {
        tx.instant();
        s.push_str("more();\n");
    });
    rt.frame(1.0 / 60.0);
    rt.frame(1.0 / 60.0);
    let thumbs = |rt: &Runtime<String>| rt.elements().into_iter().filter(|e| e.key.as_deref() == Some("thumb")).map(|e| e.rect).collect::<Vec<_>>();
    let at_rest = thumbs(&rt);
    assert_eq!(at_rest.len(), 2, "a bar on each block: {at_rest:?}");
    let code = rt.elements().into_iter().find(|e| e.text.as_deref().is_some_and(|t| t.starts_with("let second"))).expect("code").rect;
    rt.wheel_xy(240.0, code[1] + 4.0, -200.0, 0.0);
    rt.frame(1.0 / 60.0);
    rt.frame(1.0 / 60.0);
    let scrolled = thumbs(&rt);
    assert_eq!(scrolled[0], at_rest[0], "the kept block's thumb stays at its start");
    assert!(scrolled[1][0] > at_rest[1][0], "the scrolled block's thumb travels: {:?} -> {:?}", at_rest[1], scrolled[1]);
}
