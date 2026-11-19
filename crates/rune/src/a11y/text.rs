//! Text inside the accessibility tree: a text field's lines as text runs with each
//! character's place, so a reader moves by character, word and line and a magnifier follows
//! the caret; a selectable text's lines likewise; and a text's inline links as link nodes.

use accesskit::{
    Action as Ak, Node as AkNode, NodeId as AkId, Rect as AkRect, Role, TextPosition, TextSelection,
};

use super::a11y_tree::{line_id, link_id, round};
use super::*;
use crate::text::TextFlow;

/// Text longer than this gets no per-character runs (a reader still gets its whole value).
const RUN_MAX_BYTES: usize = 64 * 1024;
/// Drawn text with this many runs or more gets none: past it, run ids run into links'.
const DRAWN_LINES_MAX: usize = super::a11y_tree::LINES_MAX;
/// The most characters one text run holds: AccessKit indexes a run's characters by `u8`, so a
/// longer line is several runs that carry on one another.
const RUN_CHARS_MAX: usize = u8::MAX as usize;

/// One text run: its bytes in the drawn text, each character's place in points, and whether
/// the next run carries on the same line.
pub(crate) struct Line {
    pub range: std::ops::Range<usize>,
    pub rect: [f32; 4],
    pub lengths: Vec<u8>,
    pub positions: Vec<f32>,
    pub widths: Vec<f32>,
    pub word_starts: Vec<u8>,
    pub continues: bool,
}

/// One whole line before it is cut into runs; word starts index its characters.
struct WholeLine {
    range: std::ops::Range<usize>,
    rect: [f32; 4],
    lengths: Vec<u8>,
    positions: Vec<f32>,
    widths: Vec<f32>,
    word_starts: Vec<usize>,
}

impl WholeLine {
    fn empty(at: usize, rect: [f32; 4]) -> WholeLine {
        WholeLine {
            range: at..at,
            rect,
            lengths: Vec::new(),
            positions: Vec::new(),
            widths: Vec::new(),
            word_starts: Vec::new(),
        }
    }

    /// The line as runs of at most [`RUN_CHARS_MAX`] characters, each placed where its first
    /// character is drawn.
    fn push_runs(self, out: &mut Vec<Line>) {
        let n = self.lengths.len();
        let mut first = 0;
        let mut start = self.range.start;
        loop {
            let end = (first + RUN_CHARS_MAX).min(n);
            let bytes: usize = self.lengths[first..end].iter().map(|&b| b as usize).sum();
            let range = start..if end == n {
                self.range.end
            } else {
                start + bytes
            };
            let x = self.positions.get(first).copied().unwrap_or(0.0);
            let right = if end == n {
                self.rect[2]
            } else {
                self.positions.get(end).copied().unwrap_or(self.rect[2])
            };
            let rect = if first == 0 && end == n {
                self.rect
            } else {
                [self.rect[0] + x, self.rect[1], right - x, self.rect[3]]
            };
            let positions = self
                .positions
                .get(first..end)
                .map(|p| p.iter().map(|p| p - x).collect())
                .unwrap_or_default();
            let widths = self
                .widths
                .get(first..end)
                .map(<[f32]>::to_vec)
                .unwrap_or_default();
            let word_starts = self
                .word_starts
                .iter()
                .filter(|&&w| first <= w && w < end)
                .map(|&w| (w - first) as u8)
                .collect();
            out.push(Line {
                range: range.clone(),
                rect,
                lengths: self.lengths[first..end].to_vec(),
                positions,
                widths,
                word_starts,
                continues: end < n,
            });
            if end == n {
                return;
            }
            first = end;
            start = range.end;
        }
    }
}

/// `content`'s lines from `layout` (device px at `s`), placed from `origin` in points. A
/// by-paragraph field's newline is the last character of the line it ends, as in one layout.
pub(crate) fn lines(layout: TextFlow<'_>, content: &str, s: f32, origin: [f32; 2]) -> Vec<Line> {
    let mut whole = Vec::new();
    for p in layout.pieces() {
        let first = whole.len();
        piece_lines(p, content, s, origin, &mut whole);
        if p.is_whole() {
            continue;
        }
        if whole.len() == first {
            whole.push(WholeLine::empty(
                p.start,
                [origin[0], origin[1] + p.y / s, 0.0, p.height / s],
            ));
        }
        if content.as_bytes().get(p.end) == Some(&b'\n')
            && let Some(l) = whole.last_mut()
        {
            l.range.end += 1;
            l.lengths.push(1);
            l.positions.push(l.rect[2]);
            l.widths.push(0.0);
        }
    }
    let mut out = Vec::new();
    for l in whole {
        l.push_runs(&mut out);
    }
    out
}

