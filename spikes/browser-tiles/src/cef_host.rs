//! Chromium drawn off-screen: CEF renders each tile's page into an IOSurface, the paint
//! callback copies it on the GPU into a buffer Rune owns, and the tile's `NativeSurface` shows
//! it inside Rune's own render pass. Pointer, key, wheel and IME input are forwarded from Rune's
//! handlers. CEF runs on Rune's UI thread through `external_message_pump`.

use crate::{BrowserHost, Proto, opts};
use cef::{args::Args, *};
#[cfg(target_os = "macos")]
use mac::*;
use rune::{AppHandle, Ime, Key, KeyEvent, NativeSurface, PhysicalInput, WheelEvent};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ptr::null_mut;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Runs a CEF subprocess when this process is one; the exit code, or None in the browser process.
pub fn subprocess() -> Option<i32> {
    if !std::env::args().any(|a| a.starts_with("--type=")) {
        return None;
    }
    #[cfg(target_os = "macos")]
    let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), true);
    #[cfg(target_os = "macos")]
    assert!(loader.load());
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let args = Args::new();
    Some(execute_process(Some(args.as_main_args()), None::<&mut App>, null_mut()))
}

// ---------- the message pump: CEF asks for work at a delay; Rune's poll runs it ----------

struct Pump {
    due: Mutex<Option<Instant>>,
    cv: Condvar,
}

static PUMP: OnceLock<Arc<Pump>> = OnceLock::new();
static WAKE: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();
pub static PUMP_RUNS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// cefclient's MainMessageLoopExternalPump: CEF does not ask for every piece of work, so the
/// pump also runs at least this often.
const MAX_PUMP_DELAY_MS: i64 = 1000 / 30;

fn schedule(delay_ms: i64) {
    let Some(p) = PUMP.get() else { return };
    let at = Instant::now() + Duration::from_millis(delay_ms.max(0) as u64);
    let mut due = p.due.lock().unwrap();
    if due.is_none_or(|d| at < d) {
        *due = Some(at);
        p.cv.notify_one();
    }
}

fn start_pump_thread(wake: impl Fn() + Send + Sync + 'static) {
    let p = Arc::new(Pump { due: Mutex::new(None), cv: Condvar::new() });
    let _ = PUMP.set(p.clone());
    let _ = WAKE.set(Box::new(wake));
    std::thread::spawn(move || {
        let mut due = p.due.lock().unwrap();
        loop {
            match *due {
                None => due = p.cv.wait(due).unwrap(),
                Some(at) => {
                    let now = Instant::now();
                    if at <= now {
                        *due = None;
                        drop(due);
                        (WAKE.get().unwrap())();
                        due = p.due.lock().unwrap();
                    } else {
                        due = p.cv.wait_timeout(due, at - now).unwrap().0;
                    }
                }
            }
        }
    });
}

wrap_browser_process_handler! {
    struct ProtoProcessHandler;
    impl BrowserProcessHandler {
        fn on_schedule_message_pump_work(&self, delay_ms: i64) {
            schedule(delay_ms);
        }
    }
}

wrap_app! {
    struct ProtoApp;
    impl App {
        fn on_before_command_line_processing(&self, _process_type: Option<&CefString>, command_line: Option<&mut CommandLine>) {
            let Some(cl) = command_line else { return };
            cl.append_switch(Some(&"use-mock-keychain".into()));
            cl.append_switch(Some(&"disable-features=MacAppCodeSignClone".into()));
        }
        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(ProtoProcessHandler::new())
        }
    }
}

#[cfg(target_os = "macos")]
mod mac {
    pub use objc2::rc::Retained;
    pub use objc2::runtime::{AnyClass, AnyObject, AnyProtocol, Bool, Sel};
    pub use objc2::sel;
    pub use objc2_core_foundation::{CFBoolean, CFDictionary, CFNumber, CFRetained, CFString, CFType};
    pub use objc2_core_video::*;
    pub use objc2_io_surface::IOSurfaceRef;
    pub use objc2_metal::*;
    pub use rune::native_surface::macos::PixelBuffer;
    use std::ptr::{NonNull, null_mut};

// ---------- copying CEF's IOSurface into a buffer Rune owns ----------

pub struct Copier {
    device: Retained<objc2::runtime::ProtocolObject<dyn MTLDevice>>,
    queue: Retained<objc2::runtime::ProtocolObject<dyn MTLCommandQueue>>,
    pool: Option<(u32, u32, CFRetained<CVPixelBufferPool>)>,
}

impl Copier {
    pub fn new() -> Self {
        let device = MTLCreateSystemDefaultDevice().expect("metal device");
        let queue = device.newCommandQueue().expect("queue");
        Copier { device, queue, pool: None }
    }

