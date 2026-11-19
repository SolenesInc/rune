//! Rune's shaders build on an adapter without `SHADER_FLOAT16_IN_FLOAT32`, such as lavapipe on a
//! CPU without AVX: wgpu validates each module against the adapter's capabilities, and a module
//! that needs a missing one leaves the window without a pipeline.

use super::{CLIP_WGSL, uber_wgsl};
use wgpu::naga::{
    front::wgsl,
    valid::{Capabilities, ValidationFlags, Validator},
};

fn validates_without_float16(name: &str, source: &str) {
    let module = wgsl::parse_str(source).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
    Validator::new(ValidationFlags::all(), Capabilities::all() - Capabilities::SHADER_FLOAT16_IN_FLOAT32)
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
}

#[test]
fn every_shader_with_rune_clip_maths_builds_without_float16_in_float32() {
    let modules = [
        ("uber", uber_wgsl::CLASSIC.to_string()),
        ("uber windfoil", uber_wgsl::WINDFOIL.to_string()),
        ("uber springs", format!("{CLIP_WGSL}\n{}\n{}", include_str!("../shader.wgsl"), crate::gpu::springs::SHADER)),
        ("backdrop composite", format!("{CLIP_WGSL}\n{}", include_str!("../backdrop_composite.wgsl"))),
        ("video", format!("{CLIP_WGSL}\n{}", include_str!("../native_surface/video.wgsl"))),
    ];
    for (name, source) in &modules {
        validates_without_float16(name, source);
    }
}
