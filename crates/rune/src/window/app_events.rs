//! What the whole app hears, not one window: a quit (`crate::quit`) and a reopen.

use super::*;

impl Windows {
    /// The app starts to quit (`crate::quit`): every window leaves the screen, so the quit looks
    /// done the moment it starts, then every window still open runs `App::on_quit` unseen, once
    /// per quit.
    pub(super) fn quit(&mut self) {
        use crate::quit::{After, Effect, Event};
        for host in self.hosts.iter_mut().flatten() { host.vanish(); }
        crate::marks::mark("quit.hidden");
        for effect in crate::quit::send(Event::Start { after: After::Exit }) {
            match effect {
                Effect::RunHooks => {
                    for host in self.hosts.iter_mut().flatten() { host.quit_hook(); }
                    crate::marks::mark("quit.hooks");
                    crate::quit::send(Event::HooksRan);
                }
                // Only the loop's end relaunches.
                Effect::Relaunch => {}
            }
        }
    }

    /// The app was reopened: every `on_reopen` hook runs, then, unless one handled it or the
    /// app is an accessory, the default brings back a window when none shows (`reopen::target`).
    pub(super) fn reopen(&mut self, event_loop: &ActiveEventLoop) {
        let candidates = |hosts: &[Option<Box<dyn AnyHost>>]| -> Vec<_> {
            hosts.iter().map(|h| h.as_ref().map(|h| h.reopen_candidate()).unwrap_or_default()).collect()
        };
        let has_visible = crate::platform::reopen::has_visible(&candidates(&self.hosts));
        let mut handled = false;
        for host in self.hosts.iter_mut().flatten() {
            handled |= host.reopen_hook(event_loop, has_visible);
        }
        if handled || event_loop.exiting() { return; }
        #[cfg(target_os = "macos")]
        if crate::platform::reopen::accessory() { return; }
        // The default sees what the hooks did, to other windows too.
        for host in self.hosts.iter_mut().flatten() { host.apply_pending(event_loop); }
        if event_loop.exiting() { return; }
        #[cfg(target_os = "macos")]
        crate::platform::reopen::unhide();
        if let Some(Some(host)) = crate::platform::reopen::target(&candidates(&self.hosts)).and_then(|i| self.hosts.get_mut(i)) {
            host.reopen(event_loop);
        }
    }
}
