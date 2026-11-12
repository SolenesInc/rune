//! Real input for the macOS VM, where the app may take focus: CGEvent clicks, keys, wheel and
//! input-method composition, with the page's own record of what it received after each step.
//! Results go to <out>/drive.jsonl with window screenshots beside it.

use crate::Proto;
use crate::drive::{OUT, PAGE_JS, RECTS, eval, say};
use core_foundation::base::{CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::{CFString, CFStringRef};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGMouseButton, ScrollEventUnit};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSScreen, NSView, NSWorkspace};
use objc2_foundation::NSPoint;
use rune::AppHandle;
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

static VIEW: AtomicUsize = AtomicUsize::new(0);
static NUMBER: AtomicU32 = AtomicU32::new(0);

pub fn record_window(w: &rune::WindowHandle) {
    let ptr = w
        .with_raw_handle(|h| match h.as_raw() {
            raw_window_handle::RawWindowHandle::AppKit(a) => a.ns_view.as_ptr() as usize,
            _ => 0,
        })
        .unwrap_or(0);
    VIEW.store(ptr, Ordering::Release);
    NUMBER.store(w.window_number().unwrap_or(0), Ordering::Release);
}


fn view() -> &'static NSView {
    unsafe { &*(VIEW.load(Ordering::Acquire) as *const NSView) }
}


pub fn shot(name: &str) -> String {
    let path = format!("{}/{name}.png", OUT.lock().unwrap());
    let ok = std::process::Command::new("screencapture").args(["-x", "-o", "-l", &NUMBER.load(Ordering::Acquire).to_string(), &path]).status().map(|s| s.success()).unwrap_or(false);
    if ok { name.to_owned() } else { "screencapture failed".into() }
}

fn source() -> CGEventSource {
    CGEventSource::new(CGEventSourceStateID::HIDSystemState).expect("event source")
}

fn tap_key(code: u16, flags: CGEventFlags) {
    for down in [true, false] {
        let ev = CGEvent::new_keyboard_event(source(), code, down).expect("key event");
        ev.set_flags(flags);
        ev.post(CGEventTapLocation::HID);
        std::thread::sleep(Duration::from_millis(40));
    }
    std::thread::sleep(Duration::from_millis(120));
}

fn click(at: CGPoint) {
    let mv = CGEvent::new_mouse_event(source(), CGEventType::MouseMoved, at, CGMouseButton::Left).expect("move");
    mv.post(CGEventTapLocation::HID);
    std::thread::sleep(Duration::from_millis(150));
    for kind in [CGEventType::LeftMouseDown, CGEventType::LeftMouseUp] {
        let ev = CGEvent::new_mouse_event(source(), kind, at, CGMouseButton::Left).expect("mouse");
        ev.post(CGEventTapLocation::HID);
        std::thread::sleep(Duration::from_millis(60));
    }
    std::thread::sleep(Duration::from_millis(400));
}

fn wheel(at: CGPoint, dy: i32, n: usize) {
    let mv = CGEvent::new_mouse_event(source(), CGEventType::MouseMoved, at, CGMouseButton::Left).expect("move");
    mv.post(CGEventTapLocation::HID);
    std::thread::sleep(Duration::from_millis(150));
    for _ in 0..n {
        let ev = CGEvent::new_scroll_event(source(), ScrollEventUnit::PIXEL, 1, dy, 0, 0).expect("scroll");
        unsafe { CGEventSetLocation(std::mem::transmute_copy::<CGEvent, *mut c_void>(&ev), at) };
        unsafe { CGEventPostToPid(std::process::id() as i32, std::mem::transmute_copy::<CGEvent, *mut c_void>(&ev)) };
        std::thread::sleep(Duration::from_millis(16));
    }
    std::thread::sleep(Duration::from_millis(500));
}

const KEY_H: u16 = 4;
const KEY_I: u16 = 34;
const KEY_N: u16 = 45;
const KEY_O: u16 = 31;
const KEY_R: u16 = 15;
const KEY_K: u16 = 40;
const KEY_SPACE: u16 = 49;
const KEY_RETURN: u16 = 36;
const KEY_ESCAPE: u16 = 53;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyInputSourceID: CFStringRef;
    fn TISCreateInputSourceList(properties: *const c_void, include_all_installed: u8) -> *const c_void;
    fn TISEnableInputSource(source: *const c_void) -> i32;
    fn TISSelectInputSource(source: *const c_void) -> i32;
    fn TISRegisterInputSource(location: *const c_void) -> i32;
    fn TISGetInputSourceProperty(source: *const c_void, key: CFStringRef) -> *const c_void;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CGEventSetLocation(event: *mut c_void, location: CGPoint);
    fn CGEventPostToPid(pid: i32, event: *mut c_void);
    fn CFArrayGetCount(a: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(a: *const c_void, i: isize) -> *const c_void;
}