    fn pool(&mut self, w: u32, h: u32) -> &CVPixelBufferPool {
        if self.pool.as_ref().is_none_or(|(pw, ph, _)| (*pw, *ph) != (w, h)) {
            let empty = CFDictionary::<CFString, CFType>::from_slices(&[], &[]);
            let wn = CFNumber::new_i32(w as i32);
            let hn = CFNumber::new_i32(h as i32);
            let fmt = CFNumber::new_i32(kCVPixelFormatType_32BGRA as i32);
            let attrs = unsafe {
                CFDictionary::<CFString, CFType>::from_slices(
                    &[kCVPixelBufferWidthKey, kCVPixelBufferHeightKey, kCVPixelBufferPixelFormatTypeKey, kCVPixelBufferIOSurfacePropertiesKey, kCVPixelBufferMetalCompatibilityKey],
                    &[wn.as_ref(), hn.as_ref(), fmt.as_ref(), AsRef::<CFType>::as_ref(&*empty), CFBoolean::new(true).as_ref()],
                )
            };
            let mut out = null_mut();
            let r = unsafe { CVPixelBufferPoolCreate(None, None, Some(attrs.as_opaque()), NonNull::from(&mut out)) };
            assert_eq!(r, 0, "CVPixelBufferPoolCreate");
            self.pool = Some((w, h, unsafe { CFRetained::from_raw(NonNull::new(out).unwrap()) }));
        }
        &self.pool.as_ref().unwrap().2
    }

    fn texture(&self, surface: &IOSurfaceRef, w: u32, h: u32) -> Option<Retained<objc2::runtime::ProtocolObject<dyn MTLTexture>>> {
        let desc = unsafe { MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(MTLPixelFormat::BGRA8Unorm, w as usize, h as usize, false) };
        desc.setUsage(MTLTextureUsage::ShaderRead | MTLTextureUsage::ShaderWrite);
        unsafe { self.device.newTextureWithDescriptor_iosurface_plane(&desc, surface, 0) }
    }

    /// A GPU copy of `src` into a fresh pool buffer, finished before it returns.
    pub fn copy(&mut self, src: &IOSurfaceRef) -> Option<PixelBuffer> {
        let (w, h) = (src.width() as u32, src.height() as u32);
        let pool = self.pool(w, h);
        let mut out = null_mut();
        let r = unsafe { CVPixelBufferPoolCreatePixelBuffer(None, pool, NonNull::from(&mut out)) };
        if r != 0 {
            return None;
        }
        let pb: CFRetained<CVPixelBuffer> = unsafe { CFRetained::from_raw(NonNull::new(out)?) };
        let dst_surface = CVPixelBufferGetIOSurface(Some(&pb))?;
        let src_tex = self.texture(src, w, h)?;
        let dst_tex = self.texture(&dst_surface, w, h)?;
        let cb = self.queue.commandBuffer()?;
        let enc = cb.blitCommandEncoder()?;
        unsafe { enc.copyFromTexture_toTexture(&src_tex, &dst_tex) };
        enc.endEncoding();
        cb.commit();
        cb.waitUntilCompleted();
        unsafe { PixelBuffer::from_raw(CFRetained::as_ptr(&pb).as_ptr().cast()) }.ok()
    }
}

/// Adds Chromium's NSApplication protocols to the app class winit made, so CEF accepts it.
pub fn conform_app_class() {
    unsafe extern "C" fn is_handling(_this: &AnyObject, _: Sel) -> Bool {
        Bool::NO
    }
    unsafe extern "C" fn set_handling(_this: &AnyObject, _: Sel, _v: Bool) {}
    let mtm = objc2::MainThreadMarker::new().unwrap();
    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
    let cls: &AnyClass = app.class();
    unsafe {
        let cls_ptr = cls as *const AnyClass as *mut AnyClass;
        objc2::ffi::class_addMethod(cls_ptr, sel!(isHandlingSendEvent), std::mem::transmute::<unsafe extern "C" fn(&AnyObject, Sel) -> Bool, unsafe extern "C-unwind" fn()>(is_handling), c"B@:".as_ptr());
        objc2::ffi::class_addMethod(cls_ptr, sel!(setHandlingSendEvent:), std::mem::transmute::<unsafe extern "C" fn(&AnyObject, Sel, Bool), unsafe extern "C-unwind" fn()>(set_handling), c"v@:B".as_ptr());
        for name in [c"CrAppProtocol", c"CrAppControlProtocol", c"CefAppProtocol"] {
            if let Some(p) = AnyProtocol::get(name) {
                objc2::ffi::class_addProtocol(cls_ptr, p);
            }
        }
    }
}

}

