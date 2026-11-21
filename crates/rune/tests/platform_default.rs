//! A test harness's `Platform::set_default` holds on every host unless `RUNE_PLATFORM` names a
//! platform, and a read before it panics instead of acting as another platform.
use rune::Platform;

#[test]
fn a_default_holds_and_a_later_different_one_panics() {
    unsafe { std::env::remove_var("RUNE_PLATFORM") };
    let other = if cfg!(target_os = "macos") { Platform::Linux } else { Platform::Mac };
    Platform::set_default(other);
    Platform::set_default(other);
    assert_eq!(Platform::current(), other, "the default beats the host");
    let host = if other == Platform::Mac { Platform::Linux } else { Platform::Mac };
    let refused = std::panic::catch_unwind(|| Platform::set_default(host));
    assert!(refused.is_err(), "a default after the platform settled on another one panics");
}
