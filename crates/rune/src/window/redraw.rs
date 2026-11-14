//! When each window's next frame is asked for. A frame a window asks for while it paints is
//! asked for at the end of the loop's turn (`Windows::about_to_wait`).

/// One window's next frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Redraw {
    /// No frame is on its way, and none is wanted.
    Idle,
    /// The system was asked for a paint, which hasn't come yet.
    Asked,
    /// The window paints, and nothing has asked for another frame since it started.
    Painting,
    /// The window paints, and wants the next frame.
    PaintingAgain,
    /// The window painted and wants the next frame, which the turn's end asks for.
    Wants,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum RedrawEvent {
    /// Something asks for a frame: an input, a wake, a timer, the frame that just drew.
    Ask,
    /// A paint starts.
    Paint,
    /// The paint ends.
    Painted,
    /// The loop's turn ends.
    TurnEnds,
}

/// What the window does for a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RedrawEffect {
    None,
    /// `Window::request_redraw`.
    Request,
}

impl Redraw {
    pub(super) fn step(self, event: RedrawEvent) -> (Redraw, RedrawEffect) {
        use {Redraw::*, RedrawEffect as E, RedrawEvent::*};
        match (self, event) {
            // Asked inside the paint: winit would ask the system again before the paint returns.
            (Painting | PaintingAgain, Ask) => (PaintingAgain, E::None),
            (Asked, Ask) => (Asked, E::Request),
            (Idle | Wants, Ask) => (Asked, E::Request),
            (_, Paint) => (Painting, E::None),
            (Painting, Painted) => (Idle, E::None),
            (PaintingAgain, Painted) => (Wants, E::None),
            (state, Painted) => (state, E::None),
            (Wants, TurnEnds) => (Asked, E::Request),
            (state, TurnEnds) => (state, E::None),
        }
    }
}