// ---------- one tile ----------

pub struct TileShared {
    id: usize,
    size: Cell<[f32; 2]>,
    scale: f32,
    surface: NativeSurface,
    caret: Cell<Option<[f32; 4]>>,
    #[cfg(target_os = "macos")]
    copier: RefCell<Copier>,
    frames: Cell<u64>,
    frame_id: Cell<u64>,
    copy_ms: Cell<f64>,
    pending: RefCell<HashMap<i32, Box<dyn FnOnce(bool, &[u8])>>>,
}

wrap_render_handler! {
    struct TileRender {
        tile: Rc<TileShared>,
    }
    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(rect) = rect {
                let [w, h] = self.tile.size.get();
                rect.x = 0;
                rect.y = 0;
                rect.width = (w as i32).max(1);
                rect.height = (h as i32).max(1);
            }
        }
        fn screen_info(&self, _browser: Option<&mut Browser>, info: Option<&mut ScreenInfo>) -> ::std::os::raw::c_int {
            if let Some(info) = info {
                info.device_scale_factor = self.tile.scale;
                return 1;
            }
            0
        }
        #[cfg(target_os = "macos")]
        fn on_accelerated_paint(&self, _browser: Option<&mut Browser>, type_: PaintElementType, _dirty: Option<&[Rect]>, info: Option<&AcceleratedPaintInfo>) {
            let Some(info) = info else { return };
            if type_ != PaintElementType::VIEW || info.shared_texture_io_surface.is_null() {
                return;
            }
            let started = Instant::now();
            let surface: &IOSurfaceRef = unsafe { &*(info.shared_texture_io_surface as *const IOSurfaceRef) };
            let buffer = if opts().no_copy {
                unsafe { PixelBuffer::from_iosurface(info.shared_texture_io_surface, Default::default()) }.ok()
            } else {
                self.tile.copier.borrow_mut().copy(surface)
            };
            self.tile.copy_ms.set(self.tile.copy_ms.get() + started.elapsed().as_secs_f64() * 1000.0);
            if let Some(b) = buffer {
                let id = self.tile.frame_id.get() + 1;
                self.tile.frame_id.set(id);
                let pushed = self.tile.surface.push(b, id);
                if std::env::var_os("BT_DEBUG").is_some() {
                    let px = unsafe {
                        surface.lock(objc2_io_surface::IOSurfaceLockOptions::ReadOnly, null_mut());
                        let base = surface.base_address().as_ptr() as *const u8;
                        let row = surface.bytes_per_row();
                        let at = |x: usize, y: usize| { let p = base.add(y * row + x * 4); [*p, *p.add(1), *p.add(2), *p.add(3)] };
                        let v = [at(10, 10), at(surface.width() / 2, 100), at(surface.width() / 2, surface.height() / 2)];
                        surface.unlock(objc2_io_surface::IOSurfaceLockOptions::ReadOnly, null_mut());
                        v
                    };
                    eprintln!("paint tile={} {}x{} fmt={:x} px={:?} push={:?}", self.tile.id, surface.width(), surface.height(), surface.pixel_format(), px, pushed.as_ref().map(|_| ()));
                }
                self.tile.frames.set(self.tile.frames.get() + 1);
            }
        }
        fn on_paint(&self, _browser: Option<&mut Browser>, type_: PaintElementType, _dirty: Option<&[Rect]>, buffer: *const u8, width: ::std::os::raw::c_int, height: ::std::os::raw::c_int) {
            if type_ != PaintElementType::VIEW || buffer.is_null() {
                return;
            }
            let started = Instant::now();
            let n = (width * height * 4) as usize;
            let bytes = unsafe { std::slice::from_raw_parts(buffer, n) }.to_vec();
            let id = self.tile.frame_id.get() + 1;
            self.tile.frame_id.set(id);
            self.tile.copy_ms.set(self.tile.copy_ms.get() + started.elapsed().as_secs_f64() * 1000.0);
            let _ = self.tile.surface.push_bytes(rune::native_surface::ByteFrame { width: width as u32, height: height as u32, format: rune::native_surface::Format::Bgra, color: Default::default(), planes: [bytes, vec![]] }, id);
            self.tile.frames.set(self.tile.frames.get() + 1);
        }
        fn on_ime_composition_range_changed(&self, _browser: Option<&mut Browser>, _range: Option<&Range>, bounds: Option<&[Rect]>) {
            if let Some(b) = bounds.and_then(|b| b.last()) {
                self.tile.caret.set(Some([(b.x + b.width) as f32, b.y as f32, 2.0, b.height as f32]));
            }
        }
    }
}

