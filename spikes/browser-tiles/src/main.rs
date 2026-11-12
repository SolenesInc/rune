//! Browser tiles in a Rune window, behind one `BrowserHost` seam, with two engines:
//! `cef` draws Chromium off-screen into a `NativeSurface`; `wk` adds a `WKWebView` child view.
//!
//!   browser-tiles <cef|wk|none> [--tiles N] [--url U]... [--park X,Y] [--palette] [--anim]
//!                 [--scroll] [--lazy] [--no-copy] [--drive OUT] [--agent OUT] [--quit-after S]

mod drive;
#[cfg(target_os = "macos")]
mod drive_mac;
mod keys;
#[cfg(target_os = "macos")]
mod wk_host;
#[cfg(feature = "cef")]
mod cef_host;

use rune::tok::{self, type_};
use rune::{App, AppSpec, Cx, El, Ime, Key, KeyEvent, NativeSurface, PhysicalInput, SurfaceFit, TextEdit, Tx, WheelEvent, col, el, row, text, text_input};
use std::cell::RefCell;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Engine {
    Cef,
    Wk,
    None,
}

#[derive(Debug)]
pub struct Opts {
    pub engine: Engine,
    pub tiles: usize,
    pub urls: Vec<String>,
    pub park: Option<(i32, i32)>,
    pub palette: bool,
    pub anim: bool,
    pub scroll: bool,
    pub lazy: bool,
    pub no_copy: bool,
    pub drive: Option<String>,
    pub agent: Option<String>,
    pub quit_after: Option<f64>,
    pub shot: Option<String>,
}

static OPTS: OnceLock<Opts> = OnceLock::new();

#[cfg(target_os = "macos")]
pub fn platform_shot(name: &str) -> String {
    drive_mac::shot(name)
}

#[cfg(not(target_os = "macos"))]
pub fn platform_shot(name: &str) -> String {
    let path = format!("{}/{name}.png", drive::OUT.lock().unwrap());
    let ok = std::process::Command::new("import").args(["-window", "root", &path]).status().map(|s| s.success()).unwrap_or(false);
    if ok { name.to_owned() } else { "import failed".into() }
}

pub fn opts() -> &'static Opts {
    OPTS.get().expect("opts")
}

fn parse() -> Opts {
    let args: Vec<String> = std::env::args().skip(1).filter(|a| !a.starts_with("--type=") && !a.starts_with("-psn")).collect();
    let engine = match args.first().map(String::as_str) {
        Some("cef") => Engine::Cef,
        Some("wk") => Engine::Wk,
        _ => Engine::None,
    };
    let mut o = Opts { engine, tiles: 2, urls: vec![], park: None, palette: false, anim: false, scroll: false, lazy: false, no_copy: false, drive: None, agent: None, quit_after: None, shot: None };
    let mut i = 1;
    while i < args.len() {
        let next = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--tiles" => { o.tiles = next.parse().unwrap_or(2); i += 1 }
            "--url" => { o.urls.push(next); i += 1 }
            "--park" => { let v: Vec<i32> = next.split(',').filter_map(|s| s.parse().ok()).collect(); o.park = Some((v[0], v[1])); i += 1 }
            "--palette" => o.palette = true,
            "--anim" => o.anim = true,
            "--scroll" => o.scroll = true,
            "--lazy" => o.lazy = true,
            "--no-copy" => o.no_copy = true,
            "--drive" => { o.drive = Some(next); i += 1 }
            "--agent" => { o.agent = Some(next); i += 1 }
            "--shot" => { o.shot = Some(next); i += 1 }
            "--quit-after" => { o.quit_after = next.parse().ok(); i += 1 }
            _ => {}
        }
        i += 1;
    }
    o
}

