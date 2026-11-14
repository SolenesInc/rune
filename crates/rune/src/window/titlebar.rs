//! A titlebar the app draws itself (`App::titlebar`), and where the window's own controls sit
//! over it (`Cx::chrome_insets`).
//!
//! The app declares how tall its titlebar is; on macOS Rune takes the native one away and
//! keeps AppKit's traffic lights at the leading edge over the app's. The app keeps clear of
//! [`ChromeInsets`] and marks the parts that move the window with `.window_drag()`; hit
//! targets inside them keep their presses.

use crate::Chrome;

/// The app draws its own titlebar across the top of the window, `height` points tall
/// (`App::titlebar`, `WindowSpec::titlebar`). Plain data, the same on every platform:
///
/// - **macOS:** a full-size content view under a transparent titlebar, `.window_drag()`
///   regions only (`Chrome::drag_regions`), and the traffic lights centred in the band unless
///   `Chrome::traffic_lights` places them.
/// - **Elsewhere:** the native frame stays and [`ChromeInsets`] is zero.
///
/// `Cx::titlebar_height` changes the height while the app runs, as a UI zoom does, and
/// `Cx::set_titlebar` the whole titlebar, as a switch between layouts does. A borderless window
/// has no titlebar to replace, and ignores it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Titlebar {
    height: f32,
    traffic_lights: Option<(f32, f32)>,
}

impl Titlebar {
    pub const fn new(height: f32) -> Self {
        Self { height, traffic_lights: None }
    }

    /// macOS: the close button's top-left, in points from the window's top-left, for this
    /// titlebar (Electron's `trafficLightPosition`), in place of `Chrome::traffic_lights` and
    /// the band's middle. Ignored elsewhere and in full screen.
    pub const fn traffic_lights(mut self, x: f32, y: f32) -> Self {
        self.traffic_lights = Some((x, y));
        self
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    /// Where [`Self::traffic_lights`] put the close button, if it did.
    pub fn traffic_lights_origin(&self) -> Option<(f32, f32)> {
        self.traffic_lights
    }

    /// The frame macOS draws for this titlebar over `chrome`: full-size content, drag regions
    /// only, and the traffic lights where this titlebar or the app placed them, else in the
    /// band's middle.
    pub(crate) fn mac_chrome(&self, chrome: Chrome) -> Chrome {
        let y = ((self.height - Chrome::TRAFFIC_LIGHT) / 2.0).max(0.0).round();
        let lights = self.traffic_lights.or(chrome.traffic_lights).or(Some((16.0, y)));
        Chrome { full_size_content: true, drag_regions: true, traffic_lights: lights }
    }
}

/// What the view set of the window's titlebar on its last run: `Cx::set_titlebar`, then
/// `Cx::titlebar_height` over it. Empty keeps `App::titlebar`'s.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TitlebarSet {
    pub titlebar: Option<Titlebar>,
    pub height: Option<f32>,
}

impl TitlebarSet {
    /// The titlebar the window draws in place of the app's `base`.
    pub(crate) fn over(self, base: Titlebar) -> Titlebar {
        let titlebar = self.titlebar.unwrap_or(base);
        self.height.map_or(titlebar, |height| Titlebar { height, ..titlebar })
    }
}

/// What the window's own controls take of the app's titlebar, in points
/// (`Cx::chrome_insets`). The view re-runs when it changes: entering or leaving full screen,
/// a scale change, `Cx::titlebar_height`, `Cx::set_titlebar`. Memos rebuild with it too, so a
/// `memo` closure that reads it needs nothing in its version.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct ChromeInsets {
    /// From the window's leading edge to the end of the controls there: the macOS traffic
    /// lights (`Cx::titlebar_inset`). 0 in full screen and elsewhere.
    pub leading: f32,
    /// The titlebar's height as the window draws it: [`Titlebar::height`] where Rune replaced
    /// the native titlebar, 0 where the native frame stays (no `App::titlebar`, Linux, a
    /// borderless window).
    pub height: f32,
}

impl ChromeInsets {
    pub(crate) fn new(leading: f32, height: f32) -> Self {
        Self { leading, height }
    }

    /// The insets, in window points, in the points of a view zoomed `k` times (`Cx::zoom`).
    pub(crate) fn zoomed(self, k: f32) -> Self {
        Self { leading: self.leading / k, height: self.height / k }
    }

    /// The bits a memo scope hashes, so memos rebuild when the insets change.
    pub(crate) fn bits(&self) -> [u32; 2] {
        [self.leading, self.height].map(f32::to_bits)
    }
}
