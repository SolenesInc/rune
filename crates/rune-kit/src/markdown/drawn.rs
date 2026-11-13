//! The texts a Markdown document draws, each mapped back to its source: for an app that
//! anchors notes to what a reader selected ([`Markdown::drawn`]).

use super::*;

/// The selectable texts a [`Markdown`] draws, in document order, each with the source bytes it
/// came from. A selection's pieces name theirs by path ([`Drawn::at`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    names: Vec<String>,
    blocks: Vec<Range<usize>>,
    texts: Vec<DrawnText>,
}

/// One text a [`Markdown`] draws.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawnText {
    at: Vec<DrawnStep>,
    /// The text as drawn, which a [`rune::SelectedText`]'s range indexes.
    pub text: String,
    /// `(source bytes, text bytes)` in order. A pair whose lengths differ (an entity, typographic
    /// punctuation) maps as a whole. Empty for a text with no source of its own, such as a
    /// `<details>` summary.
    pub map: Vec<(Range<usize>, Range<usize>)>,
}

/// Where a [`DrawnText`] sits, one step down from the document: the [`Block`]s it is in, by
/// index among their siblings, and the item, table row and cell, diff line or `<details>`
/// summary that holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawnStep {
    Block(usize),
    Item(usize),
    Head,
    Row(usize),
    Cell(usize),
    Line(usize),
    Summary,
}

use DrawnStep as Step;

impl Drawn {
    /// Every text, in document order.
    pub fn texts(&self) -> &[DrawnText] {
        &self.texts
    }

    /// The source bytes of each top-level block, from the start of its first line to the end
    /// of its last, fences and markers included.
    pub fn blocks(&self) -> &[Range<usize>] {
        &self.blocks
    }

    /// The index in [`Self::texts`] of the text at `path`, a selected piece's path
    /// ([`rune::SelectedText::path`]) from the Markdown's root element down.
    pub fn at(&self, path: &str) -> Option<usize> {
        let mut segs = path.split('/');
        let first = segs.next()?;
        let top = self.names.iter().position(|n| n == first)?;
        let mut at = vec![Step::Block(top)];
        let candidates = |at: &[Step]| self.texts.iter().filter(move |t| t.at.starts_with(at)).count();
        for seg in segs {
            if candidates(&at) <= 1 {
                break;
            }
            let step = match seg.as_bytes().first() {
                Some(b'b') => seg[1..].parse().ok().map(Step::Block),
                Some(b'i') => seg[1..].parse().ok().map(Step::Item),
                Some(b'r') => seg[1..].parse().ok().map(Step::Row),
                Some(b'c') => seg[1..].parse().ok().map(Step::Cell),
                Some(b'l') => seg[1..].parse().ok().map(Step::Line),
                _ if seg == "th" => Some(Step::Head),
                _ if seg == "summary" => Some(Step::Summary),
                _ => None,
            };
            let Some(step) = step else { continue };
            let mut deeper = at.clone();
            deeper.push(step);
            if candidates(&deeper) > 0 {
                at = deeper;
            }
        }
        let mut found = self.texts.iter().enumerate().filter(|(_, t)| t.at.starts_with(&at));
        let (i, _) = found.next()?;
        found.next().is_none().then_some(i)
    }
}

impl DrawnText {
    /// Where it sits, from its top-level block down.
    pub fn at(&self) -> &[DrawnStep] {
        &self.at
    }

    /// The source bytes text bytes `range` came from, in order, adjacent ones merged.
    pub fn source(&self, range: Range<usize>) -> Vec<Range<usize>> {
        let mut out: Vec<Range<usize>> = Vec::new();
        for (src, text) in &self.map {
            let (a, b) = (range.start.max(text.start), range.end.min(text.end));
            if a >= b {
                continue;
            }
            let piece = if src.len() == text.len() {
                src.start + (a - text.start)..src.start + (b - text.start)
            } else {
                src.clone()
            };
            match out.last_mut() {
                Some(last) if last.end >= piece.start => last.end = last.end.max(piece.end),
                _ => out.push(piece),
            }
        }
        out
    }
}

impl Markdown {
    /// The selectable texts this Markdown draws with `k`'s style (or its own
    /// [`Self::style`]), in document order, each mapped to the source bytes it came from.
    /// [`Drawn::at`] finds the text a [`rune::TextSelection`] piece covers, and
    /// [`DrawnText::source`] the source of the selected bytes, which [`Self::highlights`]
    /// takes back. A diagram drawn as a picture has no text.
    pub fn drawn(&self, k: &Kit) -> Drawn {
        let style = self.style.clone().unwrap_or_else(|| MarkdownStyle::from_kit(k));
        let how = Reading { autolink: self.autolink, smart: self.smart };
        let (blocks, _, _, starts, maps) = cache::doc_as(&self.src, how, false);
        let keys = if self.block_keys.is_empty() { Vec::new() } else {
            starts.iter().map(|at| self.block_keys.iter().find(|(offset, _)| offset == at)
                .map_or_else(String::new, |(_, key)| key.clone())).collect::<Vec<_>>()
        };
        let mut texts = Vec::new();
        collect(&blocks, &maps, &mut Vec::new(), style.code_block, &mut texts);
        let ends = starts.iter().skip(1).copied().chain([self.src.len()]);
        let ranges = starts.iter().zip(ends).map(|(&a, b)| a..a + self.src[a..b].trim_end().len()).collect();
        Drawn { names: rendered_keys(blocks.len(), &keys), blocks: ranges, texts }
    }
}

