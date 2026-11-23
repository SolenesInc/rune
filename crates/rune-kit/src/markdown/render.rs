//! Blocks to elements: block keys and margins, code and diff blocks, lists, quotes, tables and
//! diagrams.

use super::*;

pub(super) struct Renderer {
    pub(super) s: MarkdownStyle,
    pub(super) highlights: Vec<(Vec<Range<usize>>, Highlight)>,
    pub(super) reveal: f32,
    pub(super) streaming: bool,
    pub(super) on_link: Option<LinkWire>,
    pub(super) on_highlight: Option<LinkWire>,
    pub(super) on_link_secondary: Option<LinkWire>,
    pub(super) link_look: Option<LinkLook>,
    pub(super) link_help: Option<LinkHelp>,
    pub(super) on_task: Option<TaskWire>,
    pub(super) details: Option<DetailsWire>,
    pub(super) tasks_read_only: bool,
    pub(super) link_ring: Option<LinkRing>,
    /// `Markdown::code_link`.
    pub(super) code_link: Option<CodeLink>,
    pub(super) selectable: bool,
    /// `Markdown::scroll_wide`.
    pub(super) scroll_wide: bool,
    /// `Markdown::dir_tentative`.
    pub(super) tentative: Option<rune::Direction>,
    pub(super) highlighter: Highlighter,
    pub(super) sources: Rc<Vec<Source>>,
    /// The next of `sources`, advanced at each [`Block::Code`] in document order.
    pub(super) next_source: Cell<usize>,
    pub(super) tasks: Rc<Vec<Option<(bool, usize)>>>,
    /// The next of `tasks`, advanced at each list item in document order.
    pub(super) next_task: Cell<usize>,
    /// `Markdown::id`, and the next sideways scroll bar's number in the top-level block being
    /// built: a bar is named by the document, its block's key and that number.
    pub(super) id: String,
    pub(super) next_wide: Cell<usize>,
    pub(super) current_block_key: RefCell<Rc<str>>,
    /// The text a stream's caret follows ([`MarkdownStyle::caret`]): where the last inline's
    /// or code block's string is held, which names it among the document's.
    pub(super) caret_after: Option<*const u8>,
    /// When a top-level block's element can be kept from one build to the next: a digest of
    /// everything but the block that decides what it draws, and each block's own digest
    /// ([`cache::Parsed::digests`]). With the app's handlers or frames in the elements
    /// ([`Markdown::kept`]), each block's start in the source too. `None` when a block's
    /// element holds what a digest can't tell apart.
    pub(super) kept: Option<((u64, u64), Rc<Vec<Option<(u64, u64)>>>, Option<Rc<Vec<usize>>>)>,
    /// `Markdown::diagram_frame`.
    #[cfg(feature = "mermaid")]
    pub(super) diagram_frame: Option<FrameWire>,
    /// `Markdown::diagram_source_frame`.
    #[cfg(feature = "mermaid")]
    pub(super) diagram_source_frame: Option<FrameWire>,
    /// `Markdown::code_frame`.
    pub(super) code_frame: Option<CodeWire>,
    /// `Markdown::quote_frame`.
    pub(super) quote_frame: Option<QuoteWire>,
    /// `Markdown::front_matter_frame`.
    pub(super) front_matter_frame: Option<FrontMatterWire>,
    /// `Markdown::link_favicon`.
    pub(super) link_favicon: Option<link_icon::FaviconWire>,
    /// `Markdown::images` and `Markdown::on_image`.
    pub(super) images: Option<image::ImageWire>,
    pub(super) on_image: Option<image::ImagePress>,
    pub(super) missing_image: Option<image::MissingFrame>,
    /// Where the blocks being drawn sit: list depth and a quote's text colour.
    pub(super) flow: Cell<Flow>,
}

/// What the blocks being drawn sit in.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Flow {
    /// Lists around them (0 at the top level).
    pub(super) lists: u32,
    /// A block quote's text colour, when they're in one.
    pub(super) ink: Option<Color>,
    /// How much narrower than the text column they are: list indents and quote rules.
    pub(super) inset: f32,
    /// The columns a nested list's copied markers are indented by (`"  - "` under `"- "`).
    pub(super) cols: u8,
    /// The marker a copy puts before the paragraph that opens a list item, its indent, and the
    /// newlines before it: two to set a top-level list apart from the block before it.
    pub(super) lead: Option<(u8, Lead, u8)>,
    /// Whether their text is italic, as a quote's may be.
    pub(super) italic: bool,
    /// Whether their text is struck through, as a finished task's may be.
    pub(super) struck: bool,
    /// The widest their lines run when they sit in a top-level list, quote or alert held to a
    /// [`MarkdownStyle::prose_measure`]: the measure less the indents and rules since its start.
    pub(super) measure: Option<f32>,
}

/// A list item's marker as copying writes it.
#[derive(Clone, Copy, Debug)]
pub(super) enum Lead {
    Bullet,
    Number(u64),
    Task(bool),
}

impl Lead {
    pub(super) fn marker(self) -> String {
        match self {
            Lead::Bullet => "- ".into(),
            Lead::Number(n) => format!("{n}. "),
            Lead::Task(done) => if done { "- [x] ".into() } else { "- [ ] ".into() },
        }
    }

    /// The columns a list nested in the item is indented by, past the item's own.
    pub(super) fn width(self) -> u8 {
        match self {
            Lead::Number(n) => n.to_string().len() as u8 + 2,
            _ => 2,
        }
    }
}

/// Shell languages, which [`MarkdownStyle::shell_code`] styles apart (`markdown.tsx`'s list).
const SHELLS: [&str; 7] = ["bash", "sh", "shell", "zsh", "fish", "console", "terminal"];
/// Plain-text fence names, which [`MarkdownStyle::plain_code_text`] colours as a fence with none.
const PLAIN_TEXT: [&str; 4] = ["text", "txt", "plain", "plaintext"];

/// The keys [`Renderer::blocks`] gives `n` blocks.
pub(super) fn rendered_keys(n: usize, keys: &[String]) -> Vec<String> {
    let mut used = Vec::new();
    (0..n).map(|i| if keys.is_empty() { format!("b{i}") } else { block_key(i, keys, &mut used) }).collect()
}

