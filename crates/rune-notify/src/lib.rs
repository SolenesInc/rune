//! System notifications for Rune apps: post one with an id, title, body and string user info,
//! take it back out of the system's list, and hear about clicks.
//!
//! - **macOS:** `UNUserNotificationCenter`. Only a bundled app can post (`available`); the first
//!   post asks the person for permission, and posts are dropped until they allow it.
//! - **Elsewhere:** `available` is false and every call does nothing.
//!
//! Notifications are silent: they never play a sound.

use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "macos")]
mod mac;

/// One notification to post, built with `Notification::new(id, title)`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Notification {
    id: String,
    title: String,
    body: String,
    info: Vec<(String, String)>,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    actions: Vec<Action>,
}

/// A button on a notification, or a text box with its send button.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
enum Action {
    Button { id: String, label: String },
    Reply { id: String, placeholder: String, label: String },
}

/// The user-info key a click on a button carries its id under (`Click::action`).
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const ACTION: &str = "rune.action";
/// The prefix of the user-info keys a click carries a text box's words under (`Click::input`).
const INPUT: &str = "rune.input.";

impl Notification {
    /// Posting another notification with the same `id` replaces this one.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Notification { id: id.into(), title: title.into(), ..Default::default() }
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// A string the click hands back (`Click::info`), such as what to open.
    pub fn info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.info.push((key.into(), value.into()));
        self
    }

    /// A button: a click on it comes back with `Click::action` = `id`, without the app coming
    /// forward (it acts where it is). macOS shows up to ten (its banner shows two, the rest
    /// under Options). Elsewhere the notification posts without them.
    pub fn action(mut self, id: impl Into<String>, label: impl Into<String>) -> Self {
        self.actions.push(Action::Button { id: id.into(), label: label.into() });
        self
    }

    /// A text box with a send button `label`: sending comes back with `Click::action` = `id`
    /// and the words in `Click::input(id)`. macOS only.
    pub fn reply(mut self, id: impl Into<String>, placeholder: impl Into<String>, label: impl Into<String>) -> Self {
        self.actions.push(Action::Reply { id: id.into(), placeholder: placeholder.into(), label: label.into() });
        self
    }
}

/// A click on one of the app's notifications, or on its default action.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Click {
    pub id: String,
    pub info: Vec<(String, String)>,
}

impl Click {
    /// The user-info value posted under `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.info.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// The button clicked (`Notification::action`, `Notification::reply`); None for a click on
    /// the notification itself.
    pub fn action(&self) -> Option<&str> {
        self.get(ACTION)
    }

    /// What was typed in text box `id` (`Notification::reply`).
    pub fn input(&self, id: &str) -> Option<&str> {
        self.info.iter().find(|(k, _)| k.strip_prefix(INPUT) == Some(id)).map(|(_, v)| v.as_str())
    }
}

type Handler = Box<dyn Fn(Click) + Send + Sync>;

fn handler() -> &'static Mutex<Option<Handler>> {
    static HANDLER: OnceLock<Mutex<Option<Handler>>> = OnceLock::new();
    HANDLER.get_or_init(|| Mutex::new(None))
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn deliver(click: Click) {
    if let Some(f) = handler().lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        f(click);
    }
}

/// Whether this process can post: on macOS it runs from a `.app` bundle with a bundle
/// identifier.
pub fn available() -> bool {
    #[cfg(target_os = "macos")]
    {
        mac::bundled()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Posts `n` now (on macOS, asking for permission first if the person hasn't been asked).
/// Returns false when the app can't post at all (not `available`).
#[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
pub fn post(n: &Notification) -> bool {
    if !available() {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        mac::post(n.clone());
        true
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Takes the app's delivered notifications with these ids out of Notification Center.
pub fn remove(ids: &[&str]) {
    if ids.is_empty() || !available() {
        return;
    }
    #[cfg(target_os = "macos")]
    mac::remove(ids);
}

/// Calls `f` for every click on one of the app's notifications, from any thread (wake the app's
/// loop from it). macOS brings the app forward on a click; call `bring_to_front` for the window.
/// Install it at launch: on macOS a click that launched the app arrives too.
pub fn on_click(f: impl Fn(Click) + Send + Sync + 'static) {
    *handler().lock().unwrap_or_else(|e| e.into_inner()) = Some(Box::new(f));
    #[cfg(target_os = "macos")]
    if available() {
        mac::install_delegate();
    }
}

/// Whether the app is in front with one of its windows focused, the moment it matters to a
/// notification: one that would show while the person is looking at the app is noise.
pub fn app_focused() -> bool {
    #[cfg(target_os = "macos")]
    {
        mac::app_focused()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Brings the app's main window forward and focuses it, restoring it if minimized (on macOS,
/// out of the Dock). Call it on the main thread (from `poll`), say after a click.
pub fn bring_to_front() {
    #[cfg(target_os = "macos")]
    mac::bring_to_front();
}

#[cfg(test)]
mod tests;
