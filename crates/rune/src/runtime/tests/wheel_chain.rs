use super::*;
use crate::el::{ScrollSpec, auto_grow, text_input};
use crate::{TextEdit, TextStyle};

struct Page {
    note: TextEdit,
    log: TextEdit,
}

fn page_view(s: &Page, _: &mut Cx<Page>) -> El {
    let style = TextStyle::new(14.0, 400.0);
    col()
        .key("page")
        .size(300.0, 200.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(el().key("header").w_full().h(40.0))
        .child(auto_grow(text_input(&s.note, style).key("note").on_edit(|s: &mut Page| &mut s.note), 1, 4).key("note-scroll"))
        .child(auto_grow(text_input(&s.log, style).key("log").on_edit(|s: &mut Page| &mut s.log), 1, 2).key("log-scroll"))
        .children((0..20).map(|i| el().key(format!("row{i}")).w_full().h(40.0)))
}

fn top(rt: &Runtime<Page>, key: &str) -> f32 {
    rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("no {key}")).rect[1]
}

fn centre(rt: &Runtime<Page>, key: &str) -> (f32, f32) {
    let r = rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("no {key}")).rect;
    (r[0] + r[2] * 0.5, r[1] + r[3] * 0.5)
}

#[test]
fn the_wheel_over_a_field_with_nothing_to_scroll_that_way_scrolls_the_page_around_it() {
    let log = (0..6).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
    let mut rt = Runtime::new(Page { note: TextEdit::new("short"), log: TextEdit::new(log) }, page_view, (300.0, 200.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let row = top(&rt, "header");

    let (x, y) = centre(&rt, "note-scroll");
    rt.wheel(x, y, -10.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(top(&rt, "header"), row - 10.0, "a field that fits passes the wheel to the page");

    let (x, y) = centre(&rt, "log-scroll");
    let line = top(&rt, "log");
    rt.wheel(x, y, -10.0);
    rt.frame(1.0 / 60.0);
    assert_eq!((top(&rt, "header"), top(&rt, "log")), (row - 10.0, line - 10.0), "a field with more below scrolls itself");

    rt.wheel(x, y, 10.0);
    rt.frame(1.0 / 60.0);
    assert_eq!((top(&rt, "header"), top(&rt, "log")), (row - 10.0, line), "the field takes the wheel back up to its top");
    rt.wheel(x, y, 10.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(top(&rt, "header"), row, "a field at its top passes a wheel up to the page");
}