/// [`Markdown::copy_source`]: the source of the top-level blocks (keyed `names`, starting at
/// `starts`) that `sel` touches, `None` for part of one block.
pub(super) fn source_copy(src: &str, starts: &[usize], names: &[String], sel: &rune::TextSelection) -> Option<String> {
    // The longest key that is the path or its first segments: an app's key may hold a `/`.
    let block_of = |path: &str| names.iter().enumerate()
        .filter(|(_, k)| path == k.as_str() || path.strip_prefix(k.as_str()).is_some_and(|rest| rest.starts_with('/')))
        .max_by_key(|(_, k)| k.len()).map(|(b, _)| b);
    let mut touched: Option<(usize, usize)> = None;
    for t in sel.texts().iter().filter(|t| t.is_selected()) {
        let Some(b) = block_of(t.path()) else { continue };
        touched = Some(touched.map_or((b, b), |(first, _)| (first, b)));
    }
    let (first, last) = touched?;
    if first == last && !sel.texts().iter().filter(|t| block_of(t.path()) == Some(first)).all(|t| t.is_whole() || t.is_empty()) {
        return None;
    }
    if (first == 0 && last + 1 >= starts.len()) || sel.texts().iter().all(|t| t.is_whole() || t.is_empty()) {
        return Some(src.to_owned());
    }
    let start = *starts.get(first)?;
    let end = starts.get(last + 1).copied().unwrap_or(src.len());
    Some(src.get(start..end)?.trim_end().to_owned())
}

fn block_key(i: usize, keys: &[String], used: &mut Vec<String>) -> String {
    if let Some(k) = keys.get(i).filter(|k| !k.is_empty() && !used.iter().any(|u| u == *k)) {
        used.push(k.clone());
        return k.clone();
    }
    let taken = |cand: &str, used: &[String]| {
        used.iter().any(|u| u == cand) || keys.get(i + 1..).is_some_and(|rest| rest.iter().any(|k| k == cand))
    };
    let mut cand = format!("b{i}");
    let mut n = 0usize;
    while taken(&cand, used) {
        cand = format!("b{n}");
        n += 1;
    }
    used.push(cand.clone());
    cand
}

/// Whether `b` keeps [`MarkdownStyle::prose_measure`] at the top level: the blocks a reader
/// reads line by line, not the tables, code, diagrams, pictures and front matter that want the room.
fn measured(b: &Block) -> bool {
    matches!(b, Block::Para(_) | Block::Heading(..) | Block::List { .. } | Block::Quote(_) | Block::Alert(..))
}

/// Whether `b` sits centered in a column that keeps a [`MarkdownStyle::prose_measure`]: the
/// measured prose, and a picture narrower than the column.
fn centered(b: &Block) -> bool {
    measured(b) || matches!(b, Block::Image { .. })
}

/// Whether `b` holds blocks that keep the measure beside blocks that take the column: measured
/// at the top level, it then starts where the measure starts and runs to the column's end, its
/// lines held to the measure and its pictures and tables running past it.
fn holds(b: &Block) -> bool {
    matches!(b, Block::List { .. } | Block::Quote(_) | Block::Alert(..) | Block::Details { .. })
}

/// Whether `b`, inside a block that [`holds`] others, runs past the measure to the column's end.
fn wide(b: &Block) -> bool {
    matches!(b, Block::Image { .. } | Block::Table(_))
}

/// (margin-top, margin-bottom) per block, collapsed like CSS margins.
pub(super) fn margins(b: &Block, s: &MarkdownStyle) -> (f32, f32) {
    match b {
        Block::Para(_) => (0.0, s.margins.para),
        Block::Heading(level, _) => {
            let [top, bottom] = s.margins.headings[usize::from((*level).clamp(1, 4)) - 1];
            (top, bottom)
        }
        Block::Code { .. } | Block::Diff(_) | Block::Mermaid(_) => (s.code_margin[0], s.code_margin[1]),
        Block::List { .. } => (s.margins.list[0], s.margins.list[1]),
        Block::Quote(_) | Block::Alert(..) => (s.margins.quote[0], s.margins.quote[1]),
        Block::FrontMatter(_) => (0.0, 16.0),
        Block::Rule => (s.margins.rule[0], s.margins.rule[1]),
        Block::Table(_) => (s.table_margin[0], s.table_margin[1]),
        Block::Image { .. } => (24.0, 24.0),
        Block::Details { .. } => (0.0, 12.0),
    }
}

/// Block `b`'s (margin-top, margin-bottom) as CSS collapses them, `i` its place among its
/// siblings and `in_item` whether they're a list item's: a list item's first paragraph has
/// none, a later one 8 above and 12 below, a nested list 4 each way. A quote or a list has no
/// vertical padding or border, so its first child's top margin and its last child's bottom
/// margin pass through it, and a list item's 8 through the list: a quote that ends a list item
/// spaces the list 16 from what follows, as it would standing alone.
pub(super) fn collapsed(b: &Block, s: &MarkdownStyle, i: usize, in_item: bool) -> (f32, f32) {
    let (mt, mb) = match b {
        Block::Para(_) if in_item => (if i == 0 { 0.0 } else { 8.0 }, if i == 0 { 0.0 } else { 12.0 }),
        Block::List { .. } if in_item => (4.0, 4.0),
        _ => margins(b, s),
    };
    let (first, last) = match b {
        Block::Quote(inner) => (
            inner.first().map(|f| collapsed(f, s, 0, false).0),
            inner.last().map(|l| collapsed(l, s, inner.len() - 1, false).1),
        ),
        Block::List { items, .. } => (
            items.first().and_then(|item| item.first()).map(|f| collapsed(f, s, 0, true).0),
            items.last().map(|item| item.last().map_or(0.0, |l| collapsed(l, s, item.len() - 1, true).1).max(s.list_gap)),
        ),
        _ => (None, None),
    };
    (first.map_or(mt, |f| f.max(mt)), last.map_or(mb, |l| l.max(mb)))
}

thread_local! {
    static PAD_SIZES: RefCell<std::collections::HashMap<u64, f32>> = RefCell::new(std::collections::HashMap::new());
}

/// The size that sets inline code's padding spaces (`PAD`, in the text's own `font`) to `pad`
/// points a side, so the text beside the chip sits where a page's `padding: 0 4px` puts it.
/// Solved twice: a variable font's optical size moves its spaces' widths with the size.
fn pad_size(font: TextStyle, pad: f32) -> f32 {
    let key = hash_of((font, pad.to_bits(), rune::text::font_epoch()));
    if let Some(size) = PAD_SIZES.with_borrow(|m| m.get(&key).copied()) {
        return size;
    }
    let probe = PAD.repeat(64);
    let wide = |size: f32| (rune::text::measure(&probe, &[], TextStyle { size, ..font }, None).0 / 64.0).max(0.1);
    let first = font.size * pad / wide(font.size);
    let size = first * pad / wide(first);
    PAD_SIZES.with_borrow_mut(|m| m.insert(key, size));
    size
}

