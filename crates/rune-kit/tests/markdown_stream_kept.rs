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
