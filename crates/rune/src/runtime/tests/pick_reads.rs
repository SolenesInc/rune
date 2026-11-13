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