/// Code as rich text with `spans` and `highlights` (byte ranges of `code`). Text layout widens
/// tabs in a monospace `style` to four-column stops itself; in any other family it draws a tab
/// as one space, so each tab is padded with uncopied spaces to the next stop: lines line up and a
/// copy still gets the tab.
/// `code` as [`tabbed`] draws it in `style`, each tab padded to the next stop unless the font
/// is monospaced, and where a byte of `code` lands in it.
pub(super) fn tab_shown(code: &str, style: TextStyle) -> (String, impl Fn(usize) -> usize) {
    let (shown, tabs) = tabs_padded(code, style);
    (shown, move |b: usize| shift_past(&tabs, b))
}

/// (source byte, shown byte, pad, padding so far including this tab's) for each padded tab.
type Tabs = Vec<(usize, usize, usize, usize)>;

fn tabs_padded(code: &str, style: TextStyle) -> (String, Tabs) {
    let mut tabs: Tabs = Vec::new();
    if !code.contains('\t') || matches!(style.family, Family::Mono | Family::SystemMono) {
        return (code.to_owned(), tabs);
    }
    let mut shown = String::with_capacity(code.len() + code.len() / 8);
    let mut col = 0;
    for (i, c) in code.char_indices() {
        if c == '\t' {
            let pad = 3 - col % 4;
            let total = tabs.last().map_or(0, |t| t.3) + pad;
            tabs.push((i, shown.len(), pad, total));
            shown.push('\t');
            shown.extend(std::iter::repeat_n(' ', pad));
            col += pad + 1;
            continue;
        }
        col = if c == '\n' { 0 } else { col + 1 };
        shown.push(c);
    }
    (shown, tabs)
}

fn shift_past(tabs: &Tabs, b: usize) -> usize {
    match tabs.partition_point(|t| t.0 < b) {
        0 => b,
        n => b + tabs[n - 1].3,
    }
}

/// How many of the renderer's sources and tasks building `b` takes (see
/// [`Renderer::next_source`] and [`Renderer::next_task`]), or `None` for a block whose element
/// is more than its source says: a diagram, which draws itself in over time.
fn spent(b: &Block) -> Option<(usize, usize)> {
    let all = |blocks: &[Block]| blocks.iter().try_fold((0, 0), |n, b| spent(b).map(|m| (n.0 + m.0, n.1 + m.1)));
    match b {
        Block::Code(_) => Some((1, 0)),
        Block::Mermaid(_) => None,
        Block::List { items, .. } => items.iter().try_fold((0, items.len()), |n, item| all(item).map(|m| (n.0 + m.0, n.1 + m.1))),
        Block::Quote(inner) | Block::Alert(_, inner) | Block::Details { body: inner, .. } => all(inner),
        _ => Some((0, 0)),
    }
}

/// How many lines of a code block share one text (see [`Renderer::code`]).
pub(super) const CODE_GROUP_LINES: usize = 32;

/// A stream's caret: a no-break space apart from the last word, then one that carries the
/// caret's colour as its background. Neither lets a line break before it, so the caret never
/// stands alone on a line.
const CARET: &str = "\u{a0}\u{a0}";

/// Where the string of the last text of `blocks` is held: what a stream's caret follows. `None`
/// when the document ends in something that isn't text (a table, a picture, a rule) or in text
/// with nothing in it yet.
pub(super) fn last_text(blocks: &[Block]) -> Option<*const u8> {
    match blocks.last()? {
        Block::Para(i) | Block::Heading(_, i) => Some(&i.text),
        Block::Code(c) => Some(c),
        Block::List { items, .. } => return last_text(items.last()?),
        Block::Quote(inner) | Block::Alert(_, inner) | Block::Details { body: inner, .. } => return last_text(inner),
        _ => None,
    }
    .filter(|t| !t.is_empty())
    .map(|t| t.as_ptr())
}

fn tabbed(code: &str, mut spans: Vec<Span>, mut highlights: Vec<TextHighlight>, (presses, strikes): (Vec<(Range<usize>, String)>, Vec<Range<usize>>), style: TextStyle) -> El {
    let (shown, tabs) = tabs_padded(code, style);
    let shift = |b: u32| shift_past(&tabs, b as usize) as u32;
    let pressed = |el: El| {
        let el = presses.iter().fold(el, |el, (r, id)| el.link(press_link(shift(r.start as u32) as usize..shift(r.end as u32) as usize, id)));
        el.strikes(strikes.iter().map(|r| (shift(r.start as u32) as usize, shift(r.end as u32) as usize, None)))
    };
    if tabs.is_empty() {
        return pressed(rich(shown, spans).text_highlights(highlights));
    }
    for sp in &mut spans {
        (sp.start, sp.end) = (shift(sp.start), shift(sp.end));
    }
    for hl in &mut highlights {
        let r = hl.range();
        *hl = hl.clone().with_range(shift(r.start as u32) as usize..shift(r.end as u32) as usize);
    }
    tabs.iter().fold(pressed(rich(shown, spans).text_highlights(highlights)), |el, t| el.uncopied(t.1 + 1..t.1 + 1 + t.2))
}

/// A highlight's button over `range`, a [`Markdown::on_highlight`] press as link `id`.
fn press_link(range: Range<usize>, id: &str) -> Link {
    Link::new(range, id).role("button").hover(LinkPaint::new()).drag_selects()
}

/// `range` less the parts `links` cover, in order.
fn outside(range: Range<usize>, links: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut out = vec![range];
    for l in links {
        out = out.into_iter().flat_map(|r| [r.start..r.end.min(l.start), r.start.max(l.end)..r.end]).filter(|r| r.start < r.end).collect();
    }
    out
}

impl Renderer {
    pub(super) fn spans(&self, inl: &Inline, font: TextStyle) -> Vec<Span> {
        let inl = &*self.with_code_links(inl);
        let s = &self.s;
        let base = font.size;
        let links: Vec<Span> = if inl.links.is_empty() {
            Vec::new()
        } else {
            inl.spans.iter().filter(|s| matches!(s.2, Style::Link)).map(|&(a, b, _)| self.link_span(a, b, inl)).collect()
        };
        let mut next_link = 0;
        let mut out = Vec::with_capacity(inl.spans.len());
        for &(a, b, st) in &inl.spans {
            if matches!(st, Style::Link) {
                let sp = links[next_link].clone();
                next_link += 1;
                let at = out.iter().position(|p: &Span| p.start < sp.end && sp.start < p.end).unwrap_or(out.len());
                out.insert(at, sp);
                continue;
            }
            if matches!(st, Style::Strike) {
                continue;
            }
            let in_link = links.iter().find(|l| l.start <= a as u32 && b as u32 <= l.end);
            let mut sp = Span::new(a, b);
            // Code in a link is a chip in the link's colour, not underlined as well.
            let no_line = in_link.is_some_and(|l| l.underline.is_some()).then_some(Color::TRANSPARENT);
            match st {
                Style::Code if self.code_link_color(a, b, inl).is_some() => {
                    sp.mono = true;
                    sp.weight = Some(s.inline_code_weight);
                    sp.size = Some(s.inline_code_size.unwrap_or(base * s.inline_code_scale));
                    sp.color = self.code_link_color(a, b, inl);
                }
                Style::Code => {
                    let link_base = in_link.and_then(|l| l.size).unwrap_or(base);
                    sp.mono = true;
                    sp.weight = Some(s.inline_code_weight);
                    sp.size = Some(s.inline_code_size.unwrap_or(link_base * s.inline_code_scale));
                    sp.underline = no_line;
                    if in_link.is_none_or(|l| l.bg.is_none()) {
                        sp = sp.bg(s.code_bg, s.inline_code_pad);
                        sp.bg_pad_y = s.inline_code_pad_y;
                    }
                }
                Style::CodePad => {
                    sp.size = Some(pad_size(font, s.inline_code_pad));
                    sp.underline = no_line;
                }
                Style::Strong => sp.weight = Some(s.strong_weight),
                Style::Em => sp.italic = true,
                Style::Strike | Style::Link => unreachable!(),
            }
            out.push(sp);
        }
        out
    }

