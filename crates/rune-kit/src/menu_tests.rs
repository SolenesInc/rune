use rune::{Cx, El, Env, Runtime};

use super::*;
use crate::Kit;

fn open_menu(_: &(), _: &mut Cx<()>) -> El {
    let k = Kit::neutral(true);
    let mut layer = Vec::new();
    let trigger = Menu::new("session").label("Session").trigger(rune::text("Session")).item(MenuItem::new("Rename")).open(true).build(&k, &mut layer);
    rune::el().w_full().h_full().child(trigger).children(layer)
}

#[test]
fn an_open_menu_carries_its_label_as_its_accessible_name() {
    let mut rt = Runtime::new((), open_menu, (400.0, 300.0), Env { system_dark: true, scale: 1.0 });
    rt.frame(0.0);
    let tree = rt.accessibility_json();
    fn find<'a>(n: &'a serde_json::Value, role: &str) -> Option<&'a serde_json::Value> {
        if n["role"] == role { return Some(n); }
        n["children"].as_array()?.iter().find_map(|c| find(c, role))
    }
    assert_eq!(find(&tree, "menu").and_then(|m| m["name"].as_str()), Some("Session"), "{tree}");
}

fn find_key<'a>(n: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    if n["key"] == key { return Some(n); }
    n["children"].as_array()?.iter().find_map(|c| find_key(c, key))
}

fn anchored_menu(_: &(), _: &mut Cx<()>) -> El {
    let k = Kit::neutral(true);
    let mut layer = Vec::new();
    let menu = Menu::new("session")
        .label("Session")
        .anchor("row.actions")
        .item(MenuItem::new("Rename").key("session.rename"))
        .item(MenuItem::new("Close"))
        .open(true)
        .build(&k, &mut layer);
    let row_button = rune::row().key("row.actions").role("button").label("Actions").size(40.0, 20.0).absolute(300.0, 200.0);
    rune::el().w_full().h_full().child(row_button).child(menu).children(layer)
}

#[test]
fn an_item_keeps_the_key_it_was_given_and_the_rest_take_the_menus() {
    let mut rt = Runtime::new((), anchored_menu, (400.0, 300.0), Env { system_dark: true, scale: 1.0 });
    rt.frame(0.0);
    let tree = rt.accessibility_json();
    assert_eq!(find_key(&tree, "session.rename").and_then(|n| n["name"].as_str()), Some("Rename"), "{tree}");
    assert_eq!(find_key(&tree, "session-item1").and_then(|n| n["name"].as_str()), Some("Close"), "{tree}");
}

#[test]
fn an_anchored_menu_opens_beside_the_element_it_names_without_wrapping_a_trigger() {
    let mut rt = Runtime::new((), anchored_menu, (400.0, 300.0), Env { system_dark: true, scale: 1.0 });
    rt.frame(0.0);
    let tree = rt.accessibility_json();
    assert!(find_key(&tree, "session-trigger").is_none(), "{tree}");
    let bounds = |key: &str| -> [f64; 4] {
        let b = find_key(&tree, key).unwrap_or_else(|| panic!("no {key} in {tree}"))["bounds"].as_array().expect("bounds").clone();
        [0, 1, 2, 3].map(|i| b[i].as_f64().expect("a number"))
    };
    let (anchor, menu) = (bounds("row.actions"), bounds("session-menu"));
    let anchor_bottom = anchor[1] + anchor[3];
    let menu_bottom = menu[1] + menu[3];
    assert!(menu[1] >= anchor_bottom || menu_bottom <= anchor[1], "the menu sits above or below its anchor: {anchor:?} {menu:?}");
    assert!((menu[0] - anchor[0]).abs() < 1.0 || menu[0] + menu[2] <= 400.0, "{anchor:?} {menu:?}");
}

#[derive(Default)]
struct Field {
    edit: rune::TextEdit,
    open: bool,
}

fn field_and_anchored_menu(s: &Field, _: &mut Cx<Field>) -> El {
    let k = Kit::neutral(true);
    let mut layer = Vec::new();
    let menu = Menu::new("session")
        .label("Session")
        .anchor("row.actions")
        .item(MenuItem::new("Rename"))
        .open(s.open)
        .on_close(|s: &mut Field| s.open = false)
        .build(&k, &mut layer);
    let field = rune::text_input(&s.edit, rune::tok::TextStyle::new(14.0, 400.0)).key("field").w(150.0).on_edit(|s: &mut Field| &mut s.edit);
    let row_button = rune::row().key("row.actions").role("button").label("Actions").size(40.0, 20.0).focusable(true).absolute(300.0, 200.0);
    rune::el().w_full().h_full().child(field).child(row_button).child(menu).children(layer)
}