/// Selects an input source on the UI thread; HIToolbox traps when TIS runs anywhere else.
fn select_source(h: &AppHandle<Proto>, id: &'static str) -> bool {
    let (tx, rx) = mpsc::channel();
    h.update(move |_, _| { let _ = tx.send(select_source_now(id)); });
    rx.recv_timeout(Duration::from_secs(3)).unwrap_or(false)
}

fn select_source_now(id: &str) -> bool {
    let mut log = serde_json::Map::new();
    if id.contains("Kotoeri") {
        if find_source(id).is_none() {
            let url = core_foundation::url::CFURL::from_path("/System/Library/Input Methods/JapaneseIM-RomajiTyping.app", true).unwrap();
            log.insert("register".into(), unsafe { TISRegisterInputSource(url.as_concrete_TypeRef() as *const c_void) }.into());
        }
        if let Some(parent) = find_source("com.apple.inputmethod.Kotoeri.RomajiTyping") {
            log.insert("enableParent".into(), unsafe { TISEnableInputSource(parent) }.into());
        }
        log.insert("sources".into(), installed_sources("Kotoeri").into());
    }
    let ok = match find_source(id) {
        Some(src) => {
            let enabled = unsafe { TISEnableInputSource(src) };
            let selected = unsafe { TISSelectInputSource(src) };
            log.insert("enable".into(), enabled.into());
            log.insert("select".into(), selected.into());
            selected == 0
        }
        None => false,
    };
    say("drive", serde_json::json!({"step": "input-source", "id": id, "ok": ok, "detail": log}));
    ok
}

fn find_source(id: &str) -> Option<*const c_void> {
    unsafe {
        let key = CFString::wrap_under_get_rule(kTISPropertyInputSourceID);
        let value = CFString::new(id);
        let dict: CFDictionary<CFType, CFType> = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);
        let list = TISCreateInputSourceList(dict.as_concrete_TypeRef() as *const c_void, 1);
        (!list.is_null() && CFArrayGetCount(list) > 0).then(|| CFArrayGetValueAtIndex(list, 0))
    }
}

fn installed_sources(containing: &str) -> Vec<String> {
    unsafe {
        let list = TISCreateInputSourceList(std::ptr::null(), 1);
        if list.is_null() {
            return vec![];
        }
        (0..CFArrayGetCount(list)).filter_map(|i| {
            let p = TISGetInputSourceProperty(CFArrayGetValueAtIndex(list, i), kTISPropertyInputSourceID);
            (!p.is_null()).then(|| CFString::wrap_under_get_rule(p as CFStringRef).to_string())
        }).filter(|id| id.contains(containing)).collect()
    }
}

/// A point in window points (y down) to global display coordinates (y down), on the UI thread.
fn to_screen(h: &AppHandle<Proto>, x: f32, y: f32) -> CGPoint {
    let (tx, rx) = mpsc::channel();
    h.update(move |_, _| {
        let mtm = MainThreadMarker::new().unwrap();
        let v = view();
        let in_window = v.convertPoint_toView(NSPoint::new(x as f64, y as f64), None);
        let on_screen = v.window().unwrap().convertPointToScreen(in_window);
        let screen_h = NSScreen::screens(mtm).firstObject().map(|s| s.frame().size.height).unwrap_or(0.0);
        let _ = tx.send(CGPoint::new(on_screen.x, screen_h - on_screen.y));
    });
    rx.recv_timeout(Duration::from_secs(3)).unwrap_or(CGPoint::new(0.0, 0.0))
}

fn tile_point(h: &AppHandle<Proto>, tile: usize, dx: f32, dy: f32) -> CGPoint {
    let r = RECTS.lock().unwrap().as_ref().and_then(|m| m.get(&tile).copied()).unwrap_or([0.0; 4]);
    to_screen(h, r[0] + dx, r[1] + dy)
}



