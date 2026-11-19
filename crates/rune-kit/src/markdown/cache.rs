//! Parsed documents kept across commits, and growing a stream's parse from its last block.

use super::*;

/// Estimated bytes of parsed documents kept beyond those on screen (see [`budget`]).
const CACHE_BYTES: usize = 16 << 20;

/// How many documents parsed or grown last a cache finds by comparing their text, without
/// hashing it: each stream arriving at once needs one, or its next string parses whole. Eight
/// is more streams than a window shows growing at a time; a ninth costs time, nothing else.
const RECENT: usize = 8;

/// A document's blocks, code sources, tasks, block starts and maps. Each is a `Vec` behind its
/// `Rc` so that a stream's parse grows in place while nothing else holds it.
pub(super) type Doc = (Rc<Vec<Block>>, Rc<Vec<Source>>, Rc<Vec<Option<(bool, usize)>>>, Rc<Vec<usize>>, Rc<Vec<BlockMap>>);

#[derive(Clone)]
pub(super) struct Parsed {
    pub(super) doc: Doc,
    pub(super) cuts: Rc<Vec<Cut>>,
    /// A hash of the source's link reference definitions, `None` without any (see
    /// [`parser::parse_doc_defs`]).
    pub(super) defs: Option<u64>,
    /// A digest of each top-level block's source, from the start of its first line to the
    /// next block's: with `defs`, all that decides how the block parses. `None` for a block
    /// that shares its lines with another (a paragraph split around an image).
    pub(super) digests: Rc<Vec<Option<(u64, u64)>>>,
}

/// [`Parsed::digests`] of the blocks from the `from`-th on, which start at `starts` in `src`.
fn digests<'a>(src: &'a str, starts: &'a [usize], from: usize) -> impl Iterator<Item = Option<(u64, u64)>> + 'a {
    (from..starts.len()).map(move |i| {
        let at = starts[i];
        let end = starts.get(i + 1).copied().unwrap_or(src.len());
        // The document's first block is the only one that can be front matter.
        (at < end).then(|| rune::memo::digest(&(&src[at..end], i == 0)))
    })
}

/// A document parsed or grown last, with the commit that last drew it.
struct Recent {
    src: String,
    parsed: Parsed,
    commit: u64,
}

pub(super) struct Cache {
    /// Documents by their source, for those no longer among `recent`.
    docs: Budget<Rc<str>, Parsed>,
    /// The last [`RECENT`] documents parsed or grown, oldest first. A stream's next string
    /// extends one of these, and is found by its text starting with that one's.
    recent: Vec<Recent>,
}

/// Parsed text is about as long as its source, plus each block's bookkeeping.
fn bytes(src: &str, parsed: &Parsed) -> usize {
    2 * src.len() + 64 * parsed.cuts.len() + 128
}

impl Cache {
    fn new(bytes: usize) -> Self {
        Cache { docs: Budget::new(bytes), recent: Vec::new() }
    }

    /// Keeps `r` as the newest of `recent`; the oldest past [`RECENT`] moves to `docs`.
    fn keep(&mut self, r: Recent) {
        if self.recent.len() == RECENT {
            let old = self.recent.remove(0);
            let bytes = bytes(&old.src, &old.parsed);
            self.docs.insert(old.src.into(), old.parsed, bytes);
        }
        self.recent.push(r);
    }

    /// What `src` parsed to, if it is held.
    #[cfg(test)]
    pub(super) fn peek(&self, src: &str) -> Option<&Parsed> {
        self.recent.iter().find(|r| r.src == src).map(|r| &r.parsed).or_else(|| self.docs.peek(src))
    }

    /// The estimated bytes of every document held.
    #[cfg(test)]
    pub(super) fn bytes(&self) -> usize {
        self.docs.bytes() + self.recent.iter().map(|r| bytes(&r.src, &r.parsed)).sum::<usize>()
    }
}