fn piece_lines(
    p: crate::text::Piece<'_>,
    content: &str,
    s: f32,
    origin: [f32; 2],
    out: &mut Vec<WholeLine>,
) {
    for line in p.layout.lines() {
        let m = line.metrics();
        let range = p.content(line.text_range().start)..p.content(line.text_range().end);
        if range.end > content.len() {
            break;
        }
        let (mut lengths, mut positions, mut widths, mut word_starts) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut x = m.offset;
        let mut at = range.start;
        for run in line.runs() {
            for cluster in run.clusters() {
                let r = cluster.text_range();
                // A pill's isolate marks hold no content bytes.
                let r = p.content(r.start)..p.content(r.end);
                if r.start < at || r.end > range.end || r.is_empty() {
                    continue;
                }
                // Bytes a cluster skipped (an ignorable) join the character before.
                if r.start > at
                    && let Some(last) = lengths.last_mut()
                {
                    *last = (*last as usize + r.start - at).min(255) as u8;
                }
                if cluster.is_word_boundary() && !lengths.is_empty() {
                    word_starts.push(lengths.len());
                }
                let adv = cluster.advance();
                lengths.push((r.end - r.start).min(255) as u8);
                positions.push((x - m.offset) / s);
                widths.push(adv / s);
                x += adv;
                at = r.end;
            }
        }
        if at < range.end {
            match lengths.last_mut() {
                Some(last) => *last = (*last as usize + range.end - at).min(255) as u8,
                None => {
                    lengths.push((range.end - at).min(255) as u8);
                    positions.push(0.0);
                    widths.push(0.0);
                }
            }
        }
        let top = (p.y + m.block_min_coord) / s;
        let height = (m.block_max_coord - m.block_min_coord) / s;
        let rect = [
            origin[0] + m.offset / s,
            origin[1] + top,
            (x - m.offset) / s,
            height,
        ];
        out.push(WholeLine {
            range,
            rect,
            lengths,
            positions,
            widths,
            word_starts,
        });
    }
}

/// An element's own drawn text as runs, one or more per drawn line, without geometry, and
/// its caret or selection in them: the drawn rows are the app's, so only the characters and
/// where the app wraps them are known.
fn drawn_runs(id: NodeId, drawn: &crate::el::DrawnText, out: &mut Vec<(AkId, AkNode)>) {
    use unicode_segmentation::UnicodeSegmentation;
    let me = out.len() - 1;
    let mut lines = Vec::new();
    let mut start = 0;
    let mut wraps = drawn.wraps.iter().copied().peekable();
    let mut l = WholeLine::empty(0, [0.0; 4]);
    let mut in_word = false;
    for g in drawn.text.graphemes(true) {
        while wraps.next_if(|&w| w < start).is_some() {}
        if wraps.next_if(|&w| w == start).is_some() && !l.lengths.is_empty() {
            std::mem::replace(&mut l, WholeLine::empty(start, [0.0; 4])).push_runs(&mut lines);
        }
        let word = g.chars().next().is_some_and(char::is_alphanumeric);
        if word && !in_word {
            l.word_starts.push(l.lengths.len());
        }
        in_word = word;
        l.lengths.push(g.len().min(255) as u8);
        start += g.len();
        l.range.end = start;
        if g.ends_with('\n') {
            std::mem::replace(&mut l, WholeLine::empty(start, [0.0; 4])).push_runs(&mut lines);
        }
    }
    l.push_runs(&mut lines);
    if lines.len() >= DRAWN_LINES_MAX {
        return;
    }
    for (k, l) in lines.iter().enumerate() {
        let run = text_run(id, &lines, k, drawn.text.get(l.range.clone()).unwrap_or(""));
        out[me].1.push_child(line_id(id, k));
        out.push((line_id(id, k), run));
    }
    let at = |byte: usize| {
        position(&lines, byte.min(drawn.text.len())).map(|(k, c)| TextPosition {
            node: line_id(id, k),
            character_index: c,
        })
    };
    if let (Some(anchor), Some(focus)) = (at(drawn.anchor), at(drawn.focus)) {
        out[me]
            .1
            .set_text_selection(TextSelection { anchor, focus });
    }
}