fn step(h: &AppHandle<Proto>, name: &'static str, tile: usize) {
    std::thread::sleep(Duration::from_millis(500));
    let page = eval(h, tile, PAGE_JS);
    let (tx, rx) = mpsc::channel();
    h.update(move |s: &mut Proto, _| {
        let mtm = MainThreadMarker::new().unwrap();
        let first = view().window().and_then(|w| w.firstResponder()).map(|r| r.class().name().to_string_lossy().into_owned()).unwrap_or_default();
        let key = view().window().map(|w| w.isKeyWindow()).unwrap_or(false);
        let _ = mtm;
        let front = NSWorkspace::sharedWorkspace().frontmostApplication().and_then(|a| a.localizedName()).map(|n| n.to_string()).unwrap_or_default();
        let _ = tx.send((s.focused.clone(), s.field.text.clone(), first, key, front));
    });
    let (focused, field, first, key, front) = rx.recv_timeout(Duration::from_secs(3)).unwrap_or_default();
    let page: serde_json::Value = serde_json::from_str(&page).unwrap_or(serde_json::Value::String(page));
    say("drive", serde_json::json!({"step": name, "tile": tile, "page": page, "runeFocus": focused, "runeField": field, "firstResponder": first, "isKeyWindow": key, "front": front, "shot": shot(name)}));
}

pub fn start(h: AppHandle<Proto>, out: String) {
    *OUT.lock().unwrap() = out.clone();
    let _ = std::fs::create_dir_all(&out);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(4000));
        h.update(|_, _| {
            let mtm = MainThreadMarker::new().unwrap();
            #[allow(deprecated)]
            NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
            view().window().unwrap().makeKeyAndOrderFront(None);
        });
        std::thread::sleep(Duration::from_millis(1500));
        select_source(&h, "com.apple.keylayout.ABC");
        let input = tile_point(&h, 0, 184.0, 42.0);
        let field = to_screen(&h, 115.0, 62.0);
        step(&h, "start", 0);

        click(input);
        step(&h, "page-click", 0);
        tap_key(KEY_H, CGEventFlags::empty());
        tap_key(KEY_I, CGEventFlags::empty());
        step(&h, "page-typed", 0);

        click(field);
        tap_key(KEY_R, CGEventFlags::empty());
        step(&h, "rune-field-typed", 0);

        click(input);
        tap_key(KEY_O, CGEventFlags::empty());
        step(&h, "back-in-page", 0);

        wheel(tile_point(&h, 0, 200.0, 300.0), -40, 20);
        step(&h, "scrolled", 0);
        for _ in 0..20 {
            h.update(|_, _| { crate::with_host(|b| b.wheel(0, rune::WheelEvent::new(0.0, -40.0).at(200.0, 300.0))); });
            std::thread::sleep(Duration::from_millis(16));
        }
        step(&h, "scrolled-through-host", 0);
        let _ = eval(&h, 0, "window.scrollTo(0,0); 'ok'");

        click(input);
        let japanese = select_source(&h, "com.apple.inputmethod.Kotoeri.RomajiTyping.Japanese");
        std::thread::sleep(Duration::from_millis(2000));
        say("drive", serde_json::json!({"step": "select-japanese", "ok": japanese}));
        for code in [KEY_N, KEY_I, KEY_H, KEY_O, KEY_N] {
            tap_key(code, CGEventFlags::empty());
        }
        step(&h, "ime-preedit", 0);
        tap_key(KEY_SPACE, CGEventFlags::empty());
        step(&h, "ime-converted", 0);
        tap_key(KEY_RETURN, CGEventFlags::empty());
        step(&h, "ime-committed", 0);
        select_source(&h, "com.apple.keylayout.ABC");
        std::thread::sleep(Duration::from_millis(800));

        tap_key(KEY_K, CGEventFlags::CGEventFlagCommand);
        step(&h, "palette-over-page", 0);
        tap_key(KEY_ESCAPE, CGEventFlags::empty());

        click(tile_point(&h, 1, 200.0, 200.0));
        step(&h, "second-tile-click", 1);

        h.update(|s: &mut Proto, _| s.wide = true);
        step(&h, "resized-wide", 0);
        h.update(|s: &mut Proto, _| s.wide = false);
        step(&h, "resized-back", 0);

        let _ = std::process::Command::new("osascript").args(["-e", "tell application \"Finder\" to activate"]).status();
        std::thread::sleep(Duration::from_millis(2500));
        step(&h, "other-app-front", 0);
        click(input);
        step(&h, "inactive-first-click", 0);
        tap_key(KEY_H, CGEventFlags::empty());
        step(&h, "inactive-typed", 0);
        std::thread::sleep(Duration::from_millis(500));
        std::process::exit(0);
    });
}


