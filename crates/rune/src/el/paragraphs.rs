//! `El::dir_by_paragraph`: each paragraph of a text in the direction of its own words, as CSS's
//! `unicode-bidi: plaintext` lays a block's paragraphs out.
use super::*;

impl El {
    /// Lays each paragraph of this text (a line a newline ends) out in the direction of its own
    /// first strong character, as CSS's `unicode-bidi: plaintext` does: right to left when that's
    /// Arabic or Hebrew, else left to right, left to right too when it has none, whatever
    /// direction the element itself lays out in. Each paragraph's lines start at its own start
    /// edge. The element's box, padding and place stay where its direction puts them.
    ///
    /// Call it last, after the text's spans, links, pills, highlights and copy settings. A text
    /// whose paragraphs all take one direction stays one text. One whose paragraphs differ
    /// becomes a column of one text per paragraph, which a selection crosses as one inside a
    /// [`El::selection_group`] and copies joined by newlines. A multi-line text field stays one
    /// field and lays each paragraph out in its own direction as it's edited, its mention pills
    /// isolated (`@src/app.ts` stays left to right on an Arabic line), as a `contenteditable`
    /// with `unicode-bidi: plaintext` does; its own direction (`El::dir(Direction::Auto)`)
    /// comes from its text outside its pills, and left to right while it's empty, its
    /// placeholder laid out in the placeholder's own. A secure, live, truncated or
    /// line-clamped text, or a one-line field, stays one text in its first paragraph's direction.
    pub fn dir_by_paragraph(mut self) -> El {
        let Some(t) = self.text.as_mut() else { return self };
        t.dir = step_text_dir(t.dir, TextDirEvent::ByParagraph);
        t.shape = None;
        if self.cold.input.is_some() {
            return self;
        }
        // Each paragraph's direction skips its isolates, and its piece takes their marks.
        t.apply_isolates();
        let paragraphs = paragraphs(&t.content);
        // A paragraph with no strong character reads the way the one before it does, and the
        // first ones the way the element does (None here, as it isn't flattened yet).
        let dirs: Vec<Option<Direction>> = paragraphs
            .iter()
            .scan(None, |last, &(a, b)| {
                *last = Direction::of(&t.content[a..b]).or(*last);
                Some(*last)
            })
            .collect();
        let whole = t.redacted || t.cold.live.is_some() || t.cold.advance_in.is_some() || t.truncate || t.truncate_fade || t.max_lines > 0;
        if whole || dirs.iter().all(|&d| d == dirs[0]) {
            return self;
        }
        let Some(t) = self.text.take() else { return self };
        let cursor = self.handlers.cursor;
        let last = paragraphs.len() - 1;
        let pieces: Vec<El> = paragraphs.iter().enumerate().map(|(k, &range)| {
            let mut e = text(String::new()).key(format!("p{k}"));
            // A line of digits or punctuation under an Arabic one reads right to left with it.
            if let Some(d) = dirs[k].filter(|_| Direction::of(&t.content[range.0..range.1]).is_none()) {
                e = e.dir(d);
            }
            let links = paragraph(&t, k, range, k == last, &mut e);
            if let Some(c) = cursor { e = e.cursor(c); }
            if links { e.cold_mut().focus.enabled = true; }
            e
        }).collect();
        // The links' Tab stops went with them.
        if !t.cold.links.is_empty() && let Some(c) = self.cold.get_mut_if_some() { c.focus.enabled = false; }
        if self.role == RoleTag::TEXT { self.role = RoleTag::GROUP; }
        self.style.set_flex_direction(FlexDirection::Column);
        let kept = std::mem::take(&mut self.children);
        self.children = pieces.into_iter().chain(kept).collect();
        self
    }

