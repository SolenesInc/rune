use super::*;

/// A column `tall` points high over `a`, 150 pt that shrinks first down to 50, and `b`, grown
/// from 100 pt.
fn column(tall: &f32, _: &mut Cx<f32>) -> El {
    col().key("root").w_full().child(
        col()
            .key("column")
            .w_full()
            .h(*tall)
            .child(el().key("a").w_full().h(150.0).shrink_first().min_h(50.0))
            .child(el().key("b").w_full().grow_from(1.0, 100.0).min_h(0.0)),
    )
}

fn height(rt: &Runtime<f32>, key: &str) -> f32 {
    rt.elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap()
        .rect[3]
}

/// A shortfall goes to an element that shrinks first until it reaches its minimum; only then
/// do its siblings give way, and room to spare still grows them.
#[test]
fn an_element_that_shrinks_first_takes_a_shortfall_down_to_its_minimum_before_its_siblings() {
    for (tall, a, b) in [
        (300.0, 150.0, 150.0),
        (200.0, 100.0, 100.0),
        (150.0, 50.0, 100.0),
        (120.0, 50.0, 70.0),
    ] {
        let mut rt = Runtime::new(tall, column, (200.0, 400.0), Env::new(true, 2.0));
        rt.frame(0.0);
        let got = [height(&rt, "a"), height(&rt, "b")];
        assert!(
            (got[0] - a).abs() < 0.01 && (got[1] - b).abs() < 0.01,
            "in a column {tall} tall: {got:?}, not {:?}",
            [a, b]
        );
    }
}
