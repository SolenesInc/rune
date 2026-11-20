//! `El::group_clip` moves a clip in while its hover group is hovered, so a row's words end in a
//! fade before the controls its hover shows over them.

use super::*;
use crate::{Cx, Direction, El, Env, col, el, row};

/// A 200 point row at y 20 whose words fade their right 16 points, and on hover end 60 points
/// sooner, where its controls show.
fn row_with_words(dir: &Direction, _: &mut Cx<Direction>) -> El {
    col().w(300.0).h(100.0).child(el().w(1.0).h(20.0)).child(
        row().key("row").w(200.0).h(28.0).dir(*dir).hover_group().child(
            row().key("words").w_full().h_full().clip_fade(0.0, 0.0, 16.0, 0.0).group_clip([0.0; 4], [0.0, 0.0, 60.0, 0.0])
                .child(el().key("ink").w_full().h_full().fill(crate::Color::WHITE)),
        ),
    )
}

fn words(rt: &Runtime<Direction>) -> ([f32; 4], [f32; 4]) {
    let i = rt.instances.iter().find(|i| i.fill[3] > 0.99 && i.params[1] == crate::render::KIND_RECT).expect("the words' ink");
    (i.clip, crate::render::unpack_fade(i.fade).0)
}

fn runtime(dir: Direction, scale: f32) -> Runtime<Direction> {
    let mut rt = Runtime::new(dir, row_with_words, (300.0, 100.0), Env::new(true, scale));
    rt.frame(0.0);
    rt
}

#[test]
fn at_rest_the_clip_is_the_box() {
    for scale in [1.0, 2.0] {
        let rt = runtime(Direction::Ltr, scale);
        let (clip, widths) = words(&rt);
        assert_eq!(clip, [0.0, 20.0, 200.0, 48.0].map(|v| v * scale), "{scale}x");
        assert!((widths[2] - 16.0 * scale).abs() < 0.01, "{scale}x: {widths:?}");
    }
}

#[test]
fn hovering_moves_the_clip_in_on_the_hover_frame_with_its_fade() {
    for scale in [1.0, 2.0] {
        let mut rt = runtime(Direction::Ltr, scale);
        rt.pointer_move(20.0, 30.0);
        rt.frame(1.0 / 120.0);
        let (clip, widths) = words(&rt);
        assert_eq!(clip, [0.0, 20.0, 140.0, 48.0].map(|v| v * scale), "{scale}x: the words end where the controls start");
        assert!((widths[2] - 16.0 * scale).abs() < 0.01, "{scale}x: the fade moves with the edge: {widths:?}");
        assert!(!rt.animating, "{scale}x: no frame owed after the hover frame");
    }
}

#[test]
fn leaving_puts_the_clip_back_at_the_box_on_the_first_frame() {
    let mut rt = runtime(Direction::Ltr, 1.0);
    rt.pointer_move(20.0, 30.0);
    rt.frame(1.0 / 120.0);
    rt.pointer_move(250.0, 90.0);
    rt.frame(1.0 / 120.0);
    assert_eq!(words(&rt).0, [0.0, 20.0, 200.0, 48.0], "the clip moves after the pointer left");
    for _ in 0..60 {
        rt.frame(1.0 / 120.0);
        assert_eq!(words(&rt).0, [0.0, 20.0, 200.0, 48.0], "the clip moves after the pointer left");
    }
    assert!(!rt.animating);
}

#[test]
fn right_to_left_the_clip_moves_in_from_the_left() {
    let mut rt = runtime(Direction::Rtl, 1.0);
    rt.pointer_move(180.0, 30.0);
    rt.frame(1.0 / 120.0);
    let (clip, widths) = words(&rt);
    assert_eq!(clip, [60.0, 20.0, 200.0, 48.0]);
    assert!(widths[0] > 0.0 && widths[2] == 0.0, "the fade sits on the left: {widths:?}");
}