    /// This text as texts of `lines` lines each (a line being what a newline ends), to stack
    /// in a column in place of it: a change to one line then shapes that line's text, not the
    /// whole one, so a long text that grows a line at a time (a code block streaming in) costs
    /// what its newest lines cost. The first is this element with the first lines; the rest are
    /// new texts with the same font, colour, wrapping, reveal and copy settings. Each takes
    /// the spans, links, highlights, strikes and uncopied ranges over its own lines, and they
    /// copy joined by the newlines between them, as [`El::dir_by_paragraph`]'s paragraphs do.
    /// Stacked with no gap they keep the one text's line pitch.
    ///
    /// Call it on the text itself, before its size, key, selection and handlers: each text
    /// returned takes its own. A text of `lines` lines or fewer comes back as it is, alone, and
    /// growing past that adds texts after it without changing it. So does a secure, live,
    /// truncated or line-clamped text, one that is a field's, and one with pills or blocks.
    /// The bytes of `text` each text [`El::line_groups`] makes of it holds, the newline between
    /// two groups left out. A newline that ends the text stays in the last group, which lays it
    /// out as the one text did.
    pub fn line_group_bytes(text: &str, lines: usize) -> Vec<std::ops::Range<usize>> {
        let mut groups = Vec::new();
        let (mut start, mut seen) = (0, 0);
        for (i, _) in text.bytes().enumerate().filter(|&(_, b)| b == b'\n') {
            seen += 1;
            if seen % lines == 0 && i + 1 < text.len() {
                groups.push(start..i);
                start = i + 1;
            }
        }
        groups.push(start..text.len());
        groups
    }

    pub fn line_groups(mut self, lines: usize) -> Vec<El> {
        let Some(t) = self.text.as_mut() else { return vec![self] };
        let whole = lines == 0 || self.cold.input.is_some() || t.redacted || t.truncate || t.truncate_fade || t.max_lines > 0 || t.source.is_some()
            || t.cold.live.is_some() || t.cold.advance_in.is_some()
            || !(t.cold.blocks.is_empty() && t.cold.soft_breaks.is_empty() && t.cold.pills.is_empty() && t.cold.isolates.is_empty());
        if whole {
            return vec![self];
        }
        let groups: Vec<(u32, u32)> = El::line_group_bytes(&t.content, lines).into_iter().map(|g| (g.start as u32, g.end as u32)).collect();
        if groups.len() == 1 {
            return vec![self];
        }
        // The parts of `s..e` in each group it reaches, as that group's bytes.
        let parts = |s: u32, e: u32| {
            let first = groups.partition_point(|g| g.1 <= s);
            groups[first..].iter().enumerate().take_while(move |(_, g)| g.0 < e).filter_map(move |(k, g)| {
                let (a, b) = (s.max(g.0), e.min(g.1));
                (a < b).then_some((first + k, a - g.0, b - g.0))
            })
        };
        let content = std::mem::take(&mut t.content);
        let spans = std::mem::take(&mut t.spans);
        let cold = (!t.cold.is_empty()).then(|| {
            let c = &mut *t.cold;
            (std::mem::take(&mut c.links), std::mem::take(&mut c.uncopied), std::mem::take(&mut c.rings), std::mem::take(&mut c.strikes),
                std::mem::take(&mut c.highlights), c.copy_breaks, c.copy_prefix.take())
        });
        t.exact_lines = true;
        t.shape = None;
        let last = groups.len() - 1;
        let mut pieces: Vec<TextSpec> = groups.iter().map(|&(a, b)| {
            let mut p = (**t).clone();
            p.content = content[a as usize..b as usize].to_owned();
            p
        }).collect();
        for s in spans {
            for (k, start, end) in parts(s.start, s.end) {
                pieces[k].spans.push(crate::text::Span { start, end, ..s.clone() });
            }
        }
        if let Some((links, uncopied, rings, strikes, highlights, breaks, prefix)) = cold {
            for l in links {
                for (k, start, end) in parts(l.start, l.end) {
                    pieces[k].cold.links.push(Link { start, end, ..l.clone() });
                }
            }
            for (s, e) in uncopied {
                for (k, s, e) in parts(s, e) { pieces[k].cold.uncopied.push((s, e)); }
            }
            for (s, e, c, w) in rings {
                for (k, s, e) in parts(s, e) { pieces[k].cold.rings.push((s, e, c, w)); }
            }
            for (s, e, c) in strikes {
                for (k, s, e) in parts(s, e) { pieces[k].cold.strikes.push((s, e, c)); }
            }
            for h in highlights {
                for (k, start, end) in parts(h.start, h.end) {
                    pieces[k].cold.highlights.push(crate::text::TextHighlight { start, end, ..h.clone() });
                }
            }
            for (k, p) in pieces.iter_mut().enumerate() {
                p.cold.copy_breaks = [if k == 0 { breaks[0] } else { 1 }, if k == last { breaks[1] } else { 1 }];
            }
            pieces[0].cold.copy_prefix = prefix;
        }
        let mut pieces = pieces.into_iter();
        self.text = pieces.next().map(crate::cold::SharedBox::new);
        std::iter::once(self).chain(pieces.map(|p| {
            let mut e = text(String::new());
            e.text = Some(crate::cold::SharedBox::new(p));
            e
        })).collect()
    }
}

