//! `App::on_input`: the app hears each input the person gives the window, before any handler
//! does, as a web page's capture-phase listeners on `window` do.
use super::*;

/// What an input the window heard was: an act of the person, which `Tx::inputs` counts, or
/// input that only follows one, such as pointer motion, a release or a key's auto-repeat.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Heard {
    Act,
    Following,
}

impl Heard {
    pub(crate) fn of(act: bool) -> Heard {
        if act { Heard::Act } else { Heard::Following }
    }

    /// A physical key's first press is an act, unless the key is a modifier.
    pub(crate) fn physical(event: &crate::PhysicalInput) -> Heard {
        use crate::PhysicalKeyCode as Code;
        Heard::of(matches!(event, crate::PhysicalInput::Key { key, pressed: true, repeat: false, .. }
            if !matches!(key, Code::ShiftLeft | Code::ShiftRight | Code::ControlLeft | Code::ControlRight
                | Code::AltLeft | Code::AltRight | Code::SuperLeft | Code::SuperRight | Code::CapsLock | Code::Fn)))
    }
}

impl RuntimeCore {
    /// Every input entry point calls this once, before it queues or runs anything: an act
    /// advances `Tx::inputs`, and the app's `on_input` hook hears the input. What the hook
    /// changes marks nothing dirty: it follows the person, not the view.
    pub(crate) fn heard_input(&mut self, heard: Heard) {
        if heard == Heard::Act { crate::input::acts::act(); }
        if self.app.has(app_side::Hook::Input) {
            self.update_app(&mut |app, tx| { app.input(tx); false });
        }
    }
}