pub const TEST_PAGE: &str = r#"<!doctype html><html><head><title>Tile test page</title><meta charset="utf-8"></head>
<body style="background:#f4f0e6;font:16px -apple-system;margin:0">
<div style="padding:24px">
<input id="f" style="width:320px;height:36px;font-size:18px" placeholder="type here" aria-label="Search">
<button id="go" onclick="document.querySelector('h1').textContent='Clicked '+document.getElementById('f').value">Go</button>
<h1>Drive heading</h1>
<p id="log">no input yet</p>
</div>
<div id="rows"></div>
<script>
window.events = [];
const note = (s) => { window.events.push(s); document.getElementById('log').textContent = window.events.slice(-12).join(' · '); };
document.addEventListener('mousedown', (e) => note('mousedown ' + e.target.tagName + ' ' + e.detail));
for (const t of ['keydown', 'keypress', 'keyup']) document.addEventListener(t, (e) => note(t + ' ' + e.key + ' ' + e.code));
document.addEventListener('beforeinput', (e) => note('input ' + e.inputType + ' ' + e.data));
document.addEventListener('compositionupdate', (e) => note('compose ' + e.data));
document.addEventListener('compositionend', (e) => note('composed ' + e.data));
window.addEventListener('focus', () => note('window focus'));
window.addEventListener('blur', () => note('window blur'));
const rows = document.getElementById('rows');
for (let i = 0; i < 400; i++) { const p = document.createElement('p'); p.style.margin = '0 24px'; p.textContent = 'Row ' + i + ' — the quick brown fox jumps over the lazy dog'; rows.appendChild(p); }
if (location.hash === '#scroll') { let y = 0; const step = () => { y = (y + 4) % 8000; window.scrollTo(0, y); requestAnimationFrame(step); }; requestAnimationFrame(step); }
</script></body></html>"#;

pub fn test_page_url(scroll: bool) -> String {
    let path = std::env::temp_dir().join("browser-tiles-test.html");
    let _ = std::fs::write(&path, TEST_PAGE);
    format!("file://{}{}", path.display(), if scroll { "#scroll" } else { "" })
}

pub struct Tile {
    pub id: usize,
    pub url: String,
    pub surface: NativeSurface,
    pub open: bool,
}

pub struct Proto {
    pub tiles: Vec<Tile>,
    pub palette: bool,
    pub field: TextEdit,
    pub wide: bool,
    pub anim_flip_at: f64,
    pub focused: Option<String>,
    pub caret: [f32; 4],
    pub rects: std::collections::HashMap<usize, [f32; 4]>,
}

/// The seam: everything the app asks of a browser engine.
pub trait BrowserHost {
    fn open(&mut self, tile: usize, url: &str, surface: &NativeSurface);
    fn close(&mut self, tile: usize);
    /// The tile's page rect in window points, as drawn.
    fn place(&mut self, tile: usize, rect: [f32; 4]);
    fn focus(&mut self, tile: usize, focused: bool);
    fn mouse(&mut self, tile: usize, x: f32, y: f32, phase: rune::Phase, clicks: u32, mods: rune::Mods);
    fn hover(&mut self, tile: usize, x: f32, y: f32, inside: bool);
    fn wheel(&mut self, tile: usize, ev: WheelEvent);
    fn physical(&mut self, tile: usize, ev: &PhysicalInput);
    fn key(&mut self, tile: usize, ev: &KeyEvent) -> bool;
    fn ime(&mut self, tile: usize, ev: &Ime) -> bool;
    /// Run the engine's work that is due; false when nothing ran.
    fn pump(&mut self) {}
    /// The caret rect CEF reports for the composition, in tile points.
    fn caret(&self, _tile: usize) -> Option<[f32; 4]> { None }
    /// Runs `js` in the page; `done` gets the result as a string. False when it cannot run.
    fn eval(&mut self, tile: usize, js: &str, done: Box<dyn FnOnce(String)>) -> bool;
    /// An agent's capture: `screenshot`, `pdf`, `cookies` or `ax-tree`, written to `path`.
    fn stats(&self) -> serde_json::Value { serde_json::Value::Null }
    fn capture(&mut self, _tile: usize, _what: &str, _path: &str, _done: Box<dyn FnOnce(bool, usize)>) -> bool { false }
}

thread_local! {
    pub static HOST: RefCell<Option<Box<dyn BrowserHost>>> = const { RefCell::new(None) };
}

pub fn with_host<R>(f: impl FnOnce(&mut dyn BrowserHost) -> R) -> Option<R> {
    HOST.with_borrow_mut(|h| h.as_mut().map(|h| f(h.as_mut())))
}

fn tile_key(id: usize) -> String {
    format!("tile-{id}")
}

fn init() -> Proto {
    let o = opts();
    let url = |i: usize| o.urls.get(i).cloned().unwrap_or_else(|| if i == 0 { test_page_url(o.scroll) } else { "https://example.com/".into() });
    let tiles = (0..2).map(|id| Tile { id, url: url(id), surface: NativeSurface::new(), open: id < o.tiles }).collect();
    Proto { tiles, palette: o.palette, field: TextEdit::new(""), wide: false, anim_flip_at: 1.0, focused: None, caret: [0.0; 4], rects: Default::default() }
}

fn setup(s: &mut Proto, wake: rune::Waker) {
    for t in &s.tiles {
        t.surface.set_waker(wake.clone());
    }
}