    /// Resolves `hl` against [`MarkdownStyle`] into a [`TextHighlight`] for `range`.
    fn make_text_highlight(&self, idx: usize, range: Range<usize>, hl: &Highlight) -> TextHighlight {
        let s = &self.s;
        let bg = hl.bg.unwrap_or(if hl.current { s.highlight_current_bg } else { s.highlight_bg });
        let fg = hl.fg.or(if hl.current { s.highlight_current_fg } else { s.highlight_fg });
        let ring = hl.ring.or(if hl.current { s.highlight_current_ring } else { s.highlight_ring });
        let key = hl.key.clone().unwrap_or_else(|| Markdown::highlight_key(idx));
        let mut th = TextHighlight::new(range, bg).key(key);
        if let Some(c) = fg {
            th = th.color(c);
        }
        if let Some((rc, rw)) = ring {
            th = th.ring(rc, rw);
        }
        if let Some(pad) = hl.pad {
            th = th.pad(pad);
        }
        if let Some(radius) = hl.radius {
            th = th.radius(radius);
        }
        th
    }

    /// The highlights over `text` (mapped from source by `map`): each text range with its
    /// highlight's index.
    fn marks_for(&self, text: &str, map: Option<&SpanMap>) -> Vec<(Range<usize>, usize)> {
        let Some(map) = map.filter(|_| !self.highlights.is_empty()) else { return Vec::new() };
        self.highlights.iter().enumerate().flat_map(|(idx, (src_ranges, _))| map.map_ranges(src_ranges, text).into_iter().map(move |r| (r, idx))).collect()
    }

    /// The byte ranges of `marks` whose highlights strike their words through.
    fn strikes(&self, marks: &[(Range<usize>, usize)]) -> Vec<Range<usize>> {
        marks.iter().filter(|(_, idx)| self.highlights[*idx].1.strike).map(|(r, _)| r.clone()).collect()
    }

    fn text_highlights(&self, marks: &[(Range<usize>, usize)]) -> Vec<TextHighlight> {
        marks.iter().map(|(r, idx)| self.make_text_highlight(*idx, r.clone(), &self.highlights[*idx].1)).collect()
    }

    /// The buttons [`Markdown::on_highlight`] makes of `marks`, around `links`: each range with
    /// its link id. None without a handler.
    fn presses(&self, marks: &[(Range<usize>, usize)], links: &[Range<usize>]) -> Vec<(Range<usize>, String)> {
        if self.on_highlight.is_none() { return Vec::new() }
        marks.iter().flat_map(|(r, idx)| {
            let key = self.highlights[*idx].1.key.clone().unwrap_or_else(|| Markdown::highlight_key(*idx));
            let id = format!("{HIGHLIGHT_PRESS}{key}");
            outside(r.clone(), links).into_iter().map(move |piece| (piece, id.clone()))
        }).collect()
    }

    /// `e` hearing its highlights' presses, when it holds any.
    fn wire_presses(&self, e: El, any: bool) -> El {
        match &self.on_highlight {
            Some(wire) if any => wire(e),
            _ => e,
        }
    }

    /// `e`, a text, selectable when the markdown is (after its links, so it shows the I-beam).
    pub(super) fn select(&self, e: El) -> El {
        if self.selectable { e.selectable(true).selection_color(self.s.selection) } else { e }
    }

    /// `e`, the text of `inl`, with its strikethroughs, highlights, links and selectable, copying
    /// without inline code's padding.
    pub(super) fn select_inline(&self, mut e: El, inl: &Inline, map: Option<&SpanMap>) -> El {
        for &(a, b, st) in &inl.spans {
            if matches!(st, Style::Strike) {
                e = e.strike(a, b, None);
            }
        }
        let marks = self.marks_for(&inl.text, map);
        e = e.text_highlights(self.text_highlights(&marks)).strikes(self.strikes(&marks).into_iter().map(|r| (r.start, r.end, None)));
        let e = self.wire_link(self.link_icons(e, inl), inl);
        let links: Vec<Range<usize>> = if self.on_link.is_some() || self.on_link_secondary.is_some() { inl.links.iter().map(|(a, b, _)| *a..*b).collect() } else { Vec::new() };
        let presses = self.presses(&marks, &links);
        let any = !presses.is_empty();
        let e = self.select(self.wire_presses(presses.iter().fold(e, |e, (r, id)| e.link(press_link(r.clone(), id))), any));
        if !self.selectable { return e; }
        inl.spans.iter().filter(|s| matches!(s.2, Style::CodePad)).fold(e, |e, s| e.uncopied(s.0..s.1))
    }

    pub(super) fn with_reveal(&self, e: El) -> El {
        if self.reveal >= 0.0 {
            e.reveal(self.reveal)
        } else if self.streaming {
            e.reveal(0.0)
        } else {
            e
        }
    }

    pub(super) fn para(&self, inl: &Inline, map: Option<&SpanMap>, style: TextStyle, color: Color) -> El {
        // Lines keep their own 22.4 points, as a page's do, so a long list doesn't drift a
        // point per item.
        let flow = self.flow.get();
        let style = TextStyle { italic: style.italic || flow.italic, ..style };
        let e = self.with_reveal(rich(inl.text.clone(), self.spans(inl, style)).font(style).color(color).exact_lines()).w_full();
        let e = if flow.struck { e.strike(0, inl.text.len(), None) } else { e };
        let e = bidi::directed(bidi::isolated(e, inl), bidi::inline_dir(inl));
        // A copy puts a blank line around a paragraph, one newline around a list item's first
        // with its marker before it.
        let e = match self.flow.get().lead {
            Some((cols, lead, before)) => e.copy_prefix(format!("{}{}", " ".repeat(cols.into()), lead.marker())).copy_breaks(before, 1),
            None => e.copy_breaks(2, 2),
        };
        self.caret(self.select_inline(e, inl, map), &inl.text)
    }