fn collect(blocks: &[Block], maps: &[BlockMap], at: &mut Vec<Step>, code_style: TextStyle, out: &mut Vec<DrawnText>) {
    let empty = BlockMap::None;
    for (i, (b, map)) in blocks.iter().zip(maps.iter().chain(std::iter::repeat(&empty))).enumerate() {
        at.push(Step::Block(i));
        match (b, map) {
            (Block::Para(inl) | Block::Heading(_, inl), m) => push(out, at, &inl.text, inline_map(m)),
            (Block::Code(code), m) => push_code(out, at, code, inline_map(m), code_style),
            #[cfg(feature = "mermaid")]
            (Block::Mermaid(code), _) if crate::mermaid::supported(code) => {}
            (Block::Mermaid(code), m) => push_code(out, at, code, inline_map(m), code_style),
            (Block::Diff(code), m) => {
                let map = inline_map(m);
                let mut start = 0;
                for (n, line) in code.split('\n').enumerate() {
                    let end = start + line.len();
                    let shown = line.strip_suffix('\r').unwrap_or(line);
                    let line_map = map.iter().filter_map(|(src, text)| {
                        let (a, b) = (text.start.max(start), text.end.min(start + shown.len()));
                        (a < b).then(|| {
                            let src = if src.len() == text.len() { src.start + (a - text.start)..src.start + (b - text.start) } else { src.clone() };
                            (src, a - start..b - start)
                        })
                    }).collect();
                    at.push(Step::Line(n));
                    push_code(out, at, if shown.is_empty() { " " } else { shown }, line_map, code_style);
                    at.pop();
                    start = end + 1;
                }
            }
            (Block::Table(t), BlockMap::Table { head, rows }) => {
                for (j, cell) in t.head.iter().enumerate() {
                    if t.pictures.iter().any(|p| p.row.is_none() && p.column == j) { continue; }
                    at.extend([Step::Head, Step::Cell(j)]);
                    push(out, at, &cell.text, head.get(j).map(|m| m.0.clone()).unwrap_or_default());
                    at.truncate(at.len() - 2);
                }
                for (r, row) in t.rows.iter().enumerate() {
                    for (j, cell) in row.iter().enumerate() {
                        if t.pictures.iter().any(|p| p.row == Some(r) && p.column == j) { continue; }
                        at.extend([Step::Row(r), Step::Cell(j)]);
                        push(out, at, &cell.text, rows.get(r).and_then(|m| m.get(j)).map(|m| m.0.clone()).unwrap_or_default());
                        at.truncate(at.len() - 2);
                    }
                }
            }
            (Block::List { items, .. }, m) => {
                let item_maps = match m { BlockMap::List(m) => m.as_slice(), _ => &[] };
                for (k, item) in items.iter().enumerate() {
                    at.push(Step::Item(k));
                    collect(item, item_maps.get(k).map_or(&[], Vec::as_slice), at, code_style, out);
                    at.pop();
                }
            }
            (Block::Quote(inner) | Block::Alert(_, inner), m) => {
                let inner_maps = match m { BlockMap::Quote(m) => m.as_slice(), _ => &[] };
                collect(inner, inner_maps, at, code_style, out);
            }
            (Block::Details { summary, body, .. }, m) => {
                at.push(Step::Summary);
                push(out, at, &summary.text, Vec::new());
                at.pop();
                let body_maps = match m { BlockMap::Details { body, .. } => body.as_slice(), _ => &[] };
                collect(body, body_maps, at, code_style, out);
            }
            _ => {}
        }
        at.pop();
    }
}

fn inline_map(m: &BlockMap) -> Vec<(Range<usize>, Range<usize>)> {
    match m {
        BlockMap::Inline(m) | BlockMap::Code(m) => m.0.clone(),
        _ => Vec::new(),
    }
}

fn push(out: &mut Vec<DrawnText>, at: &[Step], text: &str, map: Vec<(Range<usize>, Range<usize>)>) {
    out.push(DrawnText { at: at.to_vec(), text: text.to_owned(), map });
}

/// A code text as [`tabbed`](super::render) draws it in `style`, its map moved past the
/// padding the tabs gain.
fn push_code(out: &mut Vec<DrawnText>, at: &[Step], code: &str, map: Vec<(Range<usize>, Range<usize>)>, style: TextStyle) {
    let (shown, shift) = render::tab_shown(code, style);
    let map = map.into_iter().map(|(src, text)| (src, shift(text.start)..shift(text.end))).collect();
    out.push(DrawnText { at: at.to_vec(), text: shown, map });
}
