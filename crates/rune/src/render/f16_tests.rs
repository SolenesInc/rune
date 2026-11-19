//! Fade widths travel as halves. A CPU can list F16C while its OS leaves the AVX state off (a VM
//! with inconsistent CPUID, such as WSL2 under nested Hyper-V), and there F16C's VEX instructions
//! fault, so building a frame must not reach them. The shaders decode them without
//! `SHADER_FLOAT16_IN_FLOAT32`, which an adapter such as lavapipe on a CPU without AVX lacks:
//! wgpu validates each module against the adapter's capabilities, and a module that needs a
//! missing one leaves the window without a pipeline.

use super::{CLIP_WGSL, uber_wgsl, unpack_fade};
use crate::{Cx, El, Env, Runtime, TextStyle, el, text};
use wgpu::naga::{
    front::wgsl,
    valid::{Capabilities, ValidationFlags, Validator},
};

fn cut_short(_: &(), _: &mut Cx<()>) -> El {
    el().w(60.0).h(20.0).child(text("A label longer than its box").font(TextStyle::new(13.0, 500.0)).truncate_fade().shrink())
}

#[test]
fn a_frame_with_a_faded_label_builds_and_reads_its_fade_back() {
    let mut rt = Runtime::new((), cut_short, (60.0, 20.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let widest = rt.instances.iter().map(|i| unpack_fade(i.fade).0[2]).fold(0.0, f32::max);
    assert!(widest > 0.0, "the cut-short label has no right-edge fade");
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn a_cpu_listing_f16c_whose_os_keeps_avx_state_off_builds_frames() {
    let out = std::process::Command::new("sh")
        .args(["-c", "command -v qemu-x86_64 >/dev/null || exit 127; ulimit -c 0; exec qemu-x86_64 -cpu qemu64,+f16c \"$0\" --exact \"$1\""])
        .arg(std::env::current_exe().expect("test binary"))
        .arg("render::f16_tests::a_frame_with_a_faded_label_builds_and_reads_its_fade_back")
        .output()
        .expect("sh starts");
    if out.status.code() == Some(127) {
        eprintln!("skipped: qemu-x86_64 is not installed (the qemu-user package)");
        return;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success() && stdout.contains("1 passed"),
        "under an F16C CPU without AVX state: {}\n{stdout}\n{}", out.status, String::from_utf8_lossy(&out.stderr));
}

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