fn poll(s: &mut Proto, clock: f64, tx: &mut Tx) -> bool {
    with_host(|h| h.pump());
    let focused = tx.focused().map(str::to_owned);
    if focused != s.focused {
        for t in &s.tiles {
            let was = s.focused.as_deref() == Some(tile_key(t.id).as_str());
            let is = focused.as_deref() == Some(tile_key(t.id).as_str());
            if was != is && t.open {
                with_host(|h| h.focus(t.id, is));
            }
        }
        s.focused = focused;
    }
    let mut changed = false;
    if let Some(c) = s.focused.as_deref().and_then(|k| k.strip_prefix("tile-")).and_then(|i| i.parse().ok()).and_then(|i| with_host(|h| h.caret(i)).flatten()) {
        if c != s.caret {
            s.caret = c;
            changed = true;
        }
    }
    if opts().anim && clock >= s.anim_flip_at {
        s.wide = !s.wide;
        s.anim_flip_at = clock + 0.6;
        tx.wake_at(s.anim_flip_at);
        changed = true;
    }
    if let Some(q) = opts().quit_after {
        if clock >= q {
            if let Some(st) = with_host(|h| h.stats()) {
                println!("{}", serde_json::json!({"stats": st}));
            }
            std::process::exit(0);
        }
        tx.wake_at(q);
    }
    changed
}

fn on_ime(s: &mut Proto, ev: &Ime, _: &mut Tx) -> bool {
    if std::env::var_os("BT_DEBUG").is_some() {
        eprintln!("ime {ev:?} focused={:?}", s.focused);
    }
    let Some(id) = s.focused.as_deref().and_then(|k| k.strip_prefix("tile-")).and_then(|i| i.parse::<usize>().ok()) else { return false };
    with_host(|h| h.ime(id, ev)).unwrap_or(false)
}

fn on_key(s: &mut Proto, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.is_command('k') {
        s.palette = !s.palette;
        return true;
    }
    if ev.key == Key::Escape && s.palette {
        s.palette = false;
        return true;
    }
    if ev.is_command('t') {
        if let Some(t) = s.tiles.iter_mut().find(|t| !t.open) {
            t.open = true;
            with_host(|h| h.open(t.id, &t.url, &t.surface));
            if let Some(r) = s.rects.get(&t.id).copied() {
                with_host(|h| h.place(t.id, r));
            }
        }
        return true;
    }
    if ev.is_command('w') {
        if let Some(t) = s.tiles.iter_mut().rev().find(|t| t.open) {
            t.open = false;
            with_host(|h| h.close(t.id));
        }
        tx.blur();
        return true;
    }
    false
}

fn tile_view(t: &Tile, s: &Proto, flex: f32) -> El {
    let th = tok::theme(true);
    let id = t.id;
    let focused = s.focused.as_deref() == Some(tile_key(id).as_str());
    let header = row().h(32.0).px(10.0).gap(8.0).items_center().fill(th.card)
        .child(text(format!("Tile {id}: {}", t.url.chars().take(60).collect::<String>())).font(type_::BODY).color(if focused { th.text } else { th.text_secondary }));
    let mut page = el().key(tile_key(id)).grow(1.0).w_full().focusable(true)
        .on_layout(move |s: &mut Proto, r| { s.rects.insert(id, r); with_host(|h| h.place(id, r)); })
        .on_drawn_layout(move |s: &mut Proto, r| { s.rects.insert(id, r); with_host(|h| h.place(id, r)); });
    if opts().engine != Engine::Cef {
        page = page.fill(rune::oklch(0.95, 0.01, 90.0));
    }
    if opts().engine == Engine::Cef {
        page = page.native_surface(&t.surface, SurfaceFit::Fill).surface_anchor(rune::Align::Start, rune::Align::Start)
            .on_drag(move |_: &mut Proto, d, tx| {
                tx.focus(tile_key(id), false);
                with_host(|h| h.mouse(id, d.x, d.y, d.phase, d.clicks, rune::Mods::default()));
            })
            .on_pointer_move_local(move |_: &mut Proto, p, _| { with_host(|h| h.hover(id, p.x, p.y, true)); })
            .on_hover_change(move |_: &mut Proto, inside, _| { if !inside { with_host(|h| h.hover(id, 0.0, 0.0, false)); } })
            .on_wheel(move |_: &mut Proto, ev, _| with_host(|h| h.wheel(id, ev)).is_some())
            .on_physical_input(move |_: &mut Proto, ev, _| { with_host(|h| h.physical(id, ev)); false })
            .on_key(move |s: &mut Proto, ev, _| !s.palette && with_host(|h| h.key(id, ev)).unwrap_or(false));
        if focused {
            let c = s.caret;
            page = page.child(el().key(format!("caret-{id}")).absolute(c[0], c[1]).size(c[2].max(1.0), c[3].max(16.0)).ime_cursor());
        }
    }
    col().key(format!("tile-box-{id}")).grow(flex).h_full().radius(10.0).clip_rounded(10.0).border(1.0, if focused { th.indicator } else { th.card_border })
        .child(header).child(page)
}

