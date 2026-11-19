use super::tests::pixels;
use super::*;
use crate::{Color, Cx, El, Env, Runtime, el};

const SIZE: (u32, u32) = (64, 32);

fn shade(v: f32) -> Color {
    Color {
        r: v,
        g: 0.2,
        b: 0.4,
        a: 1.0,
    }
}

/// A faded group of two boxes, the first filled as `inside` says,, beside a box whose fill `outside` sets.
fn view(s: &(f32, f32), _: &mut Cx<(f32, f32)>) -> El {
    crate::row()
        .size(SIZE.0 as f32, SIZE.1 as f32)
        .child(
            crate::col()
                .size(24.0, 24.0)
                .opacity(0.5)
                .child(el().size(16.0, 8.0).fill(shade(s.0)))
                .child(el().size(12.0, 8.0).fill(Color::WHITE)),
        )
        .child(el().size(24.0, 24.0).fill(shade(s.1)))
}

/// The runtime's frame at `now` seconds, prepared, and the layer passes it draws.
fn frame(now: f64, rt: &mut Runtime<(f32, f32)>, renderer: &mut Renderer, gpu: &Gpu) -> usize {
    rt.frame(now);
    let core = &mut **rt;
    renderer.prepare(
        &gpu.device,
        &gpu.queue,
        &mut core.text.atlas,
        &core.instances,
        SIZE,
    );
    renderer.layer_stats().extra_passes
}

fn fresh_pixels(state: (f32, f32), gpu: &Gpu, target: &Offscreen) -> Vec<u8> {
    let mut rt = Runtime::new(
        state,
        view,
        (SIZE.0 as f32, SIZE.1 as f32),
        Env {
            scale: 1.0,
            system_dark: true,
        },
    );
    let mut renderer = Renderer::new(&gpu.device, OFFSCREEN_FORMAT);
    frame(0.0, &mut rt, &mut renderer, gpu);
    pixels(gpu, &renderer, target)
}

#[test]
fn an_isolated_layer_whose_instances_did_not_change_composites_without_drawing_again() {
    let gpu = Gpu::headless();
    let target = Offscreen::new(&gpu.device, SIZE.0, SIZE.1);
    let mut rt = Runtime::new(
        (0.9, 0.1),
        view,
        (SIZE.0 as f32, SIZE.1 as f32),
        Env {
            scale: 1.0,
            system_dark: true,
        },
    );
    let mut renderer = Renderer::new(&gpu.device, OFFSCREEN_FORMAT);
    assert_eq!(
        frame(0.0, &mut rt, &mut renderer, &gpu),
        1,
        "the first frame draws the faded group"
    );
    pixels(&gpu, &renderer, &target);

    rt.state_mut().1 = 0.6;
    rt.invalidate();
    assert_eq!(
        frame(0.1, &mut rt, &mut renderer, &gpu),
        0,
        "a change outside the group leaves its target as drawn"
    );
    assert_eq!(
        frame(5.0, &mut rt, &mut renderer, &gpu),
        0,
        "and so does the change's motion settling"
    );
    assert_eq!(
        pixels(&gpu, &renderer, &target),
        fresh_pixels((0.9, 0.6), &gpu, &target),
        "the kept target composites what a fresh draw shows"
    );

    rt.state_mut().0 = 0.3;
    rt.invalidate();
    assert_eq!(
        frame(5.1, &mut rt, &mut renderer, &gpu),
        1,
        "a change inside the group draws it again"
    );
    frame(10.0, &mut rt, &mut renderer, &gpu);
    assert_eq!(
        pixels(&gpu, &renderer, &target),
        fresh_pixels((0.3, 0.6), &gpu, &target)
    );
    assert_eq!(
        frame(10.1, &mut rt, &mut renderer, &gpu),
        0,
        "a settled group stays as drawn"
    );
}

#[test]
fn an_isolated_layer_holding_an_effect_slot_draws_every_frame() {
    fn with_slot(s: &(f32, f32), _: &mut Cx<(f32, f32)>) -> El {
        crate::row()
            .size(SIZE.0 as f32, SIZE.1 as f32)
            .child(
                el().size(24.0, 24.0)
                    .opacity(0.5)
                    .child(el().size(16.0, 16.0).effect(7)),
            )
            .child(el().size(24.0, 24.0).fill(shade(s.1)))
    }
    let gpu = Gpu::headless();
    let mut rt = Runtime::new(
        (0.0, 0.1),
        with_slot,
        (SIZE.0 as f32, SIZE.1 as f32),
        Env {
            scale: 1.0,
            system_dark: true,
        },
    );
    let mut renderer = Renderer::new(&gpu.device, OFFSCREEN_FORMAT);
    assert_eq!(frame(0.0, &mut rt, &mut renderer, &gpu), 1);
    rt.state_mut().1 = 0.6;
    rt.invalidate();
    assert_eq!(
        frame(0.1, &mut rt, &mut renderer, &gpu),
        1,
        "a compositor draws into the slot, so Rune cannot know it is unchanged"
    );
}
