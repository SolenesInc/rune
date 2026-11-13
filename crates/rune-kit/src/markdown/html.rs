//! The raw HTML a document may use inline, read into text and styles: `<br>`, `<b>`,
//! `<strong>`, `<i>`, `<em>`, `<code>`, `<kbd>`, `<s>` and `<del>`. Any other tag stays out of
//! the text, as a browser's sanitizer drops it.

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
