//! `App::headless_runtime` runs the app as a window does.
use crate::{App, AppSpec, Cx, El, Env, Mods, PhysicalInput, el};

#[test]
fn a_headless_runtime_tells_the_app_when_the_held_modifiers_change() {
    fn view(_: &Vec<Mods>, _: &mut Cx<Vec<Mods>>) -> El { el() }
    let mut rt = App::new(AppSpec::new("mods", (100.0, 100.0), Vec::new, view))
        .on_modifiers(|heard: &mut Vec<Mods>, mods, _| heard.push(mods))
        .headless_runtime((100.0, 100.0), Env::new(false, 1.0));
    rt.frame(0.0);
    let alt = Mods { alt: true, ..Mods::default() };
    rt.physical_input(PhysicalInput::Modifiers(alt));
    rt.frame(0.0);
    rt.physical_input(PhysicalInput::Modifiers(Mods::default()));
    rt.frame(0.0);
    assert_eq!(rt.state, [alt, Mods::default()]);
}
