//! What any engine can be asked without input: page state, captures, screenshots, logs.

use crate::{Proto, with_host};
use rune::AppHandle;
use std::collections::HashMap;
use std::io::Write;
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

pub static RECTS: Mutex<Option<HashMap<usize, [f32; 4]>>> = Mutex::new(None);
pub static OUT: Mutex<String> = Mutex::new(String::new());
pub fn note_rect(tile: usize, r: [f32; 4]) {
    RECTS.lock().unwrap().get_or_insert_with(HashMap::new).insert(tile, r);
}
pub fn say(name: &str, v: serde_json::Value) {
    println!("{v}");
    let path = format!("{}/{name}.jsonl", OUT.lock().unwrap());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{v}");
    }
}
pub const PAGE_JS: &str = "JSON.stringify({visibility: document.visibilityState, hasFocus: document.hasFocus(), active: document.activeElement && (document.activeElement.id || document.activeElement.tagName), value: (document.getElementById('f')||{}).value, scrollY: window.scrollY, events: window.events})";
/// Runs `js` in the tile's page and waits up to 3 s for its result.
pub fn eval(h: &AppHandle<Proto>, tile: usize, js: &str) -> String {
    let (tx, rx) = mpsc::channel::<String>();
    let js = js.to_owned();
    h.update(move |_, _| {
        let tx2 = tx.clone();
        let ran = with_host(|host| host.eval(tile, &js, Box::new(move |v| { let _ = tx2.send(v); }))).unwrap_or(false);
        if !ran {
            let _ = tx.send("<no eval>".into());
        }
    });
    rx.recv_timeout(Duration::from_secs(3)).unwrap_or_else(|_| "<timeout>".into())
}
/// What an agent's `browser_control_request` needs from the engine: evaluate, capture, PDF and
/// cookies, each timed, without focus or input.
pub fn agent(h: AppHandle<Proto>, out: String) {
    *OUT.lock().unwrap() = out.clone();
    let _ = std::fs::create_dir_all(&out);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(5000));
        let started = Instant::now();
        let title = eval(&h, 0, "document.title");
        say("agent", serde_json::json!({"step": "evaluate", "result": title, "ms": started.elapsed().as_millis() as u64}));
        for (name, ext) in [("screenshot", "png"), ("pdf", "pdf"), ("cookies", "json"), ("ax-tree", "json")] {
            let (tx, rx) = mpsc::channel::<(bool, usize)>();
            let path = format!("{out}/agent-{name}.{ext}");
            let started = Instant::now();
            h.update(move |_, _| {
                let tx2 = tx.clone();
                let ran = with_host(|host| host.capture(0, name, &path, Box::new(move |ok, bytes| { let _ = tx2.send((ok, bytes)); }))).unwrap_or(false);
                if !ran {
                    let _ = tx.send((false, 0));
                }
            });
            let (ok, bytes) = rx.recv_timeout(Duration::from_secs(10)).unwrap_or((false, 0));
            say("agent", serde_json::json!({"step": name, "ok": ok, "bytes": bytes, "ms": started.elapsed().as_millis() as u64}));
        }
        std::process::exit(0);
    });
}
/// Screenshots of the window and each tile's page state, without input or focus.
pub fn shots(h: AppHandle<Proto>, out: String) {
    *OUT.lock().unwrap() = out.clone();
    let _ = std::fs::create_dir_all(&out);
    std::thread::spawn(move || {
        for (i, wait) in [4000u64, 3000].iter().enumerate() {
            std::thread::sleep(Duration::from_millis(*wait));
            let pages: Vec<serde_json::Value> = (0..2).map(|t| { let v = eval(&h, t, "JSON.stringify({title: document.title, ready: document.readyState, scrollY: window.scrollY, visibility: document.visibilityState})"); serde_json::from_str(&v).unwrap_or(serde_json::Value::String(v)) }).collect();
            say("shots", serde_json::json!({"step": i, "pages": pages, "shot": crate::platform_shot(&format!("shot-{i}"))}));
        }
    });
}

/// Prints tile 0's page state every second, for a driver outside the app (xdotool on Linux).
pub fn watch(h: AppHandle<Proto>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(1000));
        let v = eval(&h, 0, PAGE_JS);
        let page: serde_json::Value = serde_json::from_str(&v).unwrap_or(serde_json::Value::String(v));
        let (tx, rx) = mpsc::channel();
        h.update(move |s: &mut Proto, _| { let _ = tx.send(s.focused.clone()); });
        let focused = rx.recv_timeout(Duration::from_secs(3)).ok().flatten();
        say("watch", serde_json::json!({"page": page, "runeFocus": focused, "stats": with_stats(&h)}));
    });
}

fn with_stats(h: &AppHandle<Proto>) -> serde_json::Value {
    let (tx, rx) = mpsc::channel();
    h.update(move |_, _| { let _ = tx.send(with_host(|x| x.stats()).unwrap_or_default().to_string()); });
    rx.recv_timeout(Duration::from_secs(3)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