wrap_dev_tools_message_observer! {
    struct TileDevTools {
        tile: Rc<TileShared>,
    }
    impl DevToolsMessageObserver {
        fn on_dev_tools_method_result(&self, _browser: Option<&mut Browser>, message_id: ::std::os::raw::c_int, success: ::std::os::raw::c_int, result: Option<&[u8]>) {
            if let Some(done) = self.tile.pending.borrow_mut().remove(&message_id) {
                done(success != 0, result.unwrap_or(&[]));
            }
        }
    }
}

wrap_client! {
    struct TileClient {
        render: RenderHandler,
    }
    impl Client {
        fn render_handler(&self) -> Option<RenderHandler> {
            Some(self.render.clone())
        }
    }
}

wrap_pdf_print_callback! {
    struct PdfDone {
        done: Rc<RefCell<Option<Box<dyn FnOnce(bool, usize)>>>>,
        path: String,
    }
    impl PdfPrintCallback {
        fn on_pdf_print_finished(&self, _path: Option<&CefString>, ok: ::std::os::raw::c_int) {
            let bytes = std::fs::metadata(&self.path).map(|m| m.len() as usize).unwrap_or(0);
            if let Some(d) = self.done.borrow_mut().take() { d(ok != 0, bytes) }
        }
    }
}

struct Open {
    browser: Browser,
    shared: Rc<TileShared>,
    _devtools: Option<Registration>,
}

pub struct Cef {
    handle: AppHandle<Proto>,
    started: bool,
    #[cfg(target_os = "macos")]
    _loader: Option<library_loader::LibraryLoader>,
    tiles: HashMap<usize, Open>,
    pressed: HashMap<usize, (i32, i32)>,
    /// Keys sent down to a page, by (tile, macOS key code), with the character their key-up repeats.
    downs: HashMap<(usize, i32), u16>,
    next_id: i32,
    scale: f32,
    pub started_at: Option<Instant>,
}

const SHIFT: u32 = 1 << 1;
const CONTROL: u32 = 1 << 2;
const ALT: u32 = 1 << 3;
const LEFT_BUTTON: u32 = 1 << 4;
const COMMAND: u32 = 1 << 7;

fn flags(m: rune::Mods) -> u32 {
    (if m.shift { SHIFT } else { 0 }) | (if m.ctrl { CONTROL } else { 0 }) | (if m.alt { ALT } else { 0 }) | (if m.cmd { COMMAND } else { 0 })
}

