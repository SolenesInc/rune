//! `App::on_input`: the app hears each input the person gives the window, before any handler
//! does, as a web page's capture-phase listeners on `window` do.
use super::*;

impl RuntimeCore {
    /// Tells the app's `on_input` hook that the person gave the window an input. What the hook
    /// changes marks nothing dirty: it follows the person, not the view.
    pub(crate) fn heard_input(&mut self) {
        if self.app.has(app_side::Hook::Input) {
            self.update_app(&mut |app, tx| { app.input(tx); false });
        }
    }
}
