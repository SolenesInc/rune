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
    assert!(rt.state().pressed_at >= 100.0, "the press ran after the clock moved: {}", rt.state().pressed_at);
    assert_eq!(rt.state().picked_at, Some(rt.state().pressed_at), "the callback reads the clock its handler read");
    assert_eq!(rt.wake_deadline(), Some(rt.state().due), "a deadline the handler stamped wakes when it is due");
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
    assert!(rt.state().asked);
    let shown = rt.state().found[0].expect("shown is on screen");
    assert_eq!(shown.0, [0.0, 40.0, 30.0, 20.0]);
    assert_eq!(shown.1, shown.0);
    let clipped = rt.state().found[1].expect("clipped is laid out");
    assert_eq!(clipped.0[3], 30.0);
    assert_eq!(clipped.1[3], 10.0, "its parent's clip leaves 10 points of it visible");
    assert_eq!(rt.state().found[2], None);
}

#[test]
fn find_links_answers_where_the_last_frame_showed_each_link_of_a_prefix_line_by_line() {
    #[derive(Default)]
    struct S { found: Vec<crate::ShownLink>, covered: bool, lidded: bool }
    const WORDS: &str = "see ag-first1 ag-second and ag-third1";
    fn at(part: &str) -> std::ops::Range<usize> {
        let i = WORDS.find(part).unwrap();
        i..i + part.len()
    }
    fn linked(id: &'static str) -> El {
        crate::rich(WORDS, Vec::new()).font(crate::TextStyle::new(14.0, 400.0).lh(20.0)).w(100.0)
            .link(crate::Link::new(at("ag-first1 ag-second"), id))
            .link(crate::Link::new(at("ag-third1"), "https://other"))
            .on_link(|_: &mut S, _, _| {})
    }
    fn view(s: &S, _: &mut Cx<S>) -> El {
        let read = |key: &str| el().key(key.to_owned()).size(40.0, 40.0).on_press_tx(|_: &mut S, tx| {
            tx.find_links("id:", |s: &mut S, found, _| s.found = found);
        });
        col().w_full().h_full()
            .child(read("ask"))
            .child(linked("id:shown"))
            .child(col().w(100.0).h(30.0).scroll(crate::ScrollSpec { stick_bottom: false })
                .child(linked("id:clipped"))
                .child(el().size(10.0, 200.0))
                .child(linked("id:scrolled-away")))
            .children(s.lidded.then(|| el().key("lid").absolute(0.0, 40.0).size(300.0, 40.0).on_press(|_: &mut S| {})))
            .children(s.covered.then(|| el().key("modal").absolute(0.0, 0.0).size(300.0, 300.0).modal(true, |_: &mut S| {}).child(read("over"))))
    }
    let mut rt = Runtime::new(S::default(), view, (300.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let ask = |rt: &mut Runtime<S>| {
        rt.pointer_move(20.0, 20.0);
        rt.pointer_down(20.0, 20.0);
        rt.pointer_up(20.0, 20.0);
        rt.frame(1.0);
        std::mem::take(&mut rt.state_mut().found)
    };
    let found = ask(&mut rt);
    let ids: Vec<&str> = found.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["id:shown", "id:clipped"], "a link of another prefix and one scrolled away are left out");
    let [first, second] = found[0].lines[..] else { panic!("the link wraps onto a second line: {found:?}") };
    assert!(first[0] > second[0] && second[1] == first[1] + 20.0, "each line has its own box: {found:?}");
    assert_eq!(found[0].clip, [0.0, 0.0, 300.0, 300.0], "only the window cuts it");
    assert_eq!(found[1].lines.len(), 2);
    assert_eq!([found[1].clip[2], found[1].clip[3]], [100.0, 30.0], "its scroll view's box");
    assert!(found[1].lines[1][3] < first[3], "the clip cuts its second line: {found:?}");

    let [x, y, w, h] = first;
    rt.pointer_move(x + w / 2.0, y + h / 2.0);
    rt.frame(2.0);
    let hovered = |rt: &Runtime<S>| rt.links("id:").iter().filter(|l| l.hovered).map(|l| l.id.clone()).collect::<Vec<_>>();
    assert_eq!(hovered(&rt), ["id:shown"]);

    rt.state_mut().lidded = true;
    rt.invalidate();
    rt.frame(3.0);
    let found = ask(&mut rt);
    assert_eq!(found.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(), ["id:clipped"], "what takes the press over a link hides it");

    rt.state_mut().covered = true;
    rt.invalidate();
    rt.frame(3.0);
    assert_eq!(ask(&mut rt), [], "a modal over the links keeps input from them");
}
