use super::*;
use crate::el::{ScrollSpec, text};

fn list(_: &(), _: &mut Cx<()>) -> El {
    col()
        .key("list")
        .size(200.0, 100.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .children((0..40).map(|i| text(format!("row {i}")).key(format!("row{i}")).w_full().h(20.0)))
}

/// Lands everything, then takes the quiet frame that paints what the last one did.
fn at_rest(rt: &mut Runtime<()>) {
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
    assert_eq!(rt.stats.paints, painted, "ten frames that changed nothing painted nothing anew");
}

#[test]
fn a_wheel_or_a_change_of_state_paints_the_next_frame() {
    let mut rt = Runtime::new((), list, (200.0, 100.0), Env::new(true, 1.0));
    at_rest(&mut rt);
    let painted = rt.stats.paints;
    rt.wheel(100.0, 50.0, -40.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.stats.paints, painted + 1, "a wheel moves the list without a commit, so it paints");
    at_rest(&mut rt);
    let painted = rt.stats.paints;
    rt.update(|_, _| {});
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.stats.paints, painted + 1, "a change of state paints");
}
