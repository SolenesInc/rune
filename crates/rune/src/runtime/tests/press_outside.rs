use super::*;

#[test]
fn a_press_away_from_an_element_tells_it_and_still_reaches_its_target() {
    #[derive(Default)]
    struct S {
        outside: u32,
        pressed: u32,
    }
    fn view(_: &S, _: &mut Cx<S>) -> El {
        col()
            .w_full()
            .h_full()
            .child(
                el().key("card")
                    .size(40.0, 40.0)
                    .on_press_outside(|s: &mut S, _| s.outside += 1),
            )
            .child(
                el().key("button")
                    .size(40.0, 40.0)
                    .on_press(|s: &mut S| s.pressed += 1),
            )
    }
    let mut rt = Runtime::new(S::default(), view, (100.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let press = |rt: &mut Runtime<S>, x: f32, y: f32| {
        rt.pointer_move(x, y);
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(1.0);
    };
    press(&mut rt, 20.0, 20.0);
    assert_eq!(
        (rt.state().outside, rt.state().pressed),
        (0, 0),
        "a press on the card is inside it"
    );
    press(&mut rt, 20.0, 60.0);
    assert_eq!(
        (rt.state().outside, rt.state().pressed),
        (1, 1),
        "the button still hears its press"
    );
    press(&mut rt, 80.0, 150.0);
    assert_eq!(
        rt.state().outside,
        2,
        "a press on empty space is outside too"
    );
}

#[test]
fn a_blur_after_a_focus_ask_cancels_it_when_both_come_before_a_frame() {
    #[derive(Default)]
    struct S {
        open: bool,
    }
    fn view(s: &S, _: &mut Cx<S>) -> El {
        let mut root = col()
            .w_full()
            .h_full()
            .child(el().key("trigger").size(40.0, 40.0).focusable(true))
            .child(
                el().key("tile")
                    .size(40.0, 40.0)
                    .on_press_tx(|_: &mut S, tx| tx.blur()),
            );
        if s.open {
            root = root.child(
                el().key("popover")
                    .size(40.0, 40.0)
                    .focusable(true)
                    .on_press_outside(|s: &mut S, tx| {
                        s.open = false;
                        tx.focus("trigger", true);
                    }),
            );
        }
        root
    }
    let focused = |rt: &Runtime<S>| {
        rt.elements()
            .into_iter()
            .find(|e| e.focused)
            .and_then(|e| e.key)
    };
    let mut rt = Runtime::new(S { open: true }, view, (100.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.update(|_, tx| tx.focus("popover", false));
    rt.frame(1.0);
    assert_eq!(focused(&rt).as_deref(), Some("popover"));

    rt.pointer_move(20.0, 60.0);
    rt.pointer_down(20.0, 60.0);
    rt.pointer_up(20.0, 60.0);
    rt.frame(2.0);
    assert!(!rt.state().open);
    assert_eq!(
        focused(&rt),
        None,
        "the tile's blur on release comes after the popover's ask on the down"
    );

    rt.update(|_, tx| tx.focus("trigger", false));
    rt.update(|_, tx| tx.blur());
    rt.frame(3.0);
    assert_eq!(
        focused(&rt),
        None,
        "focus() then blur() leaves nothing focused, as on the web"
    );
}