thread_local! {
    pub(super) static CACHE: RefCell<Cache> = RefCell::new(Cache::new(CACHE_BYTES));
    /// Sources drawn while still arriving ([`Markdown::streaming`]), parsed with their tail
    /// settled (see [`super::stream`]); kept apart, since the same string parses differently.
    pub(super) static STREAMS: RefCell<Cache> = RefCell::new(Cache::new(CACHE_BYTES / 4));
    /// [`CACHE`] and [`STREAMS`] for documents read with smart punctuation, which parse to
    /// other text.
    static SMART: RefCell<Cache> = RefCell::new(Cache::new(CACHE_BYTES));
    static SMART_STREAMS: RefCell<Cache> = RefCell::new(Cache::new(CACHE_BYTES / 4));
}

#[cfg(test)]
thread_local! {
    /// (whole documents parsed, streams parsed from their last block).
    pub(super) static PARSES: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}

/// [`parse`] memoised by source on this thread. Every document the last two commits drew stays
/// cached however many there are; past a memory budget the least recently drawn of the rest go.
pub fn parse_cached(src: &str) -> Rc<Vec<Block>> {
    doc_cached(src).0
}

pub(super) fn doc_cached(src: &str) -> Doc {
    doc_as(src, Reading::LINKED, false)
}

/// [`doc_cached`] for a source still arriving.
#[cfg(test)]
pub(super) fn stream_cached(src: &str) -> Doc {
    doc_as(src, Reading::LINKED, true)
}

/// `src` parsed as `how` says, memoised on this thread; `streaming` while it is still arriving.
pub(super) fn doc_as(src: &str, how: Reading, streaming: bool) -> Doc {
    parsed_as(src, how, streaming).doc
}

/// [`doc_as`] with what the parse knows beside the document.
pub(super) fn parsed_as(src: &str, how: Reading, streaming: bool) -> Parsed {
    let (cache, parse): (&'static std::thread::LocalKey<RefCell<Cache>>, Parse) = match (how.smart, streaming) {
        (false, false) => (&CACHE, whole),
        (false, true) => (&STREAMS, stream::parse_streaming_defs),
        (true, false) => (&SMART, whole),
        (true, true) => (&SMART_STREAMS, stream::parse_streaming_defs),
    };
    cache.with_borrow_mut(|c| cached(c, src, parse, how))
}

type Parse = fn(&str, Reading) -> (ParsedDoc, Option<u64>);

fn whole(src: &str, how: Reading) -> (ParsedDoc, Option<u64>) {
    let (doc, _, defs) = parser::parse_doc_defs(src, how);
    (doc, defs)
}

fn cached(c: &mut Cache, src: &str, parse: Parse, how: Reading) -> Parsed {
    let now = rune::commit_generation();
    if let Some(i) = c.recent.iter().position(|r| r.src == src) {
        let mut r = c.recent.remove(i);
        r.commit = now;
        let parsed = r.parsed.clone();
        c.recent.push(r);
        return parsed;
    }
    // The longest document `src` starts with is the stream it continues.
    let base = (0..c.recent.len())
        .filter(|&i| src.len() > c.recent[i].src.len() && src.starts_with(c.recent[i].src.as_str()))
        .max_by_key(|&i| c.recent[i].src.len());
    if let Some(i) = base {
        let tail = tail(&c.recent[i].parsed, c.recent[i].src.len(), src, parse, how);
        // A stream's earlier string is dead once it grows, unless something drew it this
        // commit: its parse becomes the grown one's, so a long stream holds one parse and
        // copies nothing but its last block.
        let drawn_now = now > 0 && c.recent[i].commit == now;
        let grown = match (tail, drawn_now) {
            (Some(tail), false) => {
                let mut r = c.recent.remove(i);
                splice(&mut r.parsed, tail, src);
                let held = r.src.len();
                r.src.push_str(&src[held..]);
                Some(Recent { commit: now, ..r })
            }
            (Some(tail), true) => {
                let mut parsed = c.recent[i].parsed.clone();
                splice(&mut parsed, tail, src);
                Some(Recent { src: src.to_owned(), parsed, commit: now })
            }
            (None, false) => {
                c.recent.remove(i);
                None
            }
            (None, true) => None,
        };
        if let Some(r) = grown {
            let parsed = r.parsed.clone();
            c.keep(r);
            return parsed;
        }
    }
    if let Some(hit) = c.docs.get(src) {
        return hit.clone();
    }
    #[cfg(test)]
    PARSES.with(|p| p.set((p.get().0 + 1, p.get().1)));
    let ((blocks, sources, tasks, cuts, starts, maps), defs) = parse(src, how);
    let digests = Rc::new(digests(src, &starts, 0).collect());
    let parsed = Parsed { doc: (Rc::new(blocks), Rc::new(sources), Rc::new(tasks), Rc::new(starts), Rc::new(maps)), cuts: Rc::new(cuts), defs, digests };
    c.keep(Recent { src: src.to_owned(), parsed: parsed.clone(), commit: now });
    parsed
}

