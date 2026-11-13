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