    /// `e`, which draws `text`, ending in the stream's caret when `text` is the document's last.
    pub(super) fn caret(&self, e: El, text: &str) -> El {
        match self.s.caret {
            Some(color) if self.caret_after == Some(text.as_ptr()) => e.end_mark(CARET, [Span::new(CARET.len() / 2, CARET.len()).bg(color, 0.0)]),
            _ => e,
        }
    }

    /// `blocks` in a column, spaced by their collapsed margins. A heading's bottom margin alone
    /// sets the gap under it (CSS's `h2 + p { margin-top: 0 }`). In a list item (`in_item`) the
    /// first paragraph has no margins, a later one 8 above, and a nested list 4 above and below,
    /// as a web page spaces an `li`'s children.
    pub(super) fn blocks(&self, blocks: &[Block], maps: &[BlockMap], keys: &[String], top_level: bool) -> El {
        self.blocks_in(blocks, maps, keys, top_level, false)
    }

    pub(super) fn blocks_in(&self, blocks: &[Block], maps: &[BlockMap], keys: &[String], top_level: bool, in_item: bool) -> El {
        let mut c = col().w_full();
        let mut prev_mb = 0.0f32;
        let mut after_heading = false;
        let names = rendered_keys(blocks.len(), keys);
        let empty = BlockMap::None;
        // A block with no strong character of its own reads the way the one before it does.
        let mut last = if top_level { self.tentative } else { None };
        for ((i, b), key) in blocks.iter().enumerate().zip(names) {
            // A fence still arriving with no lines yet (its language may be half written:
            // ```` ```mer ````) shows nothing until its first line, then mounts as what it is.
            if self.streaming && matches!(b, Block::Code(c) if c.is_empty()) {
                let n = self.next_source.get();
                if self.sources.get(n).is_some_and(|s| s.open && s.len == 0) {
                    self.next_source.set(n + 1);
                    continue;
                }
            }
            let (mt, mb) = collapsed(b, &self.s, i, in_item);
            let top = match b {
                _ if i == 0 => 0.0,
                // An image keeps its own 24 under a heading: the page zeroes the top margin of
                // the paragraph holding it, not the image's, which collapses through.
                Block::Heading(..) | Block::Rule | Block::Image { .. } => mt.max(prev_mb),
                _ if after_heading && !self.s.margins.collapse_under_heading => prev_mb,
                _ => mt.max(prev_mb),
            };
            if top_level {
                self.current_block_key.replace(key.clone().into());
                // Its sideways scroll bars are named within it, so a kept block's are its own.
                self.next_wide.set(0);
            }
            // Only an item's first block copies after its marker.
            if i > 0 {
                self.flow.set(Flow { lead: None, ..self.flow.get() });
            }
            let own = bidi::blocks_dir(std::slice::from_ref(b));
            let prose = !matches!(b, Block::Code(_) | Block::Diff(_) | Block::Mermaid(_) | Block::Image { .. } | Block::Rule);
            let dir = own.or(last).filter(|_| prose);
            let build = |key: String| {
                #[cfg(test)]
                if top_level {
                    BLOCKS.with(|n| n.set(n.get() + 1));
                }
                let measure = self.s.prose_measure.filter(|_| top_level && measured(b));
                let spans = measure.is_some() && holds(b);
                let flow = self.flow.get();
                if let Some(measure) = measure {
                    // Measured prose is narrower than the column by what the measure leaves over,
                    // and a block that holds others by what it leaves before the measure.
                    let before = self.s.text_width.map_or(0.0, |width| (width - measure).max(0.0) / if spans { 2.0 } else { 1.0 });
                    self.flow.set(Flow { inset: flow.inset + before, measure: spans.then_some(measure), ..flow });
                }
                let drawn = self.block(b, maps.get(i).unwrap_or(&empty));
                self.flow.set(flow);
                let drawn = match (measure, flow.measure) {
                    (Some(measure), _) if spans => drawn.grow_from(1.0, measure),
                    (Some(measure), _) => drawn.w_full().max_w(measure),
                    (None, Some(measure)) if !holds(b) && !wide(b) => drawn.w_full().max_w(measure),
                    (None, _) => drawn,
                };
                let drawn = bidi::directed(drawn, dir);
                let drawn = if top_level && self.s.prose_measure.is_some() && centered(b) && !spans { drawn.self_center() } else { drawn };
                let drawn = drawn.key(key);
                match spans {
                    true => bidi::directed(row().w_full().child(el().grow(1.0)).child(drawn), dir).mt(top),
                    false => drawn.mt(top),
                }
            };
            // A block that can't change any more is built once: the blocks above a stream's
            // last cost a lookup each time it grows, so a reply costs what its newest block
            // costs. The last block of a stream changes with every chunk, so it always builds.
            let digest = self.kept.as_ref().filter(|_| top_level && !(self.streaming && i + 1 == blocks.len()))
                .and_then(|(all, each, starts)| Some((all, each.get(i).copied().flatten()?, starts)));
            let drawn = match (digest, spent(b)) {
                (Some((all, own, starts)), Some((sources, tasks))) => {
                    let (s0, t0) = (self.next_source.get(), self.next_task.get());
                    let place = starts.as_ref().map(|at| (at.get(i).copied(), s0, t0));
                    let h = rune::memo::digest(&(all, own, &key, top.to_bits(), dir, place));
                    let drawn = rune::memo::cached(h, || {
                        let drawn = build(key);
                        debug_assert_eq!((self.next_source.get() - s0, self.next_task.get() - t0), (sources, tasks), "`spent` counts what a block's build takes");
                        drawn
                    });
                    // A kept block's build didn't run: what it would have taken is still spent.
                    self.next_source.set(s0 + sources);
                    self.next_task.set(t0 + tasks);
                    drawn
                }
                _ => build(key),
            };
            if own.is_some() {
                last = own;
            }
            c = c.child(drawn);
            prev_mb = mb;
            after_heading = matches!(b, Block::Heading(..));
        }
        c
    }

