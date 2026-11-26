use super::*;

#[derive(Default)]
struct Seen {
    log: Vec<bool>,
}

fn page(_: &Seen, _: &mut Cx<Seen>) -> El {
    crate::col().w_full()
        .child(crate::col().key("top").w_full().h(500.0))
        .child(crate::col().key("demo").w_full().h(100.0).on_visible(|s: &mut Seen, on, _| s.log.push(on)))
        .child(crate::col().key("bottom").w_full().h(600.0))
}

#[test]
fn an_element_hears_when_the_page_scrolls_it_on_screen_and_off() {
    let mut rt = Runtime::new(Seen::default(), page, (400.0, 200.0), Env::default());
    rt.page_mode();
    rt.frame(0.0);
    rt.frame(0.0);
    assert!(rt.state().log.is_empty(), "off screen at first, and nothing to say");

    rt.set_page_scroll(330.0);
    rt.frame(0.0);
    rt.frame(0.0);
    assert!(rt.state().log.is_empty(), "30 of its 100 points show: not yet");

    rt.set_page_scroll(360.0);
    rt.frame(0.0);
    rt.frame(0.0);
    assert_eq!(rt.state().log, [true], "60 points show");

    rt.set_page_scroll(450.0);
    rt.frame(0.0);
    rt.frame(0.0);
    assert_eq!(rt.state().log, [true], "still on screen: no second report");

    rt.set_page_scroll(800.0);
    rt.frame(0.0);
    rt.frame(0.0);
    assert_eq!(rt.state().log, [true, false]);
}

#[test]
fn half_of_it_or_half_the_window_counts() {
    let view = (400.0, 200.0);
    assert!(on_screen([0.0, 150.0, 400.0, 100.0], view));
    assert!(!on_screen([0.0, 151.0, 400.0, 100.0], view));
    assert!(on_screen([0.0, -400.0, 400.0, 1000.0], view), "taller than the window, covering it");
    assert!(!on_screen([0.0, 200.0, 400.0, 100.0], view));
}

#[derive(Default)]
struct Paged {
    rows: usize,
    more: bool,
    log: Vec<bool>,
}

fn paged(s: &Paged, _: &mut Cx<Paged>) -> El {
    let mut list = crate::col().key("list").w_full();
    for i in 0..s.rows {
        list = list.child(crate::col().key(format!("r{i}")).w_full().h(100.0));
    }
    let tail = if s.more {
        crate::col().key("more").w_full().h(20.0).on_visible(|s: &mut Paged, on, _| s.log.push(on))
    } else {
        crate::col().key("loading").w_full().h(20.0)
    };
    crate::col().key("page").w_full().growth(crate::el::GrowthSpec::default())
        .child(crate::col().key("list.block").w_full().child(list))
        .child(crate::col().key("tail.block").w_full().child(tail))
}

#[test]
fn an_element_that_rows_growing_above_it_carry_off_screen_never_hears_it_showed() {
    let mut rt = Runtime::new(Paged { rows: 1, ..Paged::default() }, paged, (400.0, 200.0), Env::new(true, 2.0));
    rt.frame(0.0);
    rt.frame(0.0);
    rt.update(|s, _| { s.rows = 10; s.more = true; });
    rt.frame(1.0 / 60.0);
    let more = rt.elements().into_iter().find(|e| e.key.as_deref() == Some("more")).unwrap();
    assert!(more.rect[1] < 200.0, "the rows above it are still growing: {:?}", more.rect);
    for _ in 0..240 {
        rt.frame(1.0 / 60.0);
    }
    assert!(!rt.animating);
    assert!(rt.state().log.is_empty(), "it rests at 1000, below the window: {:?}", rt.state().log);
}