/// The byte ranges of `content`'s paragraphs, the separators between them left out: a UAX #9
/// paragraph separator (a newline) ends each but the last, CR LF as one.
fn paragraphs(content: &str) -> Vec<(usize, usize)> {
    use icu_properties::{CodePointMapData, props::BidiClass};
    let classes = CodePointMapData::<BidiClass>::new();
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = content.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if classes.get(c) != BidiClass::ParagraphSeparator {
            continue;
        }
        let end = match chars.peek() {
            Some(&(j, '\n')) if c == '\r' => {
                chars.next();
                j + 1
            }
            _ => i + c.len_utf8(),
        };
        out.push((start, i));
        start = end;
    }
    out.push((start, content.len()));
    out
}

/// Fills `e` with paragraph `k` of `t`, bytes `a..b`: its words and the spans, links, pills,
/// highlights and marks over them, copying with a newline either side but where `t` began or
/// ended. Whether it has links.
fn paragraph(t: &TextSpec, k: usize, (a, b): (usize, usize), last: bool, e: &mut El) -> bool {
    let (a32, b32) = (a as u32, b as u32);
    let clip = |s: u32, e: u32| {
        let (s, e) = (s.max(a32), e.min(b32));
        (s < e).then(|| (s - a32, e - a32))
    };
    let mut p = t.clone();
    p.content = t.content[a..b].to_owned();
    p.spans = t.spans.iter().filter_map(|s| clip(s.start, s.end).map(|(start, end)| crate::text::Span { start, end, ..s.clone() })).collect();
    // A plain text's paragraphs share its empty cold fields.
    if !t.cold.is_empty() {
        let cold = &mut *p.cold;
        cold.links = t.cold.links.iter().filter_map(|l| clip(l.start, l.end).map(|(start, end)| Link { start, end, ..l.clone() })).collect();
        cold.uncopied = t.cold.uncopied.iter().filter_map(|&(s, e)| clip(s, e)).collect();
        cold.rings = t.cold.rings.iter().filter_map(|&(s, e, c, w)| clip(s, e).map(|(s, e)| (s, e, c, w))).collect();
        cold.strikes = t.cold.strikes.iter().filter_map(|&(s, e, c)| clip(s, e).map(|(s, e)| (s, e, c))).collect();
        cold.highlights = t.cold.highlights.iter().filter_map(|h| clip(h.start, h.end).map(|(start, end)| crate::text::TextHighlight { start, end, ..h.clone() })).collect();
        // A chip lies inside one line; one across a newline has no paragraph to sit in.
        cold.pills = t.cold.pills.iter().filter(|c| a32 <= c.start && c.end <= b32).map(|c| crate::text::PillChip { start: c.start - a32, end: c.end - a32, ..c.clone() }).collect();
        cold.blocks = t.cold.blocks.get(k).copied().into_iter().collect();
        cold.soft_breaks.clear();
        cold.copy_breaks = [if k == 0 { t.cold.copy_breaks[0] } else { 1 }, if last { t.cold.copy_breaks[1] } else { 1 }];
        cold.copy_prefix = if k == 0 { t.cold.copy_prefix.clone() } else { None };
    }
    // Fractional line heights, so the stacked paragraphs keep the one text's line pitch.
    p.exact_lines = true;
    p.shape = None;
    let links = !p.cold.links.is_empty();
    e.text = Some(crate::cold::SharedBox::new(p));
    links
}
