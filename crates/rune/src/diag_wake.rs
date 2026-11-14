//! `RUNE_WAKE_LOG=FILE`: one line per wake, redraw ask, paint and draw decision of a window, to
//! find where a window that stops drawing stops.

use std::io::Write;
use std::sync::{Mutex, OnceLock};

static OUT: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();

pub(crate) fn log(args: std::fmt::Arguments) {
    let out = OUT.get_or_init(|| {
        let path = std::env::var_os("RUNE_WAKE_LOG")?;
        std::fs::OpenOptions::new().create(true).append(true).open(path).ok().map(Mutex::new)
    });
    let Some(out) = out else { return };
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64();
    let _ = writeln!(out.lock().unwrap(), "{t:.3} {args}");
}

macro_rules! wake_log { ($($a:tt)*) => { $crate::diag_wake::log(format_args!($($a)*)) } }
pub(crate) use wake_log;
