// Tests of SolenesInc/rune's patches on the `attn` branch, listed in FORK.md.

/// A press that reaches a just-shown overlay before the frame that first lays its view out
/// still starts the drag under it, as when a picker shows under a pointer already pressing.
#[test]
fn a_press_before_the_first_frame_after_show_reaches_the_view() {
    fn view(_: &Vec<crate::Phase>, _: &mut Cx<Vec<crate::Phase>>) -> El {
        crate::el().key("pad").size(200.0, 50.0)
            .on_drag(|phases: &mut Vec<crate::Phase>, d: &mut crate::Drag, _| phases.push(d.phase))
    }
    let app = AppSpec::new("early press", (200.0, 50.0), Vec::new, view);
    let (mut overlay, _) = Overlay::new(OverlaySpec::new("early press", app.size, view).options(Default::default()), Vec::new(), &app);
    overlay.rt.wall_start = None;
    // Shown, with no frame drawn yet.
    overlay.presence = Presence::Shown { occluded: false };
    overlay.rt.set_live_visible(true);
    overlay.settle_before_reveal();
    overlay.dirty = true;
    overlay.shown_for_input();
    overlay.rt.pointer_move(20.0, 20.0);
    overlay.left_button(true, None);
    overlay.rt.pointer_move(120.0, 20.0);
    overlay.left_button(false, Some((120.0, 20.0)));
    present(&mut overlay, Instant::now());
    let phases = &overlay.rt.state;
    assert_eq!(phases.first(), Some(&crate::Phase::Down), "{phases:?}");
    assert_eq!(phases.last(), Some(&crate::Phase::Up), "{phases:?}");
}
