//! The measured option: a `WKWebView` per tile, a subview of Rune's content view, framed from
//! the tile's drawn rect. AppKit routes input to it directly, so the input methods do nothing.

use crate::BrowserHost;
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSEvent, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSURL, NSURLRequest};
use objc2_web_kit::{WKWebView, WKWebViewConfiguration};
use rune::{Ime, KeyEvent, NativeSurface, PhysicalInput, WheelEvent};
use std::collections::HashMap;

define_class!(
    #[unsafe(super(WKWebView))]
    #[thread_kind = MainThreadOnly]
    #[name = "BrowserTilesFirstMouseWebView"]
    struct FirstMouseWebView;

    impl FirstMouseWebView {
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }
    }
);

pub struct Wk {
    view: Retained<NSView>,
    config: Retained<WKWebViewConfiguration>,
    pages: HashMap<usize, Retained<WKWebView>>,
}

impl Wk {
    pub fn new(w: &rune::WindowHandle) -> Self {
        let ptr = w
            .with_raw_handle(|h| match h.as_raw() {
                raw_window_handle::RawWindowHandle::AppKit(a) => a.ns_view.as_ptr() as usize,
                _ => 0,
            })
            .unwrap_or(0);
        let view: Retained<NSView> = unsafe { Retained::retain(ptr as *mut NSView) }.expect("content view");
        let mtm = MainThreadMarker::new().unwrap();
        let config = unsafe { WKWebViewConfiguration::new(mtm) };
        Wk { view, config, pages: HashMap::new() }
    }
}

pub fn web_view_of(wk: &Wk, tile: usize) -> Option<Retained<WKWebView>> {
    wk.pages.get(&tile).cloned()
}

impl BrowserHost for Wk {
    fn open(&mut self, tile: usize, url: &str, _surface: &NativeSurface) {
        let mtm = MainThreadMarker::new().unwrap();
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(10.0, 10.0));
        let web: Retained<FirstMouseWebView> = unsafe { msg_send![FirstMouseWebView::alloc(mtm), initWithFrame: frame, configuration: &*self.config] };
        let web: Retained<WKWebView> = Retained::into_super(web);
        let url = NSURL::URLWithString(&NSString::from_str(url)).expect("url");
        if url.isFileURL() {
            let dir = url.URLByDeletingLastPathComponent().expect("dir");
            unsafe { web.loadFileURL_allowingReadAccessToURL(&url, &dir) };
        } else {
            unsafe { web.loadRequest(&NSURLRequest::requestWithURL(&url)) };
        }
        if std::env::args().any(|a| a == "--wk-ignore-occlusion") {
            let sel = objc2::sel!(_setWindowOcclusionDetectionEnabled:);
            let responds: bool = unsafe { msg_send![&*web, respondsToSelector: sel] };
            if responds {
                let _: () = unsafe { msg_send![&*web, _setWindowOcclusionDetectionEnabled: false] };
            }
            crate::drive::say("wk", serde_json::json!({"step": "ignore-occlusion", "spi": responds}));
        }
        self.view.addSubview(&web);
        self.pages.insert(tile, web);
    }
    fn close(&mut self, tile: usize) {
        if let Some(web) = self.pages.remove(&tile) {
            web.removeFromSuperview();
        }
    }
    fn place(&mut self, tile: usize, r: [f32; 4]) {
        crate::drive::note_rect(tile, r);
        if std::env::var("BT_DEBUG").is_ok() { eprintln!("place {tile} {r:?} has={}", self.pages.contains_key(&tile)); }
        if let Some(web) = self.pages.get(&tile) {
            web.setFrame(NSRect::new(NSPoint::new(r[0] as f64, r[1] as f64), NSSize::new(r[2] as f64, r[3] as f64)));
        }
    }
    fn focus(&mut self, _tile: usize, _focused: bool) {}
    fn mouse(&mut self, _: usize, _: f32, _: f32, _: rune::Phase, _: u32, _: rune::Mods) {}
    fn hover(&mut self, _: usize, _: f32, _: f32, _: bool) {}
    fn wheel(&mut self, _: usize, _: WheelEvent) {}
    fn physical(&mut self, _: usize, _: &PhysicalInput) {}
    fn key(&mut self, _: usize, _: &KeyEvent) -> bool { false }
    fn ime(&mut self, _: usize, _: &Ime) -> bool { false }
    fn eval(&mut self, tile: usize, js: &str, done: Box<dyn FnOnce(String)>) -> bool {
        let Some(web) = self.pages.get(&tile) else { return false };
        let done = std::cell::Cell::new(Some(done));
        let block = block2::RcBlock::new(move |result: *mut objc2::runtime::AnyObject, _err: *mut objc2_foundation::NSError| {
            let value = if result.is_null() { String::from("<null>") } else { unsafe { (*(result as *const NSString)).to_string() } };
            if let Some(d) = done.take() { d(value) }
        });
        unsafe { web.evaluateJavaScript_completionHandler(&NSString::from_str(js), Some(&block)) };
        true
    }
}