/// The blocks a stream's next string adds to what `base` parsed, and where they join it.
struct Tail {
    /// How many of the base's cuts stay.
    cuts: usize,
    cut: Cut,
    doc: ParsedDoc,
}

/// The parse of `src` from the last block of `base`, which parsed its first `base_len` bytes:
/// appended text can change only the last block or add blocks after it. Whether a line
/// starts a block can depend on the rest of that line (`|` ending a table that `| 1 |` would
/// continue), so the reparse starts at the last block that starts on a finished line. A
/// reference definition (`[x]: url`) can change links anywhere, so a source that holds one,
/// before the cut or after it, parses whole. A stream's parse may have left its last lines
/// out, so a cut is only ever at or before `base_len`.
fn tail(base: &Parsed, base_len: usize, src: &str, parse: Parse, how: Reading) -> Option<Tail> {
    let i = base.cuts.iter().rposition(|c| c.at <= base_len && src[c.at..base_len].contains('\n'))?;
    let cut = base.cuts[i];
    if cut.at == 0 || cut.blocks >= base.doc.0.len() || base.defs.is_some() {
        return None;
    }
    let (doc, defs) = parse(&src[cut.at..], how);
    if defs.is_some() {
        return None;
    }
    #[cfg(test)]
    PARSES.with(|p| p.set((p.get().0, p.get().1 + 1)));
    Some(Tail { cuts: i, cut, doc })
}

/// Makes `parsed` the parse of `src`: every top-level block before the cut stays where it is,
/// and `tail`'s blocks replace the rest. Held by nothing else, nothing before the cut is
/// copied.
fn splice(parsed: &mut Parsed, tail: Tail, src: &str) {
    let Tail { cuts: kept, cut, doc: (blocks, sources, tasks, cuts, starts, maps) } = tail;
    fn rest<T: Clone>(all: &mut Rc<Vec<T>>, keep: usize, more: impl Iterator<Item = T>) {
        let all = Rc::make_mut(all);
        all.truncate(keep);
        all.extend(more);
    }
    rest(&mut parsed.doc.0, cut.blocks, blocks.into_iter());
    rest(&mut parsed.doc.1, cut.sources, sources.into_iter());
    rest(&mut parsed.doc.2, cut.tasks, tasks.into_iter().map(|t| t.map(|(checked, at)| (checked, at + cut.at))));
    rest(&mut parsed.doc.3, cut.blocks, starts.into_iter().map(|at| at + cut.at));
    rest(&mut parsed.doc.4, cut.blocks, maps.into_iter().map(|m| m.shift_src(cut.at)));
    // The block before the cut ends where the cut is, as it did, so its digest holds.
    let new: Vec<_> = digests(src, &parsed.doc.3, cut.blocks).collect();
    rest(&mut parsed.digests, cut.blocks, new.into_iter());
    rest(&mut parsed.cuts, kept, cuts.into_iter().map(|c| Cut {
        at: c.at + cut.at,
        blocks: c.blocks + cut.blocks,
        sources: c.sources + cut.sources,
        tasks: c.tasks + cut.tasks,
    }));
}
