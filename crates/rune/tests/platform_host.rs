//! With no `RUNE_PLATFORM`, an app acts as the host it runs on.
use rune::Platform;

#[test]
fn with_no_variable_the_platform_is_the_host() {
    unsafe { std::env::remove_var("RUNE_PLATFORM") };
    let host = if cfg!(target_os = "macos") { Platform::Mac } else { Platform::Linux };
    assert_eq!(Platform::current(), host);
}