    pub(super) fn code(&self, code: &str, source: Option<&Source>, map: Option<&SpanMap>) -> El {
        let s = &self.s;
        let marks = self.marks_for(code, map);
        let (highlights, presses) = (self.text_highlights(&marks), self.presses(&marks, &[]));
        let any = !presses.is_empty();
        let shell = s.shell.filter(|_| source.and_then(|src| src.lang.as_deref()).is_some_and(|l| SHELLS.iter().any(|sh| l.eq_ignore_ascii_case(sh))));
        let plain = source.and_then(|src| src.lang.as_deref()).is_none_or(|l| PLAIN_TEXT.iter().any(|p| l.eq_ignore_ascii_case(p)));
        let (fill, border, ink) = match shell {
            Some(sh) => (sh.fill, sh.border, sh.text),
            None if plain => (s.code_block_fill, s.code_block_border, s.plain_code_text.or(s.code_text).unwrap_or(s.text)),
            None => (s.code_block_fill, s.code_block_border, s.code_text.unwrap_or(s.text)),
        };
        let spans = if shell.is_some() { Vec::new() } else { self.syntax_spans(code, source) };
        // Texts of `CODE_GROUP_LINES` lines each, so a block still arriving shapes its newest
        // lines and not all of them. A shorter block is one text.
        let body = self.with_reveal(tabbed(code, spans, highlights, (presses, self.strikes(&marks)), s.code_block).font(s.code_block).color(ink)).copy_breaks(2, 2);
        let wide = self.scroll_wide;
        let mut groups = body.line_groups(CODE_GROUP_LINES);
        if let Some(last) = groups.pop() {
            groups.push(self.caret(last, code));
        }
        let body = groups.into_iter().enumerate().map(|(k, g)| {
            let g = self.select(self.wire_presses(g, any));
            let g = if wide { g } else { g.w_full() };
            if k == 0 { g } else { g.key(format!("g{k}")) }
        });
        // `.highlights`' `border: 0.5px` takes its device pixel each side, inside which the padding sits.
        // Code reads left to right in any document, as `markdown.css` isolates it.
        let frame = col().role("code").dir(rune::Direction::Ltr).w_full().radius(6.0).fill(fill).border(s.line, border).border_layout();
        if self.scroll_wide {
            // The text sizes to its longest line inside the viewport, which scrolls under the
            // padding so lines run to the frame's edge.
            let viewport = col().key("scroll").w_full().scroll_x();
            let viewport = if s.wide_fade > 0.0 { viewport.scroll_edge_fade(s.wide_fade) } else { viewport };
            let viewport = viewport.child(col().px(s.code_block_pad[0]).py(s.code_block_pad[1]).children(body));
            return frame.child(match s.code_bar {
                Some(bar) => self.wide_bar(bar.horizontal().room(), viewport),
                None => viewport,
            });
        }
        frame.px(s.code_block_pad[0]).py(s.code_block_pad[1]).children(body)
    }

    /// `viewport`, scrolling sideways, with a scroll bar in `style` that is its own in the window.
    pub(super) fn wide_bar(&self, style: crate::ScrollBarStyle, viewport: El) -> El {
        let n = self.next_wide.get();
        self.next_wide.set(n + 1);
        crate::scroll_bar::wrap_in(style, &format!("{}wide-{}-{n}", self.id, self.current_block_key.borrow()), viewport)
    }

    /// Colour-only spans, so highlighting never moves a glyph: a block streaming in draws its
    /// newest line plain and colours it in place once the line ends.
    pub(super) fn syntax_spans(&self, code: &str, source: Option<&Source>) -> Vec<Span> {
        let Some(src) = source.filter(|_| self.s.highlight) else { return Vec::new() };
        let lang = src.lang.as_deref();
        // The built-in highlighter leaves other languages plain; skip the cache for those.
        if self.highlighter.is_builtin() && !lang.is_some_and(crate::syntax::known) {
            return Vec::new();
        }
        let Some(prefix) = code.get(..src.coloured) else { return Vec::new() };
        self.s.syntax.spans(&self.highlighter.highlight(lang, prefix), 0)
    }

    /// One row per line, so added and removed lines are tinted edge to edge.
    pub(super) fn diff(&self, src: &str, map: Option<&SpanMap>) -> El {
        let s = &self.s;
        let all_marks = self.marks_for(src, map);
        let mut c = col().role("code").dir(rune::Direction::Ltr).w_full().py(s.code_block_pad[1]).radius(6.0).fill(s.code_block_fill).border(s.line, s.code_block_border).border_layout();
        let mut line_start = 0usize;
        let last = src.split('\n').count() - 1;
        for (n, line) in src.split('\n').enumerate() {
            let line_end = line_start + line.len();
            let marks: Vec<(Range<usize>, usize)> = all_marks
                .iter()
                .filter_map(|(r, idx)| {
                    let (a, b) = (r.start.max(line_start), r.end.min(line_end));
                    (a < b).then(|| ((a - line_start)..(b - line_start), *idx))
                })
                .collect();
            let (line_hl, presses) = (self.text_highlights(&marks), self.presses(&marks, &[]));
            let any = !presses.is_empty();
            line_start = line_end + 1;
            let line = line.strip_suffix('\r').unwrap_or(line);
            let header = ["@@", "diff ", "index ", "--- ", "+++ "].iter().any(|p| line.starts_with(p));
            let (tint, color) = match line.as_bytes().first() {
                _ if header => (None, s.diff_hunk),
                Some(b'+') => (Some(s.diff_add), s.diff_add_text),
                Some(b'-') => (Some(s.diff_remove), s.diff_remove_text),
                _ => (None, s.text),
            };
            let shown = if line.is_empty() { " " } else { line };
            let mut r = row()
                .key(format!("l{n}"))
                .w_full()
                .px(s.code_block_pad[0])
                .child(self.select(self.wire_presses(self.with_reveal(tabbed(shown, Vec::new(), line_hl, (presses, self.strikes(&marks)), s.code_block).font(s.code_block).color(color).wrap()), any).w_full()
                    .copy_breaks(if n == 0 { 2 } else { 1 }, if n == last { 2 } else { 1 })));
            if let Some(t) = tint {
                r = r.fill(t);
            }
            c = c.child(r);
        }
        c
    }

    /// A diagram, drawn from its complete lines while its fence is still open; until any read
    /// (or for a type it doesn't draw), its source as a code block. While [`Renderer::streaming`],
    /// a fence whose header may yet draw shows nothing until it does, so the source never
    /// flashes before the diagram.
    pub(super) fn mermaid(&self, src: &str, source: Option<&Source>, map: Option<&SpanMap>, index: usize) -> El {
        #[cfg(not(feature = "mermaid"))]
        let _ = index;
        #[cfg(feature = "mermaid")]
        {
            let open = source.is_some_and(|s| s.open);
            let lines = if open { &src[..source.map_or(src.len(), |s| s.coloured.min(src.len()))] } else { src };
            // The card sits in a bare holder, so a fence that draws nothing yet holds its place
            // and the card mounts into it (fading in) rather than the block turning into one.
            let holder = || col().w_full().transition(rune::Transition::none());
            if let Some(d) = crate::mermaid::diagram(lines, &self.s.diagram, self.streaming, open) {
                let card = d.key("card");
                let card = match &self.diagram_frame {
                    Some(f) => f(card, &DiagramFrame { index, source: lines, open }),
                    None => card,
                };
                return holder().child(card);
            }
            if open && self.streaming && crate::mermaid::may_draw(lines) {
                return holder();
            }
            let shown = col().w_full().child(self.code(src, source, map).key("source"));
            match &self.diagram_source_frame {
                Some(f) if !open => f(shown, &DiagramFrame { index, source: src, open }),
                _ => shown,
            }
        }
        #[cfg(not(feature = "mermaid"))]
        self.code(src, source, map)
    }