/// Run `k` of `lines` with its characters, word starts and the runs it carries on, without
/// geometry.
fn text_run(id: NodeId, lines: &[Line], k: usize, value: &str) -> AkNode {
    let l = &lines[k];
    let mut run = AkNode::new(Role::TextRun);
    run.set_value(value);
    run.set_character_lengths(l.lengths.clone());
    run.set_word_starts(l.word_starts.clone());
    if l.continues {
        run.set_next_on_line(line_id(id, k + 1));
    }
    if k > 0 && lines[k - 1].continues {
        run.set_previous_on_line(line_id(id, k - 1));
    }
    run
}

/// The line and character index of drawn byte `byte`.
fn position(lines: &[Line], byte: usize) -> Option<(usize, usize)> {
    let last = lines.len().checked_sub(1)?;
    let k = lines
        .iter()
        .position(|l| byte < l.range.end)
        .unwrap_or(last);
    let l = &lines[k];
    let mut at = l.range.start;
    for (c, len) in l.lengths.iter().enumerate() {
        if byte < at + *len as usize {
            return Some((k, c));
        }
        at += *len as usize;
    }
    Some((k, l.lengths.len()))
}

/// Drawn byte at line `k`'s character `c`.
pub(crate) fn byte_at(lines: &[Line], k: usize, c: usize) -> Option<usize> {
    let l = lines.get(k)?;
    Some(l.range.start + l.lengths.iter().take(c).map(|&n| n as usize).sum::<usize>())
}

impl RuntimeCore {
    /// Node `i`'s text as AccessKit sees it: the lines of a field or selectable text, and the
    /// links of any text, pushed after it as its children.
    pub(super) fn a11y_text_nodes(
        &self,
        i: usize,
        w: &a11y_tree::Walk,
        out: &mut Vec<(AkId, AkNode)>,
    ) {
        let n = &self.nodes[i];
        if let Some(drawn) = self.a11y_own_text(i) {
            drawn_runs(n.id, &drawn, out);
            return;
        }
        let Some(spec) = n.text.as_deref() else {
            return;
        };
        let me = out.len() - 1;
        let runs = n.input.is_some() || (spec.selectable && !n.is_secure());
        if runs && let Some(lines) = self.a11y_lines(i) {
            for (k, l) in lines.iter().enumerate() {
                let id = line_id(n.id, k);
                let value = self
                    .a11y_drawn_text(i)
                    .map(|t| t.get(l.range.clone()).unwrap_or("").to_owned())
                    .unwrap_or_default();
                let mut run = text_run(n.id, &lines, k, &value);
                run.set_bounds(AkRect {
                    x0: round(l.rect[0] + w.shift[0]),
                    y0: round(l.rect[1] + w.shift[1]),
                    x1: round(l.rect[0] + l.rect[2] + w.shift[0]),
                    y1: round(l.rect[1] + l.rect[3] + w.shift[1]),
                });
                run.set_character_positions(l.positions.clone());
                run.set_character_widths(l.widths.clone());
                out[me].1.push_child(id);
                out.push((id, run));
            }
            if let Some(sel) = self.a11y_selection(i, &lines) {
                out[me].1.set_text_selection(sel);
            }
        }
        if spec.cold.links.is_empty() || spec.redacted {
            return;
        }
        let boxes = self.link_boxes(i, self.drawn_box(i));
        for (k, l) in spec.cold.links.iter().enumerate() {
            let Some(r) =
                boxes
                    .iter()
                    .filter(|(j, _)| *j == k)
                    .fold(None, |u: Option<[f32; 4]>, (_, r)| {
                        Some(u.map_or(*r, |u| {
                            [
                                u[0].min(r[0]),
                                u[1].min(r[1]),
                                u[2].max(r[2]),
                                u[3].max(r[3]),
                            ]
                        }))
                    })
            else {
                continue;
            };
            let role = crate::a11y::roles::map(
                l.role,
                &crate::a11y::roles::RoleCtx {
                    pressable: true,
                    focusable: true,
                    input: false,
                    multiline: false,
                    secure: false,
                    scrolls: false,
                    modal: false,
                    default_button: false,
                    labelled: true,
                    keyed: true,
                    has_text: true,
                    image: false,
                    drawn_text: false,
                },
            )
            .unwrap_or(Role::Link);
            let mut link = AkNode::new(role);
            let words = spec.source_slice(l.start, l.end).unwrap_or("");
            link.set_label(l.label.as_deref().unwrap_or(words));
            link.set_author_id(l.id.as_str());
            if role == Role::Link && l.id.contains("://") {
                link.set_url(l.id.as_str());
            }
            link.set_bounds(AkRect {
                x0: round(r[0] + w.shift[0]),
                y0: round(r[1] + w.shift[1]),
                x1: round(r[2] + w.shift[0]),
                y1: round(r[3] + w.shift[1]),
            });
            if self.allowed(i) {
                if spec.cold.on_link.is_some() {
                    link.add_action(Ak::Click);
                }
                if self.can_focus(i) {
                    link.add_action(Ak::Focus);
                }
                if spec.cold.on_link_secondary.is_some() {
                    link.add_action(Ak::ShowContextMenu);
                }
            }
            let id = link_id(n.id, k);
            out[me].1.push_child(id);
            out.push((id, link));
        }
    }

