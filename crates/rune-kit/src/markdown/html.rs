//! The raw HTML a document may use, read into text, styles and blocks: inline `<br>`, `<b>`,
//! `<strong>`, `<i>`, `<em>`, `<code>`, `<kbd>`, `<s>` and `<del>`, and `<details>` blocks
//! with their `<summary>`. Any other tag stays out of the text, as a browser's sanitizer
//! drops it.

use super::Style;
use super::parser::{Builder, close_run, open, push_text};
use std::ops::Range;

enum Inline {
    Break,
    Open(Style),
    Close(Style),
}

fn read(html: &str) -> Option<Inline> {
    let inner = html.trim().strip_prefix('<')?.strip_suffix('>')?;
    let (closing, inner) = match inner.strip_prefix('/') {
        Some(rest) => (true, rest),
        None => (false, inner),
    };
    let name: String = inner.chars().take_while(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase();
    let style = match name.as_str() {
        "br" => return (!closing).then_some(Inline::Break),
        "b" | "strong" => Style::Strong,
        "i" | "em" => Style::Em,
        "code" | "kbd" => Style::Code,
        "s" | "del" => Style::Strike,
        _ => return None,
    };
    Some(if closing { Inline::Close(style) } else { Inline::Open(style) })
}

/// Reads inline HTML `html`, at source bytes `range`, into the inline text `b` is building.
pub(super) fn inline(b: &mut Builder, html: &str, range: Range<usize>) {
    match read(html) {
        Some(Inline::Break) => push_text(b, "\n", range),
        Some(Inline::Open(style)) => open(b, style, None),
        Some(Inline::Close(style)) => close_run(b, style),
        None => {}
    }
}

/// The tags in `html`, in order: where each one's `<` is, its lowercased name, whether it
/// closes, its attributes' text, and the text that follows it up to the next tag.
fn tags(html: &str) -> impl Iterator<Item = (usize, String, bool, &str, &str)> {
    html.split('<').skip(1).filter_map(move |piece| {
        let lt = piece.as_ptr() as usize - html.as_ptr() as usize - 1;
        let (tag, after) = piece.split_once('>')?;
        let (closing, tag) = match tag.strip_prefix('/') {
            Some(rest) => (true, rest),
            None => (false, tag),
        };
        let name: String = tag.chars().take_while(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase();
        Some((lt, name.clone(), closing, &tag[name.len()..], after))
    })
}

/// Reads the HTML block `html`, which starts at source byte `at`, for the `<details>` blocks it
/// opens, names or closes; the body between a `<details>` and its `</details>` is Markdown.
/// Each `<details>` is known by the source byte its tag starts at.
pub(super) fn block(b: &mut Builder, html: &str, at: usize) {
    let mut summary: Option<String> = None;
    for (lt, name, closing, attrs, after) in tags(html) {
        match (name.as_str(), closing) {
            ("details", false) => {
                let open = attrs.split(|c: char| c.is_whitespace() || c == '/').any(|a| a.eq_ignore_ascii_case("open") || a.to_ascii_lowercase().starts_with("open="));
                b.open_details(open, at + lt);
            }
            ("details", true) => b.close_details(),
            ("summary", false) => summary = Some(String::new()),
            ("summary", true) => {
                if let Some(text) = summary.take() {
                    b.summarize(text.split_whitespace().collect::<Vec<_>>().join(" ").as_str());
                }
                continue;
            }
            _ => {}
        }
        if let Some(text) = summary.as_mut() {
            text.push_str(after);
        }
    }
}
