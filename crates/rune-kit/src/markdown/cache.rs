//! Parsed documents kept across commits, and growing a stream's parse from its last block.

use super::*;

/// Estimated bytes of parsed documents kept beyond those on screen (see [`budget`]).
const CACHE_BYTES: usize = 16 << 20;

pub(super) type Doc = (Rc<Vec<Block>>, Rc<[Source]>, Rc<[Option<(bool, usize)>]>, Rc<[usize]>, Rc<[BlockMap]>);

#[derive(Clone)]
pub(super) struct Parsed {
    pub(super) doc: Doc,
    pub(super) cuts: Rc<[Cut]>,
    /// A hash of the source's link reference definitions, `None` without any (see
    /// [`parser::parse_doc_defs`]).
    pub(super) defs: Option<u64>,
    /// A digest of each top-level block's source, from the start of its first line to the
    /// next block's: with `defs`, all that decides how the block parses. `None` for a block
    /// that shares its lines with another (a paragraph split around an image).
    pub(super) digests: Rc<[Option<(u64, u64)>]>,
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

pub(super) struct Cache {
    pub(super) docs: Budget<Rc<str>, Parsed>,
    /// The source parsed last, which a stream's next string extends.
    pub(super) last: Option<Rc<str>>,
}

impl Cache {
    fn new(bytes: usize) -> Self {
        Cache { docs: Budget::new(bytes), last: None }
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
    if let Some(hit) = c.docs.get(src) {
        return hit.clone();
    }
    let grown = c.last.clone().filter(|last| src.len() > last.len() && src.starts_with(&**last));
    let parsed = grown.as_ref().and_then(|last| extend(c.docs.peek(&**last)?, last.len(), src, parse, how)).unwrap_or_else(|| {
        #[cfg(test)]
        PARSES.with(|p| p.set((p.get().0 + 1, p.get().1)));
        let ((blocks, sources, tasks, cuts, starts, maps), defs) = parse(src, how);
        let digests = digests(src, &starts, 0).collect();
        Parsed { doc: (Rc::new(blocks), sources.into(), tasks.into(), starts.into(), maps.into()), cuts: cuts.into(), defs, digests }
    });
    // A stream's earlier string is dead once it grows, unless something drew it this
    // commit; left in, a long stream would fill the budget with its prefixes.
    if let Some(last) = grown
        && !c.docs.used_now(&*last)
    {
        c.docs.remove(&*last);
    }
    let key: Rc<str> = src.into();
    // Parsed text is about as long as its source, plus each block's bookkeeping.
    let bytes = 2 * src.len() + 64 * parsed.cuts.len() + 128;
    c.docs.insert(key.clone(), parsed.clone(), bytes);
    c.last = Some(key);
    parsed
}

/// The parse of `src`, which extends the `base_len` bytes `base` parsed, reusing every
/// top-level block before `base`'s last: appended text can change only the last block or add
/// blocks after it. Whether a line starts a block can depend on the rest of that line (`|`
/// ending a table that `| 1 |` would continue), so the reparse starts at the last block that
/// starts on a finished line. A reference definition (`[x]: url`) can change links anywhere,
/// so a source that holds one, before the cut or after it, parses whole. A stream's parse may
/// have left its last lines out, so a cut is only ever at or before `base_len`.
fn extend(base: &Parsed, base_len: usize, src: &str, parse: Parse, how: Reading) -> Option<Parsed> {
    let i = base.cuts.iter().rposition(|c| c.at <= base_len && src[c.at..base_len].contains('\n'))?;
    let cut = base.cuts[i];
    if cut.at == 0 || cut.blocks >= base.doc.0.len() || base.defs.is_some() {
        return None;
    }
    let ((tail, sources, tasks, cuts, starts, maps), defs) = parse(&src[cut.at..], how);
    if defs.is_some() {
        return None;
    }
    #[cfg(test)]
    PARSES.with(|p| p.set((p.get().0, p.get().1 + 1)));
    let mut blocks = base.doc.0[..cut.blocks].to_vec();
    blocks.extend(tail);
    let sources: Rc<[Source]> = base.doc.1[..cut.sources].iter().cloned().chain(sources).collect();
    let tasks = base.doc.2[..cut.tasks].iter().copied()
        .chain(tasks.into_iter().map(|t| t.map(|(checked, at)| (checked, at + cut.at)))).collect();
    let starts: Rc<[usize]> = base.doc.3[..cut.blocks].iter().copied().chain(starts.into_iter().map(|at| at + cut.at)).collect();
    // The block before the cut ends where the cut is, as it did, so its digest holds.
    let digests = base.digests[..cut.blocks].iter().copied().chain(digests(src, &starts, cut.blocks)).collect();
    let maps = base.doc.4[..cut.blocks].iter().cloned()
        .chain(maps.into_iter().map(|m| m.shift_src(cut.at))).collect();
    let cuts = base.cuts[..i]
        .iter()
        .copied()
        .chain(cuts.into_iter().map(|c| Cut {
            at: c.at + cut.at,
            blocks: c.blocks + cut.blocks,
            sources: c.sources + cut.sources,
            tasks: c.tasks + cut.tasks,
        }))
        .collect();
    Some(Parsed { doc: (Rc::new(blocks), sources, tasks, starts, maps), cuts, defs: None, digests })
}
