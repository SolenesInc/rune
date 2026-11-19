//! `El::end_mark`: a mark after a text's last word, such as the caret of a reply still arriving.
use super::*;

impl El {
    /// Ends a [`text`](crate::text) or [`rich`](crate::rich) element with `mark`, styled by
    /// `spans` (ranges of `mark`): characters that are not the text's own. They draw at
    /// once while the words before them fade in ([`El::reveal`]), so a mark that moves with
    /// each word that arrives never fades in again, and copying leaves them out. Set it last,
    /// after ranges of the text are given and after [`El::line_groups`]. On a field, or with an
    /// empty `mark`, this does nothing.
    ///
    /// ```ignore
    /// rich(reply, spans).reveal(0.18).end_mark("\u{a0}\u{a0}", [Span::new(2, 4).bg(caret, 0.0)])
    /// ```
    pub fn end_mark(mut self, mark: &str, spans: impl IntoIterator<Item = Span>) -> Self {
        if self.cold.input.is_some() || mark.is_empty() {
            return self;
        }
        let Some(t) = &mut self.text else { return self };
        let start = t.content.len();
        t.content.push_str(mark);
        let (start, end) = (start as u32, t.content.len() as u32);
        t.spans.extend(spans.into_iter().map(|s| Span { start: (start + s.start).min(end), end: (start + s.end).min(end), ..s }));
        t.cold.uncopied.push((start, end));
        t.cold.end_mark = end - start;
        t.shape = None;
        self
    }
}

#[cfg(test)]
#[path = "end_mark_tests.rs"]
mod tests;
