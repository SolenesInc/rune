//! The app's jump list: what the Dock icon's menu (macOS) offers besides the app's windows, as
//! tasks (New window) and named categories of recent things (Recent projects), when the app's
//! own menus put nothing in the Dock. Each entry carries one argument, which arrives as the link
//! the app opens (`App::on_open_url`) in the running app, so such a link is the natural
//! argument.
//! Linux's counterpart, the launcher's actions, are static in a `.desktop` file, not built here.

/// One entry: a title, and the single argument the app starts with when it's picked.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JumpItem {
    pub(crate) title: String,
    pub(crate) argument: String,
    pub(crate) description: String,
}

impl JumpItem {
    pub fn new(title: impl Into<String>, argument: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            argument: argument.into(),
            description: String::new(),
        }
    }

    /// The tooltip over the entry.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

/// Tasks first, then each category under its name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct JumpList {
    pub(crate) tasks: Vec<JumpItem>,
    pub(crate) categories: Vec<(String, Vec<JumpItem>)>,
    pub(crate) lead: Option<String>,
}

impl JumpList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn task(mut self, item: JumpItem) -> Self {
        self.tasks.push(item);
        self
    }

    /// One argument every entry starts the app with before its own (Windows): a run's own data
    /// folder, so a pick reaches that run and not the app's usual profile.
    pub fn lead(mut self, argument: impl Into<String>) -> Self {
        self.lead = Some(argument.into());
        self
    }

    /// A category of entries under `name`; an empty one isn't shown.
    pub fn category(mut self, name: impl Into<String>, items: Vec<JumpItem>) -> Self {
        if !items.is_empty() {
            self.categories.push((name.into(), items));
        }
        self
    }
}

/// Replaces the app's jump list with `list`: on macOS the Dock menu (call it on the main
/// thread). Does nothing elsewhere, and for headless runs.
pub fn set_jump_list(list: JumpList) {
    if crate::is_headless() {
        return;
    }
    #[cfg(target_os = "macos")]
    crate::native_menu::set_jump_menu(&list);
    #[cfg(not(target_os = "macos"))]
    let _ = list;
}