    #[allow(clippy::too_many_arguments)]
    fn task_item(&self, (k, n): (usize, usize), indent: f32, item: &[Block], item_maps: &[BlockMap], item_idx: usize, checked: bool) -> El {
        let t = &self.s.task;
        let interactive = self.on_task.is_some() && !self.tasks_read_only;
        let (fill, border) = if checked {
            (t.fill_on, t.border_on)
        } else {
            (t.fill_off, t.border_off)
        };
        let mut control = el()
            .key("control")
            .size(t.size, t.size)
            .ml(t.left)
            .mr(t.gap.unwrap_or(indent - t.left - t.size).max(0.0))
            .mt(t.top.unwrap_or(((self.s.body.line_height - t.size) * 0.5).max(0.0)))
            .center()
            .radius(t.radius)
            .fill(fill)
            .border(t.ring, border)
            .token_motion(t.motion)
            .focus_ring(1.0, 0.0, t.focus_ring)
            .child(icon(t.check, t.size * 0.625).key("mark").color(t.mark).opacity(if checked { 1.0 } else { 0.0 }))
            .a11y_hidden();
        if interactive {
            control = control.hover(if checked { t.hover_on } else { t.hover });
        }
        let mut r = row()
            .key(format!("i{k}"))
            .role("checkbox")
            .label(item_label(item))
            .checked(checked)
            .read_only(!interactive)
            .position_in_set(k + 1, n)
            .focus_selected(checked)
            .focusable(interactive)
            .w_full()
            .child(control)
            .child(self.done_item(checked, || self.blocks_in(item, item_maps, &[], false, true)).grow(1.0).shrink());
        if let Some(on_task) = self.on_task.as_ref().filter(|_| !self.tasks_read_only) {
            let block_key = self.current_block_key.borrow().clone();
            r = on_task(r.cursor(Cursor::Pointer), block_key, item_idx, !checked);
        }
        r
    }

    /// A task item's blocks, drawn by `draw`: a checked one's in the style's done colour and
    /// struck through, when the style sets one.
    fn done_item(&self, checked: bool, draw: impl FnOnce() -> El) -> El {
        let Some(done) = self.s.task.done.filter(|_| checked) else { return draw() };
        let flow = self.flow.get();
        self.flow.set(Flow { ink: Some(done), struck: true, ..flow });
        let drawn = draw();
        self.flow.set(flow);
        drawn
    }

