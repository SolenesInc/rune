//! Typographic punctuation and emoji shortcodes for prose ([`super::Markdown::smart_punctuation`]),
//! as attn's Tauri reader sets them: `...` as `…`, `---` as `—`, `--` between digits as `–`,
//! straight quotes as curly ones, and 32 shortcodes as their emoji.

use std::borrow::Cow;
use std::ops::Range;

const EMOJI: [(&str, &str); 32] = [
    ("smile", "😄"),
    ("heart", "❤️"),
    ("thumbsup", "👍"),
    ("thumbsdown", "👎"),
    ("fire", "🔥"),
    ("star", "⭐"),
    ("tada", "🎉"),
    ("rocket", "🚀"),
    ("bug", "🐛"),
    ("sparkles", "✨"),
    ("warning", "⚠️"),
    ("white_check_mark", "✅"),
    ("x", "❌"),
    ("eyes", "👀"),
    ("wave", "👋"),
    ("thinking", "🤔"),
    ("ok", "🆗"),
    ("construction", "🚧"),
    ("boom", "💥"),
    ("gear", "⚙️"),
    ("hourglass", "⏳"),
    ("zap", "⚡"),
    ("lock", "🔒"),
    ("unlock", "🔓"),
    ("memo", "📝"),
    ("book", "📖"),
    ("package", "📦"),
    ("hammer", "🔨"),
    ("checkered_flag", "🏁"),
    ("question", "❓"),
    ("exclamation", "❗"),
    ("bulb", "💡"),
];

/// `text` as prose sets it, in pieces that cover it in order: each piece's range in `text`
/// and what it shows as. `before` is the character the text follows on its line, if any, which
/// decides whether a quote that starts the text opens.
pub(super) fn pieces(text: &str, before: Option<char>) -> Vec<(Range<usize>, Cow<'_, str>)> {
    let mut out: Vec<(Range<usize>, Cow<'_, str>)> = Vec::new();
    let mut plain = 0;
    let mut prev = before;
    let mut i = 0;
    let bytes = text.as_bytes();
    while i < text.len() {
        let rest = &text[i..];
        let replaced: Option<(usize, &str)> = if rest.starts_with("...") {
            Some((3, "…"))
        } else if rest.starts_with("---") {
            Some((3, "—"))
        } else if rest.starts_with("--") && i > 0 && bytes[i - 1].is_ascii_digit() && bytes.get(i + 2).is_some_and(u8::is_ascii_digit) {
            Some((2, "–"))
        } else if let Some(c @ ('"' | '\'')) = rest.chars().next() {
            let opens = prev.is_none_or(|p| p.is_whitespace() || matches!(p, '(' | '[' | '{'));
            Some((1, match (c, opens) {
                ('"', true) => "“",
                ('"', false) => "”",
                (_, true) => "‘",
                _ => "’",
            }))
        } else if rest.starts_with(':') {
            match shortcode(rest) {
                Some((len, Some(emoji))) => Some((len, emoji)),
                Some((len, None)) => {
                    i += len;
                    prev = text[..i].chars().next_back();
                    continue;
                }
                None => None,
            }
        } else {
            None
        };
        match replaced {
            Some((len, shown)) => {
                if plain < i {
                    out.push((plain..i, Cow::Borrowed(&text[plain..i])));
                }
                out.push((i..i + len, Cow::Borrowed(shown)));
                i += len;
                plain = i;
                prev = shown.chars().next_back();
            }
            None => {
                let c = rest.chars().next().unwrap_or_default();
                i += c.len_utf8();
                prev = Some(c);
            }
        }
    }
    if plain < text.len() {
        out.push((plain..text.len(), Cow::Borrowed(&text[plain..])));
    }
    out
}

/// A `:name:` shortcode at the start of `text`: its length, and its emoji when it is one of the
/// 32; `None` when `text` does not start with `:`, lower case letters or `_`, and `:`.
fn shortcode(text: &str) -> Option<(usize, Option<&'static str>)> {
    let name_len = text[1..].bytes().take_while(|b| b.is_ascii_lowercase() || *b == b'_').count();
    if name_len == 0 || text.as_bytes().get(1 + name_len) != Some(&b':') {
        return None;
    }
    let name = &text[1..1 + name_len];
    Some((name_len + 2, EMOJI.iter().find(|(n, _)| *n == name).map(|(_, e)| *e)))
}

#[cfg(test)]
mod tests {
    use super::pieces;

    fn shown(text: &str, before: Option<char>) -> String {
        let ps = pieces(text, before);
        let mut at = 0;
        for (r, _) in &ps {
            assert_eq!(r.start, at, "pieces cover the text in order: {ps:?}");
            at = r.end;
        }
        assert_eq!(at, text.len(), "{ps:?}");
        ps.into_iter().map(|(_, s)| s.into_owned()).collect()
    }

    #[test]
    fn prose_sets_as_the_tauri_reader_sets_it() {
        let cases: &[(&str, Option<char>, &str)] = &[
            ("Wait...", None, "Wait…"),
            ("a---b", None, "a—b"),
            ("pages 10--12", None, "pages 10–12"),
            ("run --force", None, "run --force"),
            ("1--2--3", None, "1–2–3"),
            ("-----", None, "—--"),
            ("\"Hi,\" she said", None, "“Hi,” she said"),
            ("it's (\"x\") ['y']", None, "it’s (“x”) [‘y’]"),
            ("\"after\"", Some('a'), "”after”"),
            ("\"after\"", Some(' '), "“after”"),
            (":tada: :fire:!", None, "🎉 🔥!"),
            (":nope: :abc:fire:", None, ":nope: :abc:fire:"),
            ("time 10:30:00", None, "time 10:30:00"),
            ("a :x: b", None, "a ❌ b"),
        ];
        for (text, before, want) in cases {
            assert_eq!(shown(text, *before), *want, "{text:?}");
        }
    }
}
