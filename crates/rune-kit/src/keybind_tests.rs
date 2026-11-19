use rune::{Cx, El, Env, Runtime};

use super::*;

fn wide_cap(_: &Kit, _: String) -> El {
    row().size(30.0, 20.0)
}

fn shortcut(_: &(), _: &mut Cx<()>) -> El {
    let mut k = Kit::neutral(true);
    k.keycap = Some(wide_cap);
    rune::el().w_full().h_full().child(Keybind::new(&["⌘", "k"]).build(&k))
}

#[test]
fn a_kit_s_keycap_draws_each_key() {
    let mut rt = Runtime::new((), shortcut, (400.0, 300.0), Env { system_dark: true, scale: 1.0 });
    rt.frame(0.0);
    let tree = rt.accessibility_json();
    fn find<'a>(n: &'a serde_json::Value, role: &str) -> Option<&'a serde_json::Value> {
        if n["role"] == role { return Some(n); }
        n["children"].as_array()?.iter().find_map(|c| find(c, role))
    }
    let group = find(&tree, "group").unwrap_or_else(|| panic!("{tree}"));
    assert_eq!(group["name"], "⌘ k", "{tree}");
    assert_eq!(group["bounds"][2].as_f64(), Some(62.0), "two 30-point caps 2 points apart: {tree}");
}
