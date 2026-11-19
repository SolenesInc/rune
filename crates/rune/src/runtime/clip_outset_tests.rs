//! `El::clip_outset` moves a clip and its fades past the element's box, so what reaches out of a
//! faded cell, such as a focus ring, is drawn whole.

use super::*;
use crate::{Cx, El, Env, el, row};

/// A 100 point cell at x 50 that fades its right 16 points, holding a box that reaches 4 points
/// past it on each side, as a focus ring does.
fn cell(outset: &[f32; 4], _: &mut Cx<[f32; 4]>) -> El {
    let [l, t, r, b] = *outset;
    row().w(300.0).h(60.0).child(el().w(50.0).h(1.0)).child(
        row().key("cell").w(100.0).h(28.0).my(16.0).clip_fade(0.0, 0.0, 16.0, 0.0).clip_outset(l, t, r, b).child(
            el().key("ring").w(108.0).h(36.0).ml(-4.0).mt(-4.0).fill(crate::Color::WHITE),
        ),
    )
}

/// The ring's drawn clip and its fade widths and origins, in device px.
fn ring(scale: f32, outset: [f32; 4]) -> ([f32; 4], [f32; 4], [f32; 4]) {
    let mut rt = Runtime::new(outset, cell, (300.0, 60.0), Env::new(true, scale));
    rt.frame(0.0);
    let i = rt.instances.iter().find(|i| i.fill[3] > 0.99 && i.params[1] == crate::render::KIND_RECT).expect("the ring's fill");
    let (widths, origins) = crate::render::unpack_fade(i.fade);
    (i.clip, widths, origins)
}

#[test]
fn a_clip_with_an_outset_reaches_past_the_box_by_it_on_each_side() {
    for scale in [1.0, 2.0] {
        let (clip, ..) = ring(scale, [4.0, 4.0, 4.0, 4.0]);
        let reach = [50.0 - 4.0, 16.0 - 4.0, 150.0 + 4.0, 44.0 + 4.0].map(|v| v * scale);
        assert!(clip.iter().zip(reach).all(|(c, r)| (c - r).abs() < 0.51), "{scale}x: clip {clip:?}, the ring reaches {reach:?}");
    }
}

#[test]
fn a_clip_without_an_outset_cuts_at_the_box() {
    let (clip, ..) = ring(1.0, [0.0; 4]);
    assert_eq!(clip, [50.0, 16.0, 150.0, 44.0]);
}

#[test]
fn the_fade_moves_out_with_the_clip_and_keeps_its_width() {
    for scale in [1.0, 2.0] {
        let (_, widths, origins) = ring(scale, [0.0, 0.0, 4.0, 0.0]);
        let (_, plain, _) = ring(scale, [0.0; 4]);
        assert_eq!(widths, plain, "{scale}x: the fade's widths");
        assert!((widths[2] - 16.0 * scale).abs() < 0.01, "{scale}x: {widths:?}");
        assert_eq!(origins[2], 0.0, "{scale}x: the fade ends at the clip, not inside it at the box");
    }
}