#[test]
fn an_anchored_menu_gives_focus_back_to_where_it_was_not_to_its_anchor() {
    let mut s = Field::default();
    s.edit.focused = true;
    let mut rt = Runtime::new(s, field_and_anchored_menu, (400.0, 300.0), Env { system_dark: true, scale: 1.0 });
    rt.frame(0.0);
    let focused = |rt: &Runtime<Field>| rt.elements().iter().find(|e| e.focused).and_then(|e| e.key.clone());
    assert_eq!(focused(&rt).as_deref(), Some("field"));

    rt.state_mut().open = true;
    rt.frame(1.0 / 60.0);
    rt.key(rune::KeyEvent { key: rune::Key::Down, mods: rune::Mods::default() });
    rt.frame(1.5 / 60.0);
    assert_eq!(focused(&rt).as_deref(), Some("session-item0"));

    rt.key(rune::KeyEvent { key: rune::Key::Escape, mods: rune::Mods::default() });
    rt.frame(2.0 / 60.0);
    assert!(!rt.state().open);
    assert_eq!(focused(&rt).as_deref(), Some("field"));
}

fn standard_menu_with_its_own_look(_: &(), _: &mut Cx<()>) -> El {
    let mut k = Kit::neutral(true);
    k.standard_menu = crate::StandardMenu { edge_alpha: 1.0, radius: 8.0, inset: 5.0, row_padding: (9.0, 6.0), row_radius: 5.0 };
    k.c.surface_menu = k.c.bg_base;
    let mut layer = Vec::new();
    let menu = Menu::new("snooze").standard().anchor("row.actions").item(MenuItem::new("For an hour")).open(true).build(&k, &mut layer);
    let row_button = rune::row().key("row.actions").role("button").label("Actions").size(40.0, 20.0).absolute(20.0, 20.0);
    rune::el().w_full().h_full().child(row_button).child(menu).children(layer)
}

#[test]
fn a_standard_menu_takes_its_frame_and_rows_from_the_kit_s_standard_menu_and_menu_fill() {
    let mut rt = Runtime::new((), standard_menu_with_its_own_look, (400.0, 300.0), Env { system_dark: true, scale: 1.0 });
    rt.frame(0.0);
    let element = |key: &str| rt.elements().iter().find(|e| e.key.as_deref() == Some(key)).cloned().unwrap_or_else(|| panic!("no {key}"));
    let (frame, row) = (element("snooze-menu"), element("snooze-item0"));
    let neutral = Kit::neutral(true);
    assert_eq!((frame.radius, frame.fill), (8.0, neutral.c.bg_base));
    assert_eq!(row.radius, 5.0);
    let (menu, item) = (frame.rect, row.rect);
    let line = neutral.ty.text_small_medium.line_height;
    assert_eq!(item[0] - menu[0], 5.0, "{menu:?} {item:?}");
    assert_eq!(item[1] - menu[1], 5.0, "{menu:?} {item:?}");
    assert_eq!(item[3], line + 12.0, "{item:?}");
}

fn standard_menu_with_quiet_choices(_: &(), _: &mut Cx<()>) -> El {
    let mut k = Kit::neutral(true);
    k.standard_menu_label = Some(crate::MenuLabel { font: k.ty.text_small, ink: k.c.text_muted, focused_ink: k.c.text_strong });
    let mut layer = Vec::new();
    let menu = Menu::new("snooze")
        .standard()
        .anchor("row.actions")
        .item(MenuItem::new("For an hour").detail("11:00"))
        .note("esc cancel")
        .open(true)
        .build(&k, &mut layer);
    let row_button = rune::row().key("row.actions").role("button").label("Actions").size(40.0, 20.0).absolute(20.0, 20.0);
    rune::el().w_full().h_full().child(row_button).child(menu).children(layer)
}

#[test]
fn a_standard_menu_draws_its_labels_details_and_note_in_the_kit_s_menu_text() {
    let mut rt = Runtime::new((), standard_menu_with_quiet_choices, (400.0, 300.0), Env { system_dark: true, scale: 1.0 });
    rt.frame(0.0);
    let elements = rt.elements();
    let text_of = |said: &str| elements.iter().find(|e| e.text.as_deref() == Some(said)).cloned().unwrap_or_else(|| panic!("no text {said}"));
    let neutral = Kit::neutral(true);
    assert_eq!(text_of("For an hour").text_color, Some(neutral.c.text_muted));
    let detail = text_of("11:00");
    assert_eq!((detail.text_color, detail.opacity), (Some(neutral.c.text_faint), 0.75));
    assert_eq!(text_of("esc cancel").text_color, Some(neutral.c.text_muted));
    let row = elements.iter().find(|e| e.key.as_deref() == Some("snooze-item0")).expect("the item");
    assert_eq!(row.label.as_deref(), Some("For an hour"));
    let (detail_right, row_right) = (detail.rect[0] + detail.rect[2], row.rect[0] + row.rect[2]);
    assert!(row_right - detail_right <= 9.0, "the detail ends the row: {:?} {:?}", detail.rect, row.rect);
}

