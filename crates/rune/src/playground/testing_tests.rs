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
    assert_eq!(*rt.state(), [alt, Mods::default()]);
}

#[test]
fn a_headless_runtime_tells_the_app_of_each_input_the_person_gives_and_of_none_it_makes_itself() {
    fn view(_: &usize, _: &mut Cx<usize>) -> El { el() }
    let mut rt = App::new(AppSpec::new("input", (100.0, 100.0), || 0, view))
        .on_input(|heard: &mut usize, _| *heard += 1)
        .headless_runtime((100.0, 100.0), Env::new(false, 1.0));
    rt.frame(0.0);
    rt.key(crate::KeyEvent { key: crate::Key::Tab, mods: Mods::default() });
    rt.pointer_move(10.0, 10.0);
    rt.pointer_down(10.0, 10.0);
    rt.pointer_up(10.0, 10.0);
    rt.wheel(10.0, 10.0, 4.0);
    rt.physical_input(PhysicalInput::Modifiers(Mods { shift: true, ..Mods::default() }));
    rt.frame(0.0);
    assert_eq!(*rt.state(), 6);
    rt.window_focus(true);
    rt.window_focus(false);
    rt.frame(0.0);
    assert_eq!(*rt.state(), 6, "losing focus is not input");
}

#[test]
fn input_the_app_hears_rebuilds_no_view() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static VIEWS: AtomicUsize = AtomicUsize::new(0);
    fn view(_: &usize, _: &mut Cx<usize>) -> El {
        VIEWS.fetch_add(1, Ordering::Relaxed);
        el()
    }
    let mut rt = App::new(AppSpec::new("input", (100.0, 100.0), || 0, view))
        .on_input(|heard: &mut usize, _| *heard += 1)
        .headless_runtime((100.0, 100.0), Env::new(false, 1.0));
    rt.frame(0.0);
    let views = VIEWS.load(Ordering::Relaxed);
    rt.pointer_move(10.0, 10.0);
    rt.frame(0.0);
    assert_eq!(*rt.state(), 1);
    assert_eq!(VIEWS.load(Ordering::Relaxed), views);
}

#[test]
fn a_headless_runtime_tells_the_app_when_the_window_goes_off_screen_and_back() {
    fn view(_: &Vec<bool>, _: &mut Cx<Vec<bool>>) -> El { el() }
    let mut rt = App::new(AppSpec::new("sight", (100.0, 100.0), Vec::new, view))
        .on_visibility(|heard: &mut Vec<bool>, visible, _| heard.push(visible))
        .headless_runtime((100.0, 100.0), Env::new(false, 1.0));
    rt.frame(0.0);
    rt.set_live_visible(false);
    rt.set_live_visible(true);
    assert_eq!(*rt.state(), [false, true]);
}
