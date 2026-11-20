use super::*;

#[derive(Default)]
struct Acts {
    pressed: Option<u64>,
}

fn acts_view(_: &Acts, _: &mut Cx<Acts>) -> El {
    col()
        .key("body")
        .size(300.0, 200.0)
        .scroll(crate::ScrollSpec {
            stick_bottom: false,
        })
        .child(
            el().key("go")
                .size(200.0, 40.0)
                .on_press_tx(|s: &mut Acts, tx: &mut Tx| s.pressed = Some(tx.inputs())),
        )
        .child(el().size(200.0, 600.0))
}

fn acts() -> Runtime<Acts> {
    let mut rt = Runtime::new(
        Acts::default(),
        acts_view,
        (300.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.window_focus(true);
    rt.frame(0.0);
    rt
}

fn inputs(rt: &mut Runtime<Acts>) -> u64 {
    let mut seen = 0;
    rt.update(|_, tx| seen = tx.inputs());
    rt.frame(0.0);
    seen
}

fn text(s: &str) -> KeyEvent {
    KeyEvent {
        key: crate::Key::Text(s.into()),
        mods: Default::default(),
    }
}

#[test]
fn each_act_of_the_person_counts_once_and_motion_releases_repeats_and_preedit_do_not() {
    let mut rt = acts();
    let mut last = inputs(&mut rt);
    let mut step =
        |rt: &mut Runtime<Acts>, what: &str, act: &dyn Fn(&mut Runtime<Acts>), counts: bool| {
            act(rt);
            rt.frame(0.0);
            let now = inputs(rt);
            assert_eq!(now - last, u64::from(counts), "{what}");
            last = now;
        };
    step(&mut rt, "a key", &|rt| rt.key(text("a")), true);
    step(
        &mut rt,
        "its auto-repeat",
        &|rt| rt.repeat_key(text("a")),
        false,
    );
    step(
        &mut rt,
        "pointer motion",
        &|rt| rt.pointer_move(10.0, 10.0),
        false,
    );
    step(&mut rt, "a press", &|rt| rt.pointer_down(10.0, 10.0), true);
    step(
        &mut rt,
        "its release",
        &|rt| rt.pointer_up(10.0, 10.0),
        false,
    );
    step(&mut rt, "a wheel", &|rt| rt.wheel(10.0, 10.0, -20.0), true);
    step(
        &mut rt,
        "a two-axis wheel",
        &|rt| rt.wheel_xy(10.0, 10.0, 0.0, -20.0),
        true,
    );
    step(
        &mut rt,
        "a secondary press",
        &|rt| rt.secondary_down(10.0, 10.0),
        true,
    );
    step(
        &mut rt,
        "its release",
        &|rt| rt.secondary_up(10.0, 10.0),
        false,
    );
    step(
        &mut rt,
        "a composition",
        &|rt| rt.ime(crate::Ime::Preedit("か".into(), None)),
        false,
    );
    step(
        &mut rt,
        "its committed text",
        &|rt| rt.ime(crate::Ime::Commit("か".into())),
        true,
    );
}

#[test]
fn a_handler_reads_the_count_as_of_its_own_input_though_later_input_queued_before_it_ran() {
    let mut rt = acts();
    let before = inputs(&mut rt);
    let (x, y) = rt.find("go").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.key(text("a"));
    rt.frame(0.0);
    assert_eq!(
        rt.state().pressed,
        Some(before + 1),
        "the press reads its own count"
    );
    assert_eq!(
        inputs(&mut rt),
        before + 2,
        "a handler no input caused reads the count now"
    );
}

#[derive(Default)]
struct Leaving {
    edit: crate::TextEdit,
    refused: bool,
    keep: bool,
}

/// A field whose blur mounts a refusal under it, above a control near the bottom of the view.
fn leaving_view(s: &Leaving, _: &mut Cx<Leaving>) -> El {
    col()
        .key("body")
        .size(300.0, 200.0)
        .scroll(crate::ScrollSpec {
            stick_bottom: false,
        })
        .child(
            crate::text_input(&s.edit, crate::TextStyle::new(14.0, 400.0))
                .key("field")
                .size(200.0, 40.0)
                .on_edit(|s: &mut Leaving| &mut s.edit)
                .on_blur(|s: &mut Leaving, tx: &mut Tx| {
                    s.refused = true;
                    if s.keep {
                        tx.keep_focus_on_screen();
                    }
                }),
        )
        .children(s.refused.then(|| el().key("refusal").size(200.0, 40.0)))
        .child(el().size(200.0, 110.0))
        .child(el().key("next").size(200.0, 30.0).focusable(true))
        .child(el().size(200.0, 400.0))
        .child(el().key("far").size(200.0, 30.0).focusable(true))
}

fn leaving(keep: bool) -> Runtime<Leaving> {
    let mut rt = Runtime::new(
        Leaving {
            keep,
            ..Leaving::default()
        },
        leaving_view,
        (300.0, 200.0),
        Env::new(true, 1.0),
    );
    rt.window_focus(true);
    rt.frame(0.0);
    let (x, y) = rt.find("field").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.settle_all();
    for _ in 0..3 {
        rt.frame(1.0 / 120.0);
    }
    rt
}

fn shown(rt: &Runtime<Leaving>, key: &str) -> crate::ElementInfo {
    rt.elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("no {key}"))
}

fn click_next(keep: bool) -> ([f32; 4], crate::ElementInfo, bool) {
    let mut rt = leaving(keep);
    let before = shown(&rt, "next").rect;
    let (x, y) = rt.find("next").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.settle_all();
    rt.frame(0.0);
    let focused = rt.focus.path().is_some_and(|p| p.ends_with("next"));
    (before, shown(&rt, "next"), focused)
}

#[test]
fn a_blur_that_keeps_the_focus_on_screen_keeps_the_clicked_control_where_it_was_clicked() {
    let (before, after, focused) = click_next(true);
    assert!(focused, "the click focused it");
    assert_eq!(
        after.rect[1], before[1],
        "it stays where the person clicked it"
    );
    assert_eq!(after.visible[3], after.rect[3], "and whole");

    let (before, after, _) = click_next(false);
    assert_eq!(
        after.rect[1],
        before[1] + 40.0,
        "without it, the refusal pushes it down"
    );
    assert!(after.visible[3] < after.rect[3], "partly out of view");
}

#[test]
fn a_focus_the_last_frame_drew_out_of_view_is_still_revealed() {
    let mut rt = leaving(true);
    rt.key(KeyEvent {
        key: crate::Key::Tab,
        mods: Default::default(),
    });
    rt.key(KeyEvent {
        key: crate::Key::Tab,
        mods: Default::default(),
    });
    rt.settle_all();
    rt.frame(0.0);
    assert!(
        rt.focus.path().is_some_and(|p| p.ends_with("far")),
        "Tab reached the control below the view"
    );
    let far = shown(&rt, "far");
    assert_eq!(
        far.visible[3], far.rect[3],
        "Tab's reveal brought it whole into view"
    );
}