    /// The text node `i` draws itself ([`crate::El::a11y_text_from`]), read from the state now.
    pub(super) fn a11y_own_text(&self, i: usize) -> Option<crate::DrawnText> {
        let own = self.nodes[i].handlers.get()?.own_text.as_ref()?;
        own(self.app.states_ref())
    }

    /// What node `i` draws as text: a field's display text (its mask when secure), else its
    /// content.
    pub(super) fn a11y_drawn_text(&self, i: usize) -> Option<String> {
        let n = &self.nodes[i];
        match n.input.as_deref() {
            Some(input) => Some(input.edit.display().0),
            None => n.text.as_deref().map(|t| t.content.clone()),
        }
    }

    /// Node `i`'s laid-out lines, in points in its scroll viewport's content space (before the
    /// walk's shift).
    pub(super) fn a11y_lines(&self, i: usize) -> Option<Vec<Line>> {
        let n = &self.nodes[i];
        let (spec, layout) = (n.text.as_deref()?, n.text_layout.as_ref()?.view());
        if n.elided.is_some() {
            return None;
        }
        let content = self.a11y_drawn_text(i)?;
        if content.len() > RUN_MAX_BYTES {
            return None;
        }
        let s = self.env.scale;
        let r = self.drawn[i];
        let (x0, x1) = (r[0] + n.inset[0], r[0] + r[2] - n.inset[2]);
        let mut x = text_x(spec, x0, x1, layout.full_width() / s, layout.flow());
        if let Some(input) = n.input.as_deref()
            && input.single_line
        {
            x -= self.input_scroll.get(&n.path).copied().unwrap_or(0.0);
        }
        Some(lines(layout, &content, s, [x, r[1] + n.inset[1]]))
    }

    /// A field's caret or selection, in its lines.
    fn a11y_selection(&self, i: usize, lines: &[Line]) -> Option<TextSelection> {
        let n = &self.nodes[i];
        let input = n.input.as_deref()?;
        let edit = &input.edit;
        let (anchor, focus) = if edit.preedit().is_some() {
            let caret = caret_index(edit, &self.a11y_drawn_text(i)?);
            (caret, caret)
        } else {
            (
                edit.display_index(edit.anchor),
                edit.display_index(edit.focus),
            )
        };
        let at = |byte: usize| {
            position(lines, byte).map(|(k, c)| TextPosition {
                node: line_id(n.id, k),
                character_index: c,
            })
        };
        Some(TextSelection {
            anchor: at(anchor)?,
            focus: at(focus)?,
        })
    }

    /// The value and state a text field adds to its node.
    pub(super) fn a11y_input(&self, _i: usize, input: &crate::el::InputSpec, node: &mut AkNode) {
        if !input.placeholder.is_empty() {
            node.set_placeholder(input.placeholder.as_str());
        }
        if !input.edit.is_secure() {
            node.set_value(input.edit.text.as_str());
        }
        if input.access.is_none() {
            node.set_read_only();
        }
    }
}