fn view(s: &Proto, _: &mut Cx<Proto>) -> El {
    let th = tok::theme(true);
    let sidebar = col().key("sidebar").w(230.0).h_full().padding(14.0).gap(10.0).fill(th.card)
        .child(text("Rune sidebar").font(type_::TITLE).color(th.text))
        .child(text_input(&s.field, type_::BODY.lh(26.0)).key("rune-field").single_line().w_full().color(th.text).on_edit(|s: &mut Proto| &mut s.field))
        .child(text(format!("focus: {}", s.focused.as_deref().unwrap_or("-"))).font(type_::BODY).color(th.text_secondary))
        .child(text("⌘K palette · ⌘T open tile · ⌘W close tile").font(type_::BODY).color(th.text_secondary));
    let open: Vec<&Tile> = s.tiles.iter().filter(|t| t.open).collect();
    let tiles = row().key("tiles").grow(1.0).h_full().gap(10.0).padding(10.0).clip_rounded(14.0)
        .children(open.iter().enumerate().map(|(i, t)| tile_view(t, s, if i == 0 && s.wide { 2.0 } else { 1.0 })));
    let mut main = col().key("main").grow(1.0).h_full().child(tiles);
    if s.palette {
        let items = ["Open browser tile", "Close tile", "Reload page", "Toggle sidebar", "New session", "Pop out"];
        main = main.child(
            col().key("palette").absolute(60.0, 70.0).w(420.0).padding(8.0).gap(2.0).fill(th.card).radius(12.0).border(1.0, th.card_border)
                .shadow(8.0, 24.0, th.shadow)
                .child(el().h(36.0).px(10.0).items_center().fill(th.hover).radius(8.0).child(text("Search commands…").font(type_::BODY).color(th.text_secondary)))
                .children(items.iter().map(|i| el().h(30.0).px(10.0).items_center().child(text(*i).font(type_::BODY).color(th.text)))),
        );
    }
    row().w_full().h_full().fill(th.bg).child(sidebar).child(main)
}

fn main() {
    #[cfg(feature = "cef")]
    if let Some(code) = cef_host::subprocess() {
        std::process::exit(code);
    }
    let _ = OPTS.set(parse());
    let o = opts();
    let spec = AppSpec::<Proto> { setup: Some(setup), poll: Some(poll), on_key: Some(on_key), ..AppSpec::new("browser tiles", (1100.0, 700.0), || unreachable!(), view) };
    let mut app = App::new(spec).init(init).on_ime(on_ime);
    if let Some((x, y)) = o.park {
        app = app.window_options(rune::WindowOptions { position: Some((x, y)), always_on_top: true, ..Default::default() });
    }
    let handle = app.handle();
    let opened = std::sync::atomic::AtomicBool::new(false);
    let h2 = handle.clone();
    app = app.on_ready(move |#[allow(unused_variables)] w| {
        if opened.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        #[cfg(target_os = "macos")]
        drive_mac::record_window(w);
        let host: Option<Box<dyn BrowserHost>> = match o.engine {
            #[cfg(feature = "cef")]
            Engine::Cef => Some(Box::new(cef_host::Cef::start(h2.clone()))),
            #[cfg(target_os = "macos")]
            Engine::Wk => Some(Box::new(wk_host::Wk::new(w))),
            _ => None,
        };
        HOST.with_borrow_mut(|slot| *slot = host);
        let h3 = h2.clone();
        h2.update(move |s: &mut Proto, _| {
            for t in s.tiles.iter().filter(|t| t.open) {
                with_host(|h| h.open(t.id, &t.url, &t.surface));
                if let Some(r) = s.rects.get(&t.id).copied() {
                    with_host(|h| h.place(t.id, r));
                }
            }
            let _ = h3;
        });
        #[cfg(target_os = "macos")]
        if let Some(out) = o.drive.clone() {
            drive_mac::start(h2.clone(), out);
        }
        if std::env::args().any(|a| a == "--watch") {
            drive::watch(h2.clone());
        }
        if let Some(out) = o.shot.clone() {
            drive::shots(h2.clone(), out);
        }
        if let Some(out) = o.agent.clone() {
            drive::agent(h2.clone(), out);
        }
    });
    let _ = handle;
    app.run();
}
