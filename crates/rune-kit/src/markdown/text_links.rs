//! Words an app links in prose ([`Markdown::text_links`]).

use super::*;

type FindLinks = dyn Fn(&str) -> Vec<(Range<usize>, String)>;
pub(super) type TextLinks = Rc<FindLinks>;

impl Markdown {
    /// Links the words `f` finds in prose, as autolinks link bare URLs: `f` reads each run of
    /// text outside inline code, code blocks and links, and returns the byte ranges in that run
    /// to link, in order, each with its destination. A range that is empty, overlaps the one
    /// before it, leaves the run or cuts a character stays text. The links behave as written
    /// ones do: [`Self::on_link`], [`Self::link_help`] and [`Self::link_style`] see their
    /// destinations.
    pub fn text_links(mut self, f: impl Fn(&str) -> Vec<(Range<usize>, String)> + 'static) -> Self {
        self.text_links = Some(Rc::new(f));
        self
    }
}

pub(super) fn link_blocks(blocks: &mut [Block], f: &FindLinks) {
    for block in blocks {
        match block {
            Block::Para(inl) | Block::Heading(_, inl) => link_inline(inl, f),
            Block::List { items, .. } => items.iter_mut().for_each(|item| link_blocks(item, f)),
            Block::Quote(inner) | Block::Alert(_, inner) => link_blocks(inner, f),
            Block::Table(table) => table.head.iter_mut().chain(table.rows.iter_mut().flatten()).for_each(|inl| link_inline(inl, f)),
            Block::Code(_) | Block::FrontMatter(_) | Block::Rule | Block::Diff(_) | Block::Mermaid(_) | Block::Image { .. } => {}
        }
    }
}

fn link_inline(inl: &mut Inline, f: &FindLinks) {
    let mut found = Vec::new();
    for (a, b) in autolink::prose(inl, &[]) {
        let run = &inl.text[a..b];
        let mut last = 0;
        for (range, dest) in f(run) {
            let fits = range.start < range.end
                && range.start >= last
                && range.end <= run.len()
                && run.is_char_boundary(range.start)
                && run.is_char_boundary(range.end);
            if fits {
                found.push((a + range.start, a + range.end, dest));
                last = range.end;
            }
        }
    }
    autolink::add_links(inl, found);
}
