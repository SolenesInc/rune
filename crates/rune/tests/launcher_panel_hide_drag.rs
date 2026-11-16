//! A launcher panel (`WindowOptions::launcher_panel`) keeps a hidden app hidden: ⌘H holds while
//! the panel prewarms, opens and closes. A press and drag on the panel's `.window_drag()`
//! surface moves the panel. Posts pointer events, so it only runs with RUNE_GUI_TESTS=1 in a
//! logged-in macOS GUI session whose process may post events.

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod macos_support;

#[cfg(target_os = "macos")]
fn main() { mac::main() }

#[cfg(target_os = "macos")]
mod mac {
    use std::sync::Mutex;
    use std::time::Duration;
    use objc2::{class, msg_send, runtime::{AnyObject, Bool}};
    use objc2_foundation::{NSPoint, NSRect};
    use rune::*;
    use crate::macos_support::{can_post_events, on_main, wait};

    const SIZE: (f32, f32) = (240.0, 160.0);
    const AT: (f32, f32) = (300.0, 300.0);
    const TRAVEL: (f64, f64) = (-112.0, -90.0);

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventCreateMouseEvent(source: *const std::ffi::c_void, kind: u32, at: NSPoint, button: u32) -> *mut std::ffi::c_void;
        fn CGEventPost(tap: u32, event: *mut std::ffi::c_void);
        fn CFRelease(object: *mut std::ffi::c_void);
    }

    fn view(_: &(), _: &mut Cx<()>) -> El { el().w(SIZE.0).h(SIZE.1).window_drag() }

    fn shared_app() -> *mut AnyObject { unsafe { msg_send![class!(NSApplication), sharedApplication] } }

    fn app_hidden() -> bool {
        let hidden: Bool = unsafe { msg_send![shared_app(), isHidden] };
        hidden.as_bool()
    }

    fn hide_app() { unsafe { let _: () = msg_send![shared_app(), hide: std::ptr::null::<AnyObject>()]; } }

    fn unhide_app() { unsafe { let _: () = msg_send![shared_app(), unhideWithoutActivation]; } }

    /// The launcher's NSPanel.
    fn panel() -> *mut AnyObject {
        unsafe {
            let windows: *mut AnyObject = msg_send![shared_app(), windows];
            let count: usize = msg_send![windows, count];
            (0..count).map(|i| -> *mut AnyObject { msg_send![windows, objectAtIndex: i] })
                .find(|&window| { let panel: Bool = msg_send![window, isKindOfClass: class!(NSPanel)]; panel.as_bool() })
                .expect("the launcher's panel")
        }
    }

    /// The panel's frame in AppKit's bottom-left screen coordinates.
    fn panel_frame() -> (f64, f64, f64, f64) {
        let frame: NSRect = unsafe { msg_send![panel(), frame] };
        (frame.origin.x, frame.origin.y, frame.size.width, frame.size.height)
    }

    /// Whether the panel takes presses where the pointer is. It passes them through until it
    /// has seen the pointer over its surface.
    fn panel_takes_presses() -> bool {
        let ignores: Bool = unsafe { msg_send![panel(), ignoresMouseEvents] };
        !ignores.as_bool()
    }

    fn post(kind: u32, at: NSPoint) {
        unsafe {
            let event = CGEventCreateMouseEvent(std::ptr::null(), kind, at, 0);
            CGEventPost(0, event);
            CFRelease(event);
        }
        std::thread::sleep(Duration::from_millis(16));
    }

    /// The panel's middle, in Quartz's top-left screen points, where `open` places it.
    fn middle() -> NSPoint { NSPoint::new(f64::from(AT.0 + SIZE.0 / 2.0), f64::from(AT.1 + SIZE.1 / 2.0)) }

    /// Presses the panel's middle, drags it by `TRAVEL` in steps and releases.
    fn drag_surface() {
        let start = middle();
        post(1, start);
        for step in 1..=10 {
            let t = f64::from(step) / 10.0;
            post(6, NSPoint::new(start.x + TRAVEL.0 * t, start.y + TRAVEL.1 * t));
        }
        post(2, NSPoint::new(start.x + TRAVEL.0, start.y + TRAVEL.1));
    }

    fn open(handle: &AppHandle<()>) {
        handle.place(DisplayTarget::Primary, OverlayPlacement::Rect { x: AT.0, y: AT.1, width: SIZE.0, height: SIZE.1 });
        handle.open();
        wait("the panel to show", || handle.is_visible());
        std::thread::sleep(Duration::from_millis(200));
    }

    fn close(handle: &AppHandle<()>) {
        handle.hide();
        wait("the panel to hide", || !handle.is_visible());
        std::thread::sleep(Duration::from_millis(200));
    }

    /// The panel lets presses through until it has seen the pointer over its surface, and the
    /// window server applies that change some time after AppKit reports it.
    fn the_surface_drags_the_panel(handle: &AppHandle<()>) {
        open(handle);
        post(5, middle());
        wait("the panel to take presses", || on_main(handle, panel_takes_presses));
        std::thread::sleep(Duration::from_millis(300));
        let before = on_main(handle, panel_frame);
        drag_surface();
        wait("the panel to move", || on_main(handle, panel_frame) != before);
        let after = on_main(handle, panel_frame);
        let moved = (after.0 - before.0, before.1 - after.1);
        assert!((moved.0 - TRAVEL.0).abs() < 4.0 && (moved.1 - TRAVEL.1).abs() < 4.0,
            "the panel moved {moved:?} for a pointer travel of {TRAVEL:?}");
        println!("the surface drags the panel: moved {moved:?}");
        close(handle);
    }

    fn a_hidden_app_stays_hidden(handle: &AppHandle<()>) {
        on_main(handle, hide_app);
        wait("the app to hide", || on_main(handle, app_hidden));
        std::thread::sleep(Duration::from_millis(500));
        assert!(on_main(handle, app_hidden), "the hidden launcher's prewarm unhid the app");
        open(handle);
        assert!(on_main(handle, app_hidden), "opening the panel unhid the app");
        handle.hide();
        wait("the panel to hide", || !handle.is_visible());
        assert!(on_main(handle, app_hidden), "closing the panel unhid the app");
        open(handle);
        handle.hide();
        std::thread::sleep(Duration::from_millis(300));
        assert!(on_main(handle, app_hidden), "closing the panel right after it opened unhid the app");
        println!("a hidden app stays hidden");
        on_main(handle, unhide_app);
    }

    pub fn main() {
        if std::env::var_os("RUNE_GUI_TESTS").is_none() {
            println!("launcher_panel_hide_drag: skipped (set RUNE_GUI_TESTS=1 in a GUI session)");
            return;
        }
        assert!(can_post_events(), "launcher_panel_hide_drag posts pointer events: grant this process Accessibility");
        std::thread::spawn(|| { std::thread::sleep(Duration::from_secs(60)); eprintln!("timeout"); std::process::exit(2) });
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let app = App::new(AppSpec::new("launcher panel", SIZE, || (), view)).args(Vec::new()).keep_running()
            .configured_options(WindowOptions::launcher_panel())
            .on_ready(move |_| { let _ = ready_tx.send(()); });
        let handle = app.handle();
        let result = std::sync::Arc::new(Mutex::new(None));
        let out = result.clone();
        std::thread::spawn(move || {
            let run = std::panic::catch_unwind(|| {
                ready_rx.recv_timeout(Duration::from_secs(10)).expect("launcher ready");
                a_hidden_app_stays_hidden(&handle);
                the_surface_drags_the_panel(&handle);
            });
            *out.lock().unwrap() = Some(run.is_ok());
            handle.quit();
        });
        app.run();
        assert_eq!(*result.lock().unwrap(), Some(true), "launcher_panel_hide_drag failed");
        println!("launcher_panel_hide_drag: ok");
    }
}
