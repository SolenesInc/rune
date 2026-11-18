//! Telling the platform's input method that Rune ended a composition itself (a blur, a click,
//! the window going inactive): the field already wrote what it showed, so the input method
//! drops its own copy rather than carry it into the next field. winit only switches the input
//! method off and on, which leaves the composition in AppKit's.

/// Drops the window's open composition, if any.
#[cfg(target_os = "macos")]
pub(crate) fn cancel(window: &winit::window::Window) {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let Ok(handle) = window.window_handle() else { return };
    let RawWindowHandle::AppKit(h) = handle.as_raw() else { return };
    // SAFETY: `ns_view` is the window's live content view on the main thread; `inputContext`
    // returns it or nil, and `discardMarkedText` takes no arguments.
    unsafe {
        let view = h.ns_view.as_ptr().cast::<objc2::runtime::AnyObject>();
        let context: *mut objc2::runtime::AnyObject = objc2::msg_send![view, inputContext];
        if !context.is_null() {
            let () = objc2::msg_send![context, discardMarkedText];
        }
    }
}

/// Wayland's and X11's input methods end the composition when winit turns them off, as the
/// web's text field does when it blurs.
#[cfg(not(target_os = "macos"))]
pub(crate) fn cancel(_window: &winit::window::Window) {}

/// The key AppKit is handling when it is an Option dead key, such as ⌥E on a US layout: its
/// physical key, the modifiers it carries (winit hears them only after the input method) and
/// the character it types alone. While the input method is on, winit reports
/// such a press only as the composition it opens.
#[cfg(target_os = "macos")]
pub(crate) fn dead_key_press() -> Option<(crate::PhysicalKeyCode, crate::Mods, Option<char>)> {
    use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSEventType};
    use winit::platform::scancode::PhysicalKeyExtScancode;
    let app = NSApplication::sharedApplication(objc2_foundation::MainThreadMarker::new()?);
    // SAFETY: main thread (the marker above); each call reads the key event AppKit is handling.
    let (flags, dead, code, base) = unsafe {
        let event = app.currentEvent().filter(|e| e.r#type() == NSEventType::KeyDown)?;
        (event.modifierFlags(), event.characters().is_some_and(|c| c.length() == 0), event.keyCode(),
            event.charactersIgnoringModifiers().and_then(|s| s.to_string().chars().next()))
    };
    let option_only = flags.contains(NSEventModifierFlags::NSEventModifierFlagOption)
        && !flags.intersects(NSEventModifierFlags::NSEventModifierFlagCommand | NSEventModifierFlags::NSEventModifierFlagControl);
    if !option_only || !dead { return None; }
    let winit::keyboard::PhysicalKey::Code(key) = crate::PhysicalKeyCode::from_scancode(code.into()) else { return None };
    let shift = flags.contains(NSEventModifierFlags::NSEventModifierFlagShift);
    Some((key, crate::Mods { alt: true, shift, ..crate::Mods::default() }, base))
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn dead_key_press() -> Option<(crate::PhysicalKeyCode, crate::Mods, Option<char>)> { None }
