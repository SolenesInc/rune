//! `Tx::inputs`: how many times the person has acted on this thread's windows (a key, a
//! pointer or secondary press, a touch, a wheel, committed input-method text), so an answer
//! that lands later can tell whether they did anything since the action that asked for it.
//! Pointer motion, releases, a key's auto-repeat and a lone modifier are not acts. Each queued
//! action carries the count as of its own input, so a handler sees it as of that input even
//! when later input queued before it ran.

use std::cell::Cell;

thread_local! {
    static COUNT: Cell<u64> = const { Cell::new(0) };
    static CURRENT: Cell<Option<u64>> = const { Cell::new(None) };
}

/// The person acted.
pub(crate) fn act() {
    COUNT.with(|c| c.set(c.get() + 1));
}

/// The count as of the input that is queuing an action now.
pub(crate) fn queued() -> u64 {
    COUNT.with(Cell::get)
}

/// The count a handler sees: as of the input it runs for, else as of now.
pub(crate) fn current() -> u64 {
    CURRENT.with(Cell::get).unwrap_or_else(queued)
}

/// Holds the count of the action being dispatched until dropped, then restores the one before.
#[must_use]
pub(crate) struct Dispatching(Option<u64>);

pub(crate) fn scope(count: u64) -> Dispatching {
    Dispatching(CURRENT.with(|c| c.replace(Some(count))))
}

impl Drop for Dispatching {
    fn drop(&mut self) {
        CURRENT.with(|c| c.set(self.0));
    }
}