impl Cef {
    pub fn start(handle: AppHandle<Proto>) -> Self {
        #[cfg(target_os = "macos")]
        let scale = objc2_app_kit::NSScreen::mainScreen(objc2::MainThreadMarker::new().unwrap()).map(|s| s.backingScaleFactor() as f32).unwrap_or(2.0);
        #[cfg(not(target_os = "macos"))]
        let scale = 1.0;
        Cef { handle, started: false, #[cfg(target_os = "macos")] _loader: None, tiles: HashMap::new(), pressed: HashMap::new(), downs: HashMap::new(), next_id: 1, scale, started_at: None }
    }

    fn ensure_started(&mut self) {
        if self.started {
            return;
        }
        let t0 = Instant::now();
        #[cfg(target_os = "macos")]
        {
            let loader = library_loader::LibraryLoader::new(&std::env::current_exe().unwrap(), false);
            assert!(loader.load(), "load the CEF framework");
            self._loader = Some(loader);
        }
        let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
        #[cfg(target_os = "macos")]
        conform_app_class();
        let h = self.handle.clone();
        start_pump_thread(move || h.wake());
        let args = Args::new();
        let cache = std::env::var("HOME").unwrap() + "/.cache/attn-browser-proto/profile";
        let settings = Settings {
            windowless_rendering_enabled: 1,
            external_message_pump: 1,
            no_sandbox: 1,
            root_cache_path: cache.as_str().into(),
            cache_path: (cache.clone() + "/default").as_str().into(),
            persist_session_cookies: 1,
            log_severity: LogSeverity::WARNING,
            ..Default::default()
        };
        let mut app = ProtoApp::new();
        assert_eq!(initialize(Some(args.as_main_args()), Some(&settings), Some(&mut app), null_mut()), 1, "cef initialize");
        self.started = true;
        self.started_at = Some(Instant::now());
        crate::drive::say("cef", serde_json::json!({"step": "initialized", "ms": t0.elapsed().as_millis() as u64}));
    }

    fn host(&self, tile: usize) -> Option<cef::BrowserHost> {
        self.tiles.get(&tile).and_then(|o| o.browser.host())
    }

    fn send_key(&self, tile: usize, kind: KeyEventType, native: i32, vk: i32, ch: u16, mods: u32) {
        if let Some(h) = self.host(tile) {
            let ev = cef::KeyEvent { size: std::mem::size_of::<cef::sys::cef_key_event_t>(), type_: kind, modifiers: mods, windows_key_code: vk, native_key_code: native, is_system_key: 0, character: ch, unmodified_character: ch, focus_on_editable_field: 0 };
            h.send_key_event(Some(&ev));
        }
    }

    fn devtools(&mut self, tile: usize, method: &str, params: serde_json::Value, done: Box<dyn FnOnce(bool, &[u8])>) -> bool {
        let Some(o) = self.tiles.get(&tile) else { return false };
        let Some(h) = o.browser.host() else { return false };
        let id = self.next_id;
        self.next_id += 1;
        o.shared.pending.borrow_mut().insert(id, done);
        let msg = serde_json::json!({"id": id, "method": method, "params": params}).to_string();
        h.send_dev_tools_message(Some(msg.as_bytes())) != 0
    }

    fn stats_json(&self) -> serde_json::Value {
        let tiles: Vec<_> = self.tiles.values().map(|o| serde_json::json!({"tile": o.shared.id, "frames": o.shared.frames.get(), "copyMs": o.shared.copy_ms.get(), "surfaceError": o.shared.surface.last_error()})).collect();
        serde_json::json!({"pumpRuns": PUMP_RUNS.load(std::sync::atomic::Ordering::Relaxed), "tiles": tiles})
    }
}

/// CefRange::InvalidRange: CEF drops an IME call whose range arguments are missing.
const NO_RANGE: Range = Range { from: u32::MAX, to: u32::MAX };

fn b64(data: &str) -> Vec<u8> {
    let val = |c: u8| match c { b'A'..=b'Z' => c - b'A', b'a'..=b'z' => c - b'a' + 26, b'0'..=b'9' => c - b'0' + 52, b'+' => 62, b'/' => 63, _ => 0 };
    let bytes: Vec<u8> = data.bytes().filter(|c| *c != b'=' && !c.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, c)| acc | (val(*c) as u32) << (18 - 6 * i));
        out.push((n >> 16) as u8);
        if chunk.len() > 2 { out.push((n >> 8) as u8) }
        if chunk.len() > 3 { out.push(n as u8) }
    }
    out
}

