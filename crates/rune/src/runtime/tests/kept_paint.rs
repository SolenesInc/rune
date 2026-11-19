use super::*;
use crate::el::{ScrollSpec, text};

fn list(_: &(), _: &mut Cx<()>) -> El {
    col()
        .key("list")
        .size(200.0, 100.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .children((0..40).map(|i| {
            text(format!("row {i}"))
                .key(format!("row{i}"))
                .w_full()
                .h(20.0)
        }))
}

/// Lands everything, then takes the quiet frame that paints what the last one did.
fn at_rest<S: 'static>(rt: &mut Runtime<S>) {
    rt.settle_all();
    while rt.frame(1.0 / 60.0) {}
    rt.frame(1.0 / 60.0);
}

#[test]
fn a_frame_that_changes_nothing_keeps_its_paint_however_many_elements_are_mounted() {
    let mut rt = Runtime::new((), list, (200.0, 100.0), Env::new(true, 1.0));
    at_rest(&mut rt);
    let painted = rt.stats.paints;
    for _ in 0..10 {
        assert!(!rt.frame(1.0 / 60.0), "a kept frame wants no other");
    }
    assert_eq!(
        rt.stats.paints, painted,
        "ten frames that changed nothing painted nothing anew"
    );
}

#[test]
fn a_wheel_or_a_change_of_state_paints_the_next_frame() {
    let mut rt = Runtime::new((), list, (200.0, 100.0), Env::new(true, 1.0));
    at_rest(&mut rt);
    let painted = rt.stats.paints;
    rt.wheel(100.0, 50.0, -40.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(
        rt.stats.paints,
        painted + 1,
        "a wheel moves the list without a commit, so it paints"
    );
    at_rest(&mut rt);
    let painted = rt.stats.paints;
    rt.update(|_, _| {});
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.stats.paints, painted + 1, "a change of state paints");
}

#[test]
fn a_wheel_that_dismisses_a_tooltip_and_scrolls_nothing_paints_it_away() {
    fn tipped(_: &(), _: &mut Cx<()>) -> El {
        col()
            .size(200.0, 100.0)
            .child(el().key("tipped").size(100.0, 40.0).help("hello"))
    }
    let mut rt = Runtime::new((), tipped, (200.0, 100.0), Env::new(true, 1.0));
    rt.help_delay = 0.0;
    rt.frame(1.0 / 60.0);
    rt.pointer_move(50.0, 20.0);
    at_rest(&mut rt);
    assert!(rt.help_tooltip().is_some(), "the tooltip opened");
    let (painted, drawn) = (rt.stats.paints, rt.instances.len());
    rt.wheel(50.0, 20.0, -40.0);
    rt.frame(1.0 / 60.0);
    assert!(
        rt.help_tooltip().is_none(),
        "the wheel dismissed the tooltip"
    );
    assert_eq!(rt.stats.paints, painted + 1, "dismissing it paints");
    assert!(rt.instances.len() < drawn, "the tooltip is no longer drawn");
}

#[test]
fn a_caret_that_moves_within_its_element_moves_the_input_method_and_paints_nothing() {
    use crate::Live;
    fn editor(caret: &Live<[f32; 4]>, _: &mut Cx<Live<[f32; 4]>>) -> El {
        col()
            .size(200.0, 100.0)
            .child(text("an editor the app draws").key("text"))
            .child(el().key("cells").size(160.0, 80.0).ime_cursor_at(caret))
    }
    let caret = Live::new([8.0, 0.0, 1.0, 16.0]);
    let mut rt = Runtime::new(caret.clone(), editor, (200.0, 100.0), Env::new(true, 2.0));
    at_rest(&mut rt);
    let painted = rt.stats.paints;
    let [x, y, ..] = rt.ime_area.expect("the caret has the input method");
    caret.set([40.0, 32.0, 1.0, 16.0]);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.stats.paints, painted, "moving the caret paints nothing");
    assert_eq!(
        rt.ime_area,
        Some([x + 32.0, y + 32.0, 1.0, 16.0]),
        "the input method follows the caret"
    );
}

#[test]
fn a_fling_from_rest_moves_the_content_on_the_next_frame() {
    fn strip(_: &(), _: &mut Cx<()>) -> El {
        col().key("strip").size(200.0, 100.0).scroll_x().child(
            (0..10).fold(crate::el::row(), |r, i| {
                r.child(el().key(format!("tile{i}")).size(100.0, 100.0))
            }),
        )
    }
    let x_of = |rt: &Runtime<()>, key: &str| {
        rt.elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap()
            .rect[0]
    };
    let y_of = |rt: &Runtime<()>, key: &str| {
        rt.elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(key))
            .unwrap()
            .rect[1]
    };

    let mut rt = Runtime::new((), list, (200.0, 100.0), Env::new(true, 1.0));
    at_rest(&mut rt);
    let y = y_of(&rt, "row1");
    rt.fling(100.0, 50.0, -2000.0);
    rt.frame(1.0 / 60.0);
    assert!(y_of(&rt, "row1") < y, "a vertical fling moves the list");

    let mut rt = Runtime::new((), strip, (200.0, 100.0), Env::new(true, 1.0));
    at_rest(&mut rt);
    let x = x_of(&rt, "tile1");
    rt.fling_xy(100.0, 50.0, -2000.0, 0.0);
    rt.frame(1.0 / 60.0);
    assert!(x_of(&rt, "tile1") < x, "a horizontal fling moves the strip");
}

#[test]
fn a_motion_frozen_by_a_zero_time_scale_keeps_its_paint() {
    fn sized(width: &f32, _: &mut Cx<f32>) -> El {
        col()
            .size(100.0, 50.0)
            .child(el().key("box").size(*width, 8.0))
    }
    let mut rt = Runtime::new(12.0, sized, (100.0, 50.0), Env::default());
    rt.set_system_reduced_motion(false);
    rt.time_scale = 0.0;
    at_rest(&mut rt);
    rt.update(|width, _| *width = 20.0);
    for _ in 0..4 {
        rt.frame(1.0 / 60.0);
    }
    let painted = rt.stats.paints;
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.stats.paints, painted, "a frozen motion paints as it did");
    assert_eq!(
        rt.presented("box").unwrap().0[2],
        12.0,
        "a zero time scale holds the box where it was"
    );
}
