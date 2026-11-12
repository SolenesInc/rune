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

/// `RUNE_OVERLAY_FRAME_LOG`: an app reads an overlay's frame times from the log.
#[test]
fn each_presented_frame_is_one_line_in_the_log() {
    let path = std::env::temp_dir().join(format!("rune-overlay-frame-log-{}.jsonl", std::process::id()));
    // SAFETY: no other test in this process reads or writes this variable.
    unsafe { std::env::set_var("RUNE_OVERLAY_FRAME_LOG", &path) };
    let started = Instant::now();
    super::draw::frame_log::drew("picker", started);
    super::draw::frame_log::drew("picker", started);
    let text = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    let lines: Vec<serde_json::Value> = text.lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(lines.len(), 2, "{text}");
    for line in &lines {
        assert_eq!(line["name"], "picker");
        assert!(line["ms"].as_f64().is_some_and(|ms| ms >= 0.0), "{line}");
    }
    assert!(lines[1]["t_ms"].as_f64() >= lines[0]["t_ms"].as_f64(), "{text}");
}
