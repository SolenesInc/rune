//! `text::without_system_fonts` makes text load no installed font: monospace text measures with
//! the font the app gave, and text the app's fonts lack finds no system font.
use rune::{Family, TextStyle, text::{TextSystem, without_system_fonts}};

const INTER: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");
const PLEX: &[u8] = include_bytes!("../assets/IBMPlexMono-Regular.ttf");

#[test]
fn text_measures_with_the_apps_fonts_only() {
    without_system_fonts(PLEX);
    let mut system = TextSystem::new(&[INTER], ("Inter", "Menlo, DejaVu Sans Mono, Liberation Mono, ui-monospace"));
    let fonts = |system: &mut TextSystem, text: &str, style: TextStyle| -> Vec<Vec<u8>> {
        let layout = system.layout_rich(text, &[], style, 2.0, None);
        layout.lines().flat_map(|l| l.runs().map(|r| r.font().data.as_ref().to_vec()).collect::<Vec<_>>()).collect()
    };
    let style = TextStyle::new(13.0, 400.0);
    for (what, style) in [("the mono slot", style.mono()), ("Family::SystemMono", style.family(Family::SystemMono))] {
        let used = fonts(&mut system, "fn main() {}", style);
        assert!(!used.is_empty() && used.iter().all(|f| f == PLEX), "{what} measures with the app's mono");
    }
    let used = fonts(&mut system, "Hello 世界 ⎋ Ω", style);
    assert!(used.iter().all(|f| f == INTER || f == PLEX), "no system font fills what the app's fonts lack");
    without_system_fonts(PLEX);
    let refused = std::panic::catch_unwind(|| without_system_fonts(INTER));
    assert!(refused.is_err(), "another mono after the first panics");
}