impl BrowserHost for Cef {
    fn open(&mut self, tile: usize, url: &str, surface: &NativeSurface) {
        self.ensure_started();
        let shared = Rc::new(TileShared {
            id: tile,
            size: Cell::new([800.0, 600.0]),
            scale: self.scale,
            surface: surface.clone(),
            caret: Cell::new(None),
            #[cfg(target_os = "macos")]
            copier: RefCell::new(Copier::new()),
            frames: Cell::new(0),
            frame_id: Cell::new(0),
            copy_ms: Cell::new(0.0),
            pending: RefCell::new(HashMap::new()),
        });
        let window_info = WindowInfo { windowless_rendering_enabled: 1, shared_texture_enabled: cfg!(target_os = "macos") as i32, external_begin_frame_enabled: 0, ..Default::default() };
        let settings = BrowserSettings { windowless_frame_rate: 60, background_color: 0xFFFFFFFF, ..Default::default() };
        let mut client = TileClient::new(TileRender::new(shared.clone()));
        let browser = browser_host_create_browser_sync(Some(&window_info), Some(&mut client), Some(&url.into()), Some(&settings), None, None).expect("create browser");
        let devtools = browser.host().and_then(|h| h.add_dev_tools_message_observer(Some(&mut TileDevTools::new(shared.clone()))));
        self.tiles.insert(tile, Open { browser, shared, _devtools: devtools });
    }
    fn close(&mut self, tile: usize) {
        if let Some(o) = self.tiles.remove(&tile) {
            if let Some(h) = o.browser.host() {
                h.close_browser(1);
            }
            o.shared.surface.clear();
        }
    }
    fn place(&mut self, tile: usize, r: [f32; 4]) {
        crate::drive::note_rect(tile, r);
        if let Some(o) = self.tiles.get(&tile) {
            if o.shared.size.get() != [r[2], r[3]] {
                o.shared.size.set([r[2], r[3]]);
                if let Some(h) = o.browser.host() {
                    h.was_resized();
                }
            }
        }
    }
    fn focus(&mut self, tile: usize, focused: bool) {
        if let Some(h) = self.host(tile) {
            h.set_focus(focused as i32);
        }
    }
    fn mouse(&mut self, tile: usize, x: f32, y: f32, phase: rune::Phase, clicks: u32, mods: rune::Mods) {
        let Some(h) = self.host(tile) else { return };
        let ev = MouseEvent { x: x as i32, y: y as i32, modifiers: flags(mods) | if phase == rune::Phase::Move { LEFT_BUTTON } else { 0 } };
        match phase {
            rune::Phase::Down => h.send_mouse_click_event(Some(&ev), MouseButtonType::LEFT, 0, clicks as i32),
            rune::Phase::Up => h.send_mouse_click_event(Some(&ev), MouseButtonType::LEFT, 1, clicks as i32),
            rune::Phase::Move => h.send_mouse_move_event(Some(&ev), 0),
        }
    }
    fn hover(&mut self, tile: usize, x: f32, y: f32, inside: bool) {
        if let Some(h) = self.host(tile) {
            h.send_mouse_move_event(Some(&MouseEvent { x: x as i32, y: y as i32, modifiers: 0 }), (!inside) as i32);
        }
    }
    fn wheel(&mut self, tile: usize, ev: WheelEvent) {
        if std::env::var_os("BT_DEBUG").is_some() {
            eprintln!("wheel tile={tile} at {},{} d {},{} precise={}", ev.x, ev.y, ev.dx, ev.dy, ev.precise);
        }
        if let Some(h) = self.host(tile) {
            h.send_mouse_wheel_event(Some(&MouseEvent { x: ev.x as i32, y: ev.y as i32, modifiers: flags(ev.mods) }), ev.dx as i32, ev.dy as i32);
        }
    }
    fn physical(&mut self, tile: usize, ev: &PhysicalInput) {
        if let PhysicalInput::Key { key, mods, pressed, .. } = ev {
            let Some((native, vk)) = crate::keys::codes(*key) else { return };
            if *pressed {
                self.pressed.insert(tile, (native, vk));
            } else {
                if let Some(ch) = self.downs.remove(&(tile, native)) {
                    self.send_key(tile, KeyEventType::KEYUP, native, vk, ch, flags(*mods));
                }
            }
        }
    }
    fn key(&mut self, tile: usize, ev: &KeyEvent) -> bool {
        let (native, vk) = self.pressed.get(&tile).copied().unwrap_or((0, 0));
        let m = flags(ev.mods);
        match &ev.key {
            Key::Text(t) => {
                let mut units = t.encode_utf16();
                let first = units.next().unwrap_or(0);
                self.send_key(tile, KeyEventType::RAWKEYDOWN, native, vk, first, m);
                self.downs.insert((tile, native), first);
                for u in t.encode_utf16() {
                    self.send_key(tile, KeyEventType::CHAR, native, vk, u, m);
                }
                true
            }
            Key::Enter => {
                self.send_key(tile, KeyEventType::RAWKEYDOWN, 0x24, 0x0D, 13, m);
                self.send_key(tile, KeyEventType::CHAR, 0x24, 0x0D, 13, m);
                self.downs.insert((tile, 0x24), 13);
                true
            }
            Key::Command(c) => {
                let Some(frame) = self.tiles.get(&tile).and_then(|o| o.browser.main_frame()) else { return false };
                match c {
                    'a' => frame.select_all(),
                    'c' => frame.copy(),
                    'v' => frame.paste(),
                    'x' => frame.cut(),
                    'z' if ev.mods.shift => frame.redo(),
                    'z' => frame.undo(),
                    _ => return false,
                }
                true
            }
            Key::Backspace | Key::Delete | Key::Left | Key::Right | Key::Up | Key::Down | Key::Home | Key::End | Key::PageUp | Key::PageDown | Key::Tab | Key::Escape => {
                let ch = crate::keys::mac_char(native);
                self.send_key(tile, KeyEventType::RAWKEYDOWN, native, vk, ch, m);
                self.downs.insert((tile, native), ch);
                true
            }
            _ => false,
        }
    }
    fn ime(&mut self, tile: usize, ev: &Ime) -> bool {
        let Some(h) = self.host(tile) else { return false };
        match ev {
            Ime::Preedit(text, sel) if !text.is_empty() => {
                let len = text.encode_utf16().count() as u32;
                let to_utf16 = |byte: usize| text[..byte.min(text.len())].encode_utf16().count() as u32;
                let (from, to) = sel.map(|(a, b)| (to_utf16(a), to_utf16(b))).unwrap_or((len, len));
                let underline = CompositionUnderline { size: std::mem::size_of::<cef::sys::cef_composition_underline_t>(), range: Range { from: 0, to: len }, color: 0xFF000000, background_color: 0, thick: 0, style: CompositionUnderlineStyle::SOLID };
                h.ime_set_composition(Some(&text.as_str().into()), Some(&[underline]), Some(&NO_RANGE), Some(&Range { from, to }));
            }
            Ime::Preedit(..) => h.ime_cancel_composition(),
            Ime::Commit(text) => h.ime_commit_text(Some(&text.as_str().into()), Some(&NO_RANGE), 0),
            _ => return false,
        }
        true
    }
    fn pump(&mut self) {
        if self.started {
            PUMP_RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            do_message_loop_work();
            schedule(MAX_PUMP_DELAY_MS);
        }
    }
    fn stats(&self) -> serde_json::Value {
        self.stats_json()
    }
    fn caret(&self, tile: usize) -> Option<[f32; 4]> {
        self.tiles.get(&tile).and_then(|o| o.shared.caret.get())
    }
    fn eval(&mut self, tile: usize, js: &str, done: Box<dyn FnOnce(String)>) -> bool {
        self.devtools(tile, "Runtime.evaluate", serde_json::json!({"expression": js, "returnByValue": true}), Box::new(move |ok, bytes| {
            let v: serde_json::Value = serde_json::from_slice(bytes).unwrap_or_default();
            let value = &v["result"]["value"];
            done(if !ok { format!("<error {}>", String::from_utf8_lossy(bytes)) } else if let Some(s) = value.as_str() { s.to_owned() } else { value.to_string() })
        }))
    }
    fn capture(&mut self, tile: usize, what: &str, path: &str, done: Box<dyn FnOnce(bool, usize)>) -> bool {
        let path = path.to_owned();
        let out = path.clone(); let write = move |data: Vec<u8>| -> usize { let n = data.len(); let _ = std::fs::write(&out, data); n };
        match what {
            "screenshot" => self.devtools(tile, "Page.captureScreenshot", serde_json::json!({"format": "png"}), Box::new(move |ok, bytes| {
                let v: serde_json::Value = serde_json::from_slice(bytes).unwrap_or_default();
                let n = write(b64(v["data"].as_str().unwrap_or("")));
                done(ok, n)
            })),
            "pdf" => {
                let Some(h) = self.host(tile) else { return false };
                let mut cb = PdfDone::new(Rc::new(RefCell::new(Some(done))), path.clone());
                h.print_to_pdf(Some(&path.as_str().into()), Some(&PdfPrintSettings::default()), Some(&mut cb));
                true
            }
            "cookies" => self.devtools(tile, "Network.getCookies", serde_json::json!({}), Box::new(move |ok, bytes| { let n = write(bytes.to_vec()); done(ok, n) })),
            "ax-tree" => self.devtools(tile, "Accessibility.getFullAXTree", serde_json::json!({}), Box::new(move |ok, bytes| { let n = write(bytes.to_vec()); done(ok, n) })),
            _ => false,
        }
    }
}
