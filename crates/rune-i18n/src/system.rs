//! The languages the user prefers, most wanted first, as BCP 47 tags.

/// The OS's preferred languages: macOS' and iOS' `NSLocale.preferredLanguages`, the browser's `navigator.languages`, and the
/// POSIX locale variables elsewhere ([`posix_languages`]).
pub fn system_languages() -> Vec<String> {
    platform()
}

/// The languages POSIX locale variables name, read through `var`: `LANGUAGE`'s colon list, then
/// `LC_ALL`, `LC_MESSAGES` and `LANG`, each turned from `de_AT.UTF-8@euro` into `de-AT`, without
/// `C`, `POSIX` and repeats.
pub fn posix_languages(var: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let names = var("LANGUAGE").unwrap_or_default();
    let singles = ["LC_ALL", "LC_MESSAGES", "LANG"].map(|name| var(name).unwrap_or_default());
    for value in names.split(':').chain(singles.iter().map(String::as_str)) {
        let tag = value.split(['.', '@']).next().unwrap_or("").replace('_', "-");
        if tag.is_empty() || tag == "C" || tag == "POSIX" || out.contains(&tag) {
            continue;
        }
        out.push(tag);
    }
    out
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn platform() -> Vec<String> {
    objc2_foundation::NSLocale::preferredLanguages().iter().map(|tag| tag.to_string()).collect()
}

#[cfg(target_arch = "wasm32")]
fn platform() -> Vec<String> {
    let Some(window) = web_sys::window() else { return Vec::new() };
    window.navigator().languages().iter().filter_map(|tag| tag.as_string()).collect()
}

#[cfg(not(any(target_os = "macos", target_os = "ios", target_arch = "wasm32")))]
fn platform() -> Vec<String> {
    posix_languages(|name| std::env::var_os(name).and_then(|value| value.into_string().ok()))
}
