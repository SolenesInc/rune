//! The operating system an app acts as: its keymap, its title bar and its field menu. A
//! window takes it from the host; a run without one can act as the other, so a test suite gives
//! the same result on every machine.

/// The platform whose conventions keys, chords, the title bar and the field menu follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    /// ⌘ is the primary modifier; Control is a separate key. Glyph labels: "⇧⌘T".
    Mac,
    /// Ctrl is the primary modifier. Word labels, the logo key is "Super". The web uses this
    /// in a browser that isn't on an Apple device.
    Linux,
}

#[cfg(not(target_arch = "wasm32"))]
static PLATFORM: std::sync::OnceLock<Platform> = std::sync::OnceLock::new();

impl Platform {
    /// The platform `RUNE_PLATFORM` names (`mac` or `linux`), else the one
    /// [`Platform::set_default`] gave, else the one this build runs on. Read once per process.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn current() -> Self {
        *PLATFORM.get_or_init(|| Self::named_by_env().unwrap_or_else(Self::host))
    }

    /// Makes `platform` this process's platform unless `RUNE_PLATFORM` names one, so a test
    /// harness acts as one platform on every machine while `cargo run` keeps the host's. Call it
    /// before anything asks [`Platform::current`]; it panics when an earlier read settled on
    /// another platform.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_default(platform: Self) {
        let named = Self::named_by_env();
        let current = *PLATFORM.get_or_init(|| named.unwrap_or(platform));
        assert!(
            named.is_some() || current == platform,
            "Platform::current() was read as {current:?} before Platform::set_default({platform:?})"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn named_by_env() -> Option<Self> {
        match std::env::var("RUNE_PLATFORM").as_deref() {
            Ok("mac") => Some(Self::Mac),
            Ok("linux") => Some(Self::Linux),
            Ok(other) => panic!("RUNE_PLATFORM is {other:?}; expected mac or linux"),
            Err(_) => None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn host() -> Self {
        if cfg!(any(target_os = "macos", target_os = "ios")) { Self::Mac } else { Self::Linux }
    }

    /// The platform the browser runs on: `Mac` when `navigator.platform` names an Apple
    /// device (as the browser's own shortcuts do), else `Linux`.
    #[cfg(target_arch = "wasm32")]
    pub fn current() -> Self {
        static PLATFORM: std::sync::OnceLock<Platform> = std::sync::OnceLock::new();
        *PLATFORM.get_or_init(|| {
            let platform = js_sys::Reflect::get(&js_sys::global(), &"navigator".into())
                .and_then(|navigator| js_sys::Reflect::get(&navigator, &"platform".into()))
                .ok().and_then(|p| p.as_string()).unwrap_or_default();
            if ["Mac", "iPod", "iPhone", "iPad"].iter().any(|apple| platform.contains(apple)) { Self::Mac } else { Self::Linux }
        })
    }

    pub(crate) fn mac(self) -> bool { matches!(self, Self::Mac) }
}
