use rune::{Cx, El, Env, Runtime, col};
use super::Collapsible;
use crate::Kit;

#[derive(Default)]
struct S {
    general: bool,
    advanced: bool,
}

fn view(s: &S, _: &mut Cx<S>) -> El {
    let k = Kit::neutral(true);
    col()
        .w(400.0)
        .child(Collapsible::new("General", s.general).key("general").on_toggle(|s: &mut S| s.general = !s.general).build(&k))
        .child(Collapsible::new("Advanced", s.advanced).key("advanced").on_toggle(|s: &mut S| s.advanced = !s.advanced).build(&k))
}

#[test]
fn each_keyed_collapsible_is_pressed_and_focused_by_its_own_trigger() {
    let mut rt = Runtime::new(S::default(), view, (600.0, 400.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let (x, y) = rt.find("advanced-trigger").expect("the second trigger");
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    assert_eq!((rt.state().general, rt.state().advanced), (false, true));
    assert!(rt.find("advanced-content").is_some() && rt.find("general-content").is_none());

    rt.update(|_, tx| tx.focus("general-trigger", true));
    rt.frame(0.0);
    let focused = rt.elements().into_iter().find(|e| e.focused).and_then(|e| e.key);
    assert_eq!(focused.as_deref(), Some("general-trigger"));
}
