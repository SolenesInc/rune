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