fn standard_menu_of_quiet_choices(open: &bool, _: &mut Cx<bool>) -> El {
    let mut k = Kit::neutral(true);
    k.standard_menu_label = Some(crate::MenuLabel { font: k.ty.text_small, ink: k.c.text_muted, focused_ink: k.c.text_strong });
    let mut layer = Vec::new();
    let menu = Menu::new("snooze")
        .standard()
        .anchor("row.actions")
        .item(MenuItem::new("For an hour"))
        .item(MenuItem::new("Until tomorrow"))
        .open(*open)
        .on_close(|open: &mut bool| *open = false)
        .build(&k, &mut layer);
    let row_button = rune::row().key("row.actions").role("button").label("Actions").size(40.0, 20.0).absolute(20.0, 20.0);
    rune::el().w_full().h_full().child(row_button).child(menu).children(layer)
}

#[test]
fn the_row_the_keys_or_pointer_reach_draws_its_label_in_the_focused_ink() {
    let mut rt = Runtime::new(false, standard_menu_of_quiet_choices, (400.0, 300.0), Env { system_dark: true, scale: 1.0 });
    rt.frame(0.0);
    rt.update(|open, _| *open = true);
    rt.frame(2.0 / 60.0);
    let neutral = Kit::neutral(true);
    let (rest, lit) = (Some(neutral.c.text_muted), Some(neutral.c.text_strong));
    let inks = |rt: &Runtime<bool>| {
        let elements = rt.elements();
        let ink = |said: &str| elements.iter().find(|e| e.text.as_deref() == Some(said)).and_then(|e| e.text_color);
        (ink("For an hour"), ink("Until tomorrow"))
    };
    assert_eq!(inks(&rt), (rest, rest));

    rt.key(rune::KeyEvent { key: rune::Key::Down, mods: rune::Mods::default() });
    rt.frame(3.0 / 60.0);
    assert_eq!(inks(&rt), (lit, rest));

    rt.key(rune::KeyEvent { key: rune::Key::Down, mods: rune::Mods::default() });
    rt.frame(4.0 / 60.0);
    assert_eq!(inks(&rt), (rest, lit));

    let first = rt.elements().iter().find(|e| e.key.as_deref() == Some("snooze-item0")).expect("the first row").rect;
    rt.pointer_move(first[0] + first[2] / 2.0, first[1] + first[3] / 2.0);
    rt.frame(5.0 / 60.0);
    assert_eq!(inks(&rt), (lit, rest));
}

fn standard_menu_with_inset_separators(_: &(), _: &mut Cx<()>) -> El {
    let mut k = Kit::neutral(true);
    k.standard_menu = crate::StandardMenu { edge_alpha: 1.0, radius: 8.0, inset: 5.0, row_padding: (9.0, 6.0), row_radius: 5.0 };
    let mut layer = Vec::new();
    let menu = Menu::new("session")
        .standard()
        .anchor("row.actions")
        .separator_inset(3.0)
        .item(MenuItem::new("Rename session"))
        .separator()
        .item(MenuItem::new("Close session"))
        .open(true)
        .build(&k, &mut layer);
    let row_button = rune::row().key("row.actions").role("button").label("Actions").size(40.0, 20.0).absolute(20.0, 20.0);
    rune::el().w_full().h_full().child(row_button).child(menu).children(layer)
}

#[test]
fn a_menus_separators_sit_its_separator_inset_inside_its_rows() {
    let mut rt = Runtime::new((), standard_menu_with_inset_separators, (400.0, 300.0), Env { system_dark: true, scale: 1.0 });
    rt.frame(0.0);
    let element = |key: &str| rt.elements().iter().find(|e| e.key.as_deref() == Some(key)).cloned().unwrap_or_else(|| panic!("no {key}"));
    let (menu, rule) = (element("session-menu").rect, element("sep1").rect);
    assert_eq!(rule[0] - menu[0], 8.0, "{menu:?} {rule:?}");
    assert_eq!((menu[0] + menu[2]) - (rule[0] + rule[2]), 8.0, "{menu:?} {rule:?}");
}
