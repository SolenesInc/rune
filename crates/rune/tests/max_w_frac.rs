//! `max_w_frac` holds an item a column does not stretch inside the column: the item stops at
//! the column's width and its shrinkable children give up the rest, where without it the item
//! takes its content's width and runs past the column.
use rune::runtime::ElementInfo;
use rune::{Cx, El, Env, Runtime, col, el, row};

struct S;

fn wide(key: &str) -> El {
    row().key(key).child(el().key(format!("{key}.child")).w(300.0).h(10.0).shrink().min_w(0.0))
}

fn view(_: &S, _: &mut Cx<S>) -> El {
    col().w_full().h_full().child(
        col().key("column").w(200.0).items_start()
            .child(wide("held").max_w_frac(1.0))
            .child(wide("free"))
            .child(row().key("narrow").max_w_frac(1.0).child(el().w(80.0).h(10.0))),
    )
}

fn rect(rt: &Runtime<S>, key: &str) -> [f32; 4] {
    let e: ElementInfo = rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("{key}"));
    e.rect
}

#[test]
fn an_item_held_to_its_columns_width_shrinks_its_children_to_fit() {
    let mut rt = Runtime::new(S, view, (400.0, 400.0), Env::new(true, 2.0));
    rt.frame(0.0);
    assert_eq!(rect(&rt, "column")[2], 200.0);
    assert_eq!(rect(&rt, "held")[2], 200.0, "held to the column's 200");
    assert_eq!(rect(&rt, "held.child")[2], 200.0, "its child shrinks to fit");
    assert_eq!(rect(&rt, "free")[2], 300.0, "without it the item takes its content's 300");
    assert_eq!(rect(&rt, "narrow")[2], 80.0, "a narrower item keeps its own width");
}
