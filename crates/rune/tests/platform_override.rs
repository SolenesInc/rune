//! `RUNE_PLATFORM=mac` makes a run act as a Mac on any host: its keymap, the title bar it
//! reports without a window, its field menu, ⌘Y and the modifiers a script's `cmd` holds.
use std::sync::{Mutex, Once};

use rune::*;

fn as_mac() {
    static SET: Once = Once::new();
    SET.call_once(|| unsafe { std::env::set_var("RUNE_PLATFORM", "mac") });
    assert_eq!(Platform::current(), Platform::Mac);
}

#[test]
fn the_platform_variable_runs_the_macs_keymap() {
    as_mac();
    let chord: Chord = "mod+b".parse().unwrap();
    assert_eq!(chord.label(), "⌘B");
    let control_b = KeyEvent { key: Key::Command('b'), mods: Mods { ctrl: true, ..Mods::default() } };
    assert!(!chord.matches(&control_b), "Control+B is not the primary chord on a Mac");
}

fn chrome_view(_: &(), cx: &mut Cx<()>) -> El {
    let insets = cx.chrome_insets();
    col().child(
        row()
            .child(el().key("inset").size(cx.titlebar_inset().max(1.0), 10.0))
            .child(el().key("leading").size(insets.leading.max(1.0), 10.0))
            .child(el().key("height").size(10.0, insets.height.max(1.0))),
    )
}

#[test]
fn a_run_without_a_window_reports_the_traffic_lights_and_the_title_bar() {
    as_mac();
    let app = App::new(AppSpec::new("t", (400.0, 300.0), || (), chrome_view)).titlebar(Titlebar::new(38.0).traffic_lights(14.0, 12.0));
    let mut rt = app.headless_runtime((400.0, 300.0), Env::new(true, 2.0));
    rt.frame(0.0);
    let size = |key: &str| { let r = rt.presented(key).unwrap().0; (r[2], r[3]) };
    let lights = rune::Chrome { traffic_lights: Some((14.0, 12.0)), ..rune::Chrome::default() }.titlebar_inset();
    assert!(lights > 14.0, "three traffic lights end past the first one's origin: {lights}");
    assert_eq!(size("inset").0, lights, "Cx::titlebar_inset reports where the zoom button ends");
    assert_eq!(size("leading").0, lights, "ChromeInsets::leading keeps clear of the traffic lights");
    assert_eq!(size("height").1, 38.0, "ChromeInsets::height is the title bar's");
}

struct Field { draft: TextEdit, heard: Vec<KeyEvent> }

fn field_view(s: &Field, _: &mut Cx<Field>) -> El {
    col().w_full().h_full().padding(20.0).child(
        text_input(&s.draft, TextStyle::new(14.0, 400.0)).key("field").w(200.0).on_edit(|s: &mut Field| &mut s.draft),
    )
}

fn hear(s: &mut Field, k: &KeyEvent, _: &mut Tx) -> bool {
    s.heard.push(k.clone());
    true
}

fn field() -> Field {
    let mut draft = TextEdit::new("words");
    draft.focused = true;
    Field { draft, heard: Vec::new() }
}

fn field_runtime() -> Runtime<Field> {
    let spec = AppSpec { on_key: Some(hear), ..AppSpec::new("t", (300.0, 200.0), field, field_view) };
    let mut rt = App::new(spec).headless_runtime((300.0, 200.0), Env::new(true, 2.0));
    rt.frame(0.0);
    rt
}

#[test]
fn the_field_menu_is_the_macs() {
    as_mac();
    let mut rt = field_runtime();
    rt.secondary_down(40.0, 28.0);
    rt.secondary_up(40.0, 28.0);
    rt.frame(1.0 / 60.0);
    let menu = rt.popup_menu().expect("the field's menu opened");
    assert!(menu.find("paste").is_some(), "the menu has the edit items");
    assert!(menu.find("select-all").is_none(), "a Mac's field menu has no Select All");
}

#[test]
fn command_y_is_no_redo_and_reaches_the_app() {
    as_mac();
    let mut rt = field_runtime();
    let command_y = KeyEvent { key: Key::Command('y'), mods: Mods { cmd: true, ..Mods::default() } };
    rt.key(command_y.clone());
    rt.frame(1.0 / 60.0);
    assert!(rt.state().heard.contains(&command_y), "⌘Y is the app's on a Mac, not the field's redo");
}

static SCRIPT_MODS: Mutex<Vec<Mods>> = Mutex::new(Vec::new());

fn script_view(_: &(), _: &mut Cx<()>) -> El { el().key("root").size(10.0, 10.0) }

fn script_key(_: &mut (), k: &KeyEvent, _: &mut Tx) -> bool {
    SCRIPT_MODS.lock().unwrap().push(k.mods);
    true
}

#[test]
fn a_scripts_cmd_is_the_command_key() {
    as_mac();
    let dir = std::env::temp_dir().join(format!("rune-platform-script-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (script, tree) = (dir.join("script.json"), dir.join("tree.json"));
    std::fs::write(&script, r#"[{"t": 0.0, "type": "key", "name": "k", "cmd": true}]"#).unwrap();
    let spec = AppSpec { on_key: Some(script_key), ..AppSpec::new("t", (100.0, 100.0), || (), script_view) };
    App::new(spec).args(["--script", script.to_str().unwrap(), "--tree", tree.to_str().unwrap()].map(String::from)).run();
    std::fs::remove_dir_all(&dir).unwrap();
    let mods = SCRIPT_MODS.lock().unwrap().clone();
    assert!(!mods.is_empty(), "the script's key reached the app");
    assert!(mods.iter().all(|m| m.cmd && m.logo && !m.ctrl), "cmd holds ⌘, not Control: {mods:?}");
}
