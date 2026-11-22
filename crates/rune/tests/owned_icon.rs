use rune::{Color, Cx, El, Env, OwnedIcon, Runtime, col, icon};

struct State {
    logo: OwnedIcon,
    tick: usize,
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    col()
        .w_full()
        .h_full()
        .child(icon(&s.logo, 24.0).key("logo").color(Color::WHITE))
}

#[test]
fn an_owned_icon_paints_and_updates_when_replaced() {
    let first = OwnedIcon::new(
        "first",
        24.0,
        r#"<circle cx="12" cy="12" r="10" fill="currentColor"/>"#,
    );
    let second = OwnedIcon::new(
        "second",
        24.0,
        r#"<rect x="2" y="2" width="20" height="20" fill="currentColor"/>"#,
    );
    let mut rt = Runtime::new(
        State {
            logo: first,
            tick: 0,
        },
        view,
        (100.0, 100.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    assert!(
        !rt.instances.is_empty(),
        "an owned monochrome icon must paint into the frame"
    );
    let uv1 = rt.instances[0].uv;

    // Rebuilding with the same OwnedIcon keeps the same atlas slot and settles cleanly.
    rt.update(|s, _| s.tick += 1);
    rt.frame(1.0 / 60.0);
    assert_eq!(rt.instances[0].uv, uv1);

    // Replacing with another OwnedIcon paints its own atlas slot.
    rt.update(|s, _| s.logo = second.clone());
    rt.frame(1.0 / 60.0);
    assert!(!rt.instances.is_empty());
    assert_ne!(rt.instances[0].uv, uv1);
}

#[test]
fn a_colored_owned_icon_paints_as_an_image_when_svg_is_enabled() {
    let badge = OwnedIcon::new(
        "badge",
        24.0,
        r##"<rect x="2" y="2" width="20" height="20" fill="#ff3366"/>"##,
    );
    let mut rt = Runtime::new(
        State {
            logo: badge,
            tick: 0,
        },
        view,
        (100.0, 100.0),
        Env::new(true, 1.0),
    );
    rt.frame(0.0);
    assert!(
        !rt.instances.is_empty(),
        "a colored owned icon must paint into the frame"
    );
}
