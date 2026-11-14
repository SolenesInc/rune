use super::*;

#[test]
fn a_pick_read_callback_runs_on_the_frame_clock_its_handler_saw() {
    #[derive(Default)]
    struct S { pressed_at: f64, picked_at: Option<f64>, due: f64 }
    fn view(_: &S, _: &mut Cx<S>) -> El {
        el().w_full().h_full().child(el().key("button").size(40.0, 40.0).on_press_tx(|s: &mut S, tx| {
            s.pressed_at = tx.clock();
            s.due = tx.clock() + 5.0;
            tx.element_at(20.0, 20.0, |s: &mut S, _, tx| {
                s.picked_at = Some(tx.clock());
                tx.wake_at(s.due);
            });
        }))
    }
    let mut rt = Runtime::new(S::default(), view, (100.0, 100.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.frame(100.0);
    rt.pointer_move(20.0, 20.0);
    rt.pointer_down(20.0, 20.0);
    rt.pointer_up(20.0, 20.0);
    rt.frame(1.0);
    assert!(rt.state.pressed_at >= 100.0, "the press ran after the clock moved: {}", rt.state.pressed_at);
    assert_eq!(rt.state.picked_at, Some(rt.state.pressed_at), "the callback reads the clock its handler read");
    assert_eq!(rt.wake_deadline(), Some(rt.state.due), "a deadline the handler stamped wakes when it is due");
}

#[test]
fn find_elements_answers_where_the_last_frame_showed_each_key_after_the_handler() {
    #[derive(Default)]
    struct S { asked: bool, found: Vec<Option<([f32; 4], [f32; 4])>> }
    fn view(_: &S, _: &mut Cx<S>) -> El {
        col().w_full().h_full()
            .child(el().key("ask").size(40.0, 40.0).on_press_tx(|s: &mut S, tx| {
                s.asked = true;
                tx.find_elements(vec!["shown".into(), "clipped".into(), "absent".into()], |s: &mut S, found, _| {
                    s.found = found.into_iter().map(|e| e.map(|e| (e.rect, e.visible))).collect();
                });
                assert!(s.found.is_empty(), "the answer comes after the handler");
            }))
            .child(el().key("shown").size(30.0, 20.0))
            .child(col().w(50.0).h(10.0).scroll(crate::ScrollSpec { stick_bottom: false }).child(el().key("clipped").size(50.0, 30.0)))
    }
    let mut rt = Runtime::new(S::default(), view, (100.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.pointer_move(20.0, 20.0);
    rt.pointer_down(20.0, 20.0);
    rt.pointer_up(20.0, 20.0);
    rt.frame(1.0);
    assert!(rt.state.asked);
    let shown = rt.state.found[0].expect("shown is on screen");
    assert_eq!(shown.0, [0.0, 40.0, 30.0, 20.0]);
    assert_eq!(shown.1, shown.0);
    let clipped = rt.state.found[1].expect("clipped is laid out");
    assert_eq!(clipped.0[3], 30.0);
    assert_eq!(clipped.1[3], 10.0, "its parent's clip leaves 10 points of it visible");
    assert_eq!(rt.state.found[2], None);
}
