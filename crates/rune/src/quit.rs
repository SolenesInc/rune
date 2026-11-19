//! The app quitting, as one signal whatever starts it: `Tx::quit` or `AppHandle::quit` from any
//! window, the last window closing, macOS Quit (?Q, the Dock, logout and shutdown, which AppKit
//! asks the app delegate about with `applicationShouldTerminate:`). Every open window's `App::on_quit`
//! runs once when it starts, before any window closes for it, and [`quitting`] holds from then
//! on, so a window's `on_close` can tell the app ending from the user closing that one window.
//!
//! One machine ([`State`], [`step`]) holds where the quit is: running, quitting (the windows'
//! hooks owed, then run, and whether the app starts again after, `Tx::relaunch`), done. The
//! hooks run in the event loop (`Windows`, which carries out the effects).

use std::sync::Mutex;

#[cfg(target_os = "macos")]
mod mac;
pub(crate) mod relaunch;

/// Where the app's quit is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    /// The app runs.
    Running,
    /// A quit started; the windows' hooks are owed or ran, and what follows the exit.
    Quitting(Hooks, After),
    /// The loop ended.
    Done,
}

/// The windows' `App::on_quit` hooks, for a quit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hooks {
    /// Not run yet.
    Owed,
    /// Every open window ran its hook.
    Ran,
}

/// What follows a quit's exit: nothing, or the app starting again (`Tx::relaunch`). A relaunch
/// asked during a quit outranks the plain exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum After {
    Exit,
    Relaunch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    /// Something asks to quit: `Tx::quit` or `Tx::relaunch`, the last window closing, macOS
    /// terminating.
    Start { after: After },
    /// The loop ran every window's hook.
    HooksRan,
    /// The event loop ended.
    Exited,
}

/// What the loop does for a step, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    /// Every open window runs `App::on_quit`, then the loop sends `Event::HooksRan`.
    RunHooks,
    /// The loop ended for a relaunch: start the app again (`crate::relaunch`).
    Relaunch,
}

pub(crate) fn step(state: State, event: Event) -> (State, Vec<Effect>) {
    use {Effect::*, Event::*, State::*};
    match (state, event) {
        (Done, _) => (Done, vec![]),
        (Quitting(_, After::Relaunch), Exited) => (Done, vec![Relaunch]),
        (_, Exited) => (Done, vec![]),
        (Running, Start { after }) => (Quitting(Hooks::Owed, after), vec![RunHooks]),
        (Quitting(Hooks::Owed, was), Start { after }) => {
            (Quitting(Hooks::Owed, was.max(after)), vec![RunHooks])
        }
        (Quitting(Hooks::Ran, was), Start { after }) => {
            (Quitting(Hooks::Ran, was.max(after)), vec![])
        }
        (Quitting(_, after), HooksRan) => (Quitting(Hooks::Ran, after), vec![]),
        (Running, HooksRan) => (Running, vec![]),
    }
}

static STATE: Mutex<State> = Mutex::new(State::Running);
static FAST_EXIT: std::sync::OnceLock<fn()> = std::sync::OnceLock::new();

/// Ends the process without tearing anything down once the app has quit: every window hides as
/// the quit starts, every window's `App::on_quit` runs, the clipboard hands over what it holds,
/// tray icons leave the notification area, a relaunch starts, then `last` runs (the app's final
/// durable write, a trace) and the process ends at once. Windows, GPU devices, threads and memory
/// aren't dropped: the OS takes them back. Nothing after the event loop returns runs,
/// so whatever the app does there belongs in `last` or its quit hooks. Set it once, before the
/// app runs; a later call does nothing. On the web the page's own lifetime applies instead.
pub fn fast_exit(last: fn()) {
    let _ = FAST_EXIT.set(last);
}

/// The app's last step before a fast exit, when it asked for one (`fast_exit`).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn fast_exit_hook() -> Option<fn()> {
    FAST_EXIT.get().copied()
}

/// A fast exit, once the windows' hooks ran and the loop is ending: what must leave the system
/// tidy goes in order (the clipboard's hand-over, tray icons, a relaunch), then the app's
/// `last`, then the process ends without dropping a window, a device or an allocation.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn end_fast(last: fn()) -> ! {
    crate::clipboard::shutdown();
    crate::tray::remove_all();
    if send(Event::Exited).contains(&Effect::Relaunch) {
        relaunch::spawn();
    }
    crate::marks::mark("quit.last");
    last();
    end_process()
}

/// Ends this process now with exit code 0: no destructors, no `atexit`.
#[cfg(not(target_arch = "wasm32"))]
fn end_process() -> ! {
    crate::marks::mark("quit.end");
    #[cfg(unix)]
    // SAFETY: `_exit` ends the process without running anything of this process's.
    unsafe {
        libc::_exit(0)
    }
    #[allow(unreachable_code)]
    std::process::exit(0)
}

/// Steps the app's quit, returning what the loop must do.
pub(crate) fn send(event: Event) -> Vec<Effect> {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let (next, effects) = step(*state, event);
    if *state == State::Running && matches!(next, State::Quitting(..)) {
        crate::marks::mark("quit.start");
        #[cfg(not(target_arch = "wasm32"))]
        watch_quit();
    }
    *state = next;
    effects
}

/// A quit still running this long after it started says where it is on stderr, and again as long
/// as it runs: the last step it reached.
#[cfg(not(target_arch = "wasm32"))]
const QUIT_REPORT_EVERY: std::time::Duration = std::time::Duration::from_secs(3);

#[cfg(not(target_arch = "wasm32"))]
fn watch_quit() {
    let started = std::time::Instant::now();
    let _ = std::thread::Builder::new()
        .name("rune-quit-watch".into())
        .spawn(move || {
            loop {
                std::thread::sleep(QUIT_REPORT_EVERY);
                eprintln!(
                    "rune: still quitting {} ms after the quit started: {}",
                    started.elapsed().as_millis(),
                    where_the_quit_is()
                );
            }
        });
}

/// Ends the process `after` from now if it's still alive then, saying on stderr what it was
/// waiting for (the last step it reached) and marking `quit.deadline`.
/// For an app to call once nothing of its own is left to lose, after its storage has closed, so
/// that a quit always ends.
#[cfg(not(target_arch = "wasm32"))]
pub fn exit_deadline(after: std::time::Duration) {
    let _ = std::thread::Builder::new().name("rune-exit-deadline".into()).spawn(move || {
        std::thread::sleep(after);
        eprintln!("rune: the quit didn't end within {} ms of the app's last step, so the process ends itself: {}", after.as_millis(), where_the_quit_is());
        crate::marks::mark("quit.deadline");
        end_process();
    });
}

/// The quit's last step reached and how long ago.
#[cfg(not(target_arch = "wasm32"))]
fn where_the_quit_is() -> String {
    let last = crate::marks::marks().into_iter().last().map_or_else(
        || "no step".to_owned(),
        |(name, at)| format!("last step {name}, {} ms ago", at.elapsed().as_millis()),
    );
    last
}

pub(crate) fn state() -> State {
    *STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Whether the app is quitting: a quit started. A window's
/// `on_close` that runs while this holds closes because the app ends, so it keeps what it would
/// forget when the user closes it alone (an app reopening its windows next launch keeps that
/// window's place in the list). False while the app runs, including when its last window
/// closes, whose `on_close` runs before the quit starts.
pub fn quitting() -> bool {
    !matches!(state(), State::Running)
}

/// Hooks the system's quit requests into the loop. Once per process, on the UI thread.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) fn install() {
    #[cfg(target_os = "macos")]
    mac::install();
}