    pub(super) fn block(&self, b: &Block, map: &BlockMap) -> El {
        let s = &self.s;
        let inl_map = match map {
            BlockMap::Inline(m) => Some(m),
            _ => None,
        };
        let code_map = match map {
            BlockMap::Code(m) => Some(m),
            _ => None,
        };
        match b {
            Block::Para(inl) => self.para(inl, inl_map, if self.flow.get().lists > 0 { s.list_font.unwrap_or(s.body) } else { s.body }, self.flow.get().ink.unwrap_or(s.text)),
            Block::Heading(l, inl) => {
                let level = (*l as usize).clamp(1, 4) - 1;
                let ink = s.heading_text[level].unwrap_or(if level == 3 { s.muted } else { s.text });
                self.para(inl, inl_map, s.headings[level], ink).role("heading").level(u32::from(*l))
            }
            Block::Code(code) => {
                let n = self.next_source.get();
                self.next_source.set(n + 1);
                let source = self.sources.get(n).filter(|src| src.len == code.len());
                let card = self.code(code, source, code_map);
                match &self.code_frame {
                    Some(f) => f(card.key("card"), &CodeFrame { index: n, source: code, language: source.and_then(|s| s.lang.as_deref()), open: source.is_some_and(|s| s.open) }),
                    None => card,
                }
            }
            Block::Diff(src) => self.diff(src, code_map),
            Block::FrontMatter(yaml) => {
                let block = text(yaml.as_str()).font(s.code_block).color(s.muted).role("note").label("Front matter").w_full();
                match &self.front_matter_frame {
                    Some(f) => f(block, yaml),
                    None => block,
                }
            }
            Block::Mermaid(src) => {
                let n = self.next_source.get();
                self.next_source.set(n + 1);
                self.mermaid(src, self.sources.get(n).filter(|s| s.len == src.len()), code_map, n)
            }
            Block::Table(t) => {
                let (head_maps, row_maps) = match map {
                    BlockMap::Table { head, rows } => (head.as_slice(), rows.as_slice()),
                    _ => (&[][..], &[][..]),
                };
                // Its first column stands on the side its words start on.
                bidi::directed(self.table(t, head_maps, row_maps), bidi::table_dir(t))
            }
            Block::List { start, items } => {
                let item_maps = match map {
                    BlockMap::List(m) => m.as_slice(),
                    _ => &[][..],
                };
                let flow = self.flow.get();
                let indent = match s.list_nested {
                    Some([bullets, numbers]) if flow.lists > 0 => if start.is_some() { numbers } else { bullets },
                    _ => s.list_indent,
                };
                // Outside markers end a space's width before the text, as a browser sets "1. ".
                let space = match s.list_marker_gap {
                    _ if !s.list_outside => 0.0,
                    Some(gap) => gap,
                    None => rune::text::measure("x x", &[], s.body, None).0 - rune::text::measure("xx", &[], s.body, None).0,
                };
                let mut c = col().role("list").w_full();
                let n = items.len();
                // Between items, their 8 collapsed with the margins passing out of the blocks
                // either side (`collapsed`): an item ending in a quote keeps its 16 below.
                let edge = |item: &[Block], last: bool| match (last, item) {
                    (true, [.., l]) => collapsed(l, s, item.len() - 1, true).1,
                    (false, [f, ..]) => collapsed(f, s, 0, true).0,
                    _ => 0.0,
                };
                let gap = |k: usize| if k == 0 { 0.0 } else { s.list_gap.max(edge(&items[k - 1], true)).max(edge(&items[k], false)) };
                for (k, item) in items.iter().enumerate() {
                    let imap = item_maps.get(k).map_or(&[][..], Vec::as_slice);
                    let task_idx = self.next_task.get();
                    self.next_task.set(task_idx + 1);
                    let task = self.tasks.get(task_idx).copied().flatten();
                    let lead = match (task, start) {
                        (Some((checked, _)), _) => Lead::Task(checked),
                        (None, Some(n)) => Lead::Number(n + k as u64),
                        (None, None) => Lead::Bullet,
                    };
                    self.flow.set(Flow { lists: flow.lists + 1, inset: flow.inset + indent, measure: flow.measure.map(|m| (m - indent).max(0.0)), cols: flow.cols + lead.width(), lead: Some((flow.cols, lead, if k == 0 && flow.lists == 0 { 2 } else { 1 })), struck: false, ..flow });
                    if let Some((checked, marker_offset)) = task {
                        c = c.child(self.task_item((k, n), indent, item, imap, marker_offset, checked).mt(gap(k)));
                        continue;
                    }
                    let marker = match start {
                        Some(n) => format!("{}.", n + k as u64),
                        None => "•".into(),
                    };
                    let font = s.list_font.unwrap_or(s.body);
                    let font = if s.list_tabular && start.is_some() { TextStyle { features: super::table::with_tnum(font.features), ..font } } else { font };
                    let marker = text(marker).font(font).color(s.list_marker.unwrap_or(s.muted)).exact_lines().w(indent);
                    let marker = if s.list_outside { marker.align(Align::End).pr(space) } else { marker.align(Align::Center) };
                    // The item's place in the list says what its marker shows.
                    c = c.child(
                        row()
                            .key(format!("i{k}"))
                            .role("listitem")
                            .position_in_set(k + 1, n)
                            .w_full()
                            .mt(gap(k))
                            .child(self.with_reveal(marker).a11y_hidden())
                            .child(self.blocks_in(item, imap, &[], false, true).grow(1.0).shrink()),
                    );
                }
                self.flow.set(flow);
                // One direction for the whole list, its first item's with any, so every marker
                // stands on one rail; each item's paragraphs still read their own way.
                bidi::directed(c, items.iter().find_map(|item| bidi::blocks_dir(item)))
            }
            Block::Quote(inner) => {
                let inner_maps = match map {
                    BlockMap::Quote(m) => m.as_slice(),
                    _ => &[][..],
                };
                let q = s.quote;
                let flow = self.flow.get();
                let first_task = self.next_task.get();
                self.flow.set(Flow { ink: q.text.or(flow.ink), inset: flow.inset + q.width + q.pad, measure: flow.measure.map(|m| (m - q.width - q.pad).max(0.0)), italic: flow.italic || q.italic, ..flow });
                let drawn = self.blocks(inner, inner_maps, &[], false);
                self.flow.set(flow);
                // The rule stands on the side the quote starts on.
                let quote = bidi::directed(
                    row()
                        .role("blockquote")
                        .w_full()
                        .child(el().w(q.width).h_full().radius((q.width / 2.0).min(1.0)).fill(s.quote_rule))
                        .child(drawn.grow(1.0).shrink().pl(q.pad)),
                    bidi::blocks_dir(inner),
                );
                match &self.quote_frame {
                    Some(f) => f(quote, &QuoteFrame { text: &self.plain_text(inner, first_task, Flow { lead: None, ..flow }), alert: None }),
                    None => quote,
                }
            }
            Block::Alert(alert, inner) => {
                let inner_maps = match map {
                    BlockMap::Quote(m) => m.as_slice(),
                    _ => &[][..],
                };
                let (q, ink) = (s.quote, s.alert(*alert));
                let flow = self.flow.get();
                let first_task = self.next_task.get();
                self.flow.set(Flow { inset: flow.inset + 3.0 + q.pad, measure: flow.measure.map(|m| (m - 3.0 - q.pad).max(0.0)), ..flow });
                let drawn = self.blocks(inner, inner_maps, &[], false);
                self.flow.set(flow);
                let title = row()
                    .gap(8.0)
                    .items_center()
                    .mb(4.0)
                    .child(icon(alert_icon::of(*alert), 16.0).key("icon").color(ink).a11y_hidden())
                    .child(text(alert.title()).font(TextStyle { size: 15.0, weight: 600.0, ..s.body }).color(ink));
                let quote = bidi::directed(
                    row()
                        .role("note")
                        .label(alert.title())
                        .w_full()
                        .child(el().w(3.0).h_full().fill(ink))
                        .child(col().grow(1.0).shrink().py(8.0).pl(q.pad).child(title).child(drawn)),
                    bidi::blocks_dir(inner),
                );
                match &self.quote_frame {
                    Some(f) => f(quote, &QuoteFrame { text: &self.plain_text(inner, first_task, Flow { lead: None, ..flow }), alert: Some(*alert) }),
                    None => quote,
                }
            }
            Block::Details { summary, open, body } => {
                let (at, body_maps) = match map {
                    BlockMap::Details { at, body } => (Some(*at), body.as_slice()),
                    _ => (None, &[][..]),
                };
                let wire = self.details.as_ref().zip(at);
                let shown = wire.as_ref().map_or(true, |(d, at)| (d.open)(*at, *open));
                let drawn = self.blocks(body, body_maps, &[], false);
                let marker = row().w(16.0).items_center().a11y_hidden()
                    .child(icon(if shown { &crate::glyphs::TRIANGLE_DOWN } else { &crate::glyphs::TRIANGLE_RIGHT }, 16.0).key("marker").color(s.muted));
                let mut head = row()
                    .key("summary")
                    .role("button")
                    .label(unpadded(&summary.text))
                    .expanded(shown)
                    .w_full()
                    .child(marker)
                    .child(self.para(summary, None, s.body, s.text).grow(1.0).shrink());
                if let Some(measure) = self.flow.get().measure {
                    head = head.max_w(measure);
                }
                if let Some((d, at)) = wire {
                    head = (d.press)(head.cursor(Cursor::Pointer), at, !shown);
                }
                col().role("group").w_full().child(head).children(shown.then(|| drawn.mt(8.0)))
            }
            // A clear rule is spacing only (`hr { border: none; height: 0 }`).
            Block::Image { src, alt, title } => self.image(src, alt, title),
            Block::Rule if s.rule.a <= 0.0 => el().w_full().h(0.0),
            Block::Rule => el().w_full().h(s.line).fill(s.rule),
        }
    }
}

fn item_label(blocks: &[Block]) -> String {
    let mut out = String::new();
    for b in blocks {
        let text = match b {
            Block::Para(inl) | Block::Heading(_, inl) => unpadded(&inl.text),
            Block::Code(s) | Block::Diff(s) | Block::Mermaid(s) => s.clone(),
            Block::Image { alt, .. } => alt.clone(),
            _ => continue,
        };
        if !text.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&text);
        }
    }
    out
}

pub(super) fn hash_of(v: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}
