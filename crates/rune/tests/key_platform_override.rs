use rune::{Chord, Key, KeyEvent, KeyPlatform, Mods};

#[test]
fn the_key_platform_variable_runs_the_other_platforms_keymap() {
    let other = if cfg!(target_os = "macos") { KeyPlatform::Linux } else { KeyPlatform::Mac };
    let name = if other == KeyPlatform::Mac { "mac" } else { "linux" };
    unsafe { std::env::set_var("RUNE_KEY_PLATFORM", name) };

    assert_eq!(KeyPlatform::current(), other);
    let chord: Chord = "mod+b".parse().unwrap();
    assert_eq!(chord.label(), if other == KeyPlatform::Mac { "⌘B" } else { "Ctrl+B" });
    let control_b = KeyEvent { key: Key::Command('b'), mods: Mods { ctrl: true, ..Mods::default() } };
    assert_eq!(chord.matches(&control_b), other == KeyPlatform::Linux, "Control+B is the primary chord only off macOS");
}
