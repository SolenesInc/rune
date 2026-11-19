//! `El::scroll_room`: a viewport keeps a strip clear across from each axis its content overflows
//! on, as a browser's classic scroll bar takes its width from an `overflow: auto` box, and gives
//! it back while the content fits.
use rune::{Cx, El, Env, Runtime, ScrollSpec, col, el, row};

const ROOM: f32 = 8.0;

#[derive(Clone, Copy)]
enum Shape {
    /// `rows` 40 pt rows filling a 200 by 200 vertical viewport's width.
    Down { rows: usize },
    /// A tile `wide` points wide in a 200 pt sideways viewport whose height is its content's.
    Sideways { wide: f32 },
    /// A 6 by 6 grid of 100 pt cells in a 200 by 200 viewport scrolling both ways.
    BothWays,
}

fn view(s: &Shape, _: &mut Cx<Shape>) -> El {
    let viewport = match *s {
        Shape::Down { rows } => (0..rows).fold(
            col().h(200.0).scroll(ScrollSpec {
                stick_bottom: false,
            }),
            |c, i| c.child(el().key(format!("row{i}")).w_full().h(40.0)),
        ),
        Shape::Sideways { wide } => col().scroll_x().child(el().key("tile").w(wide).h(30.0)),
        Shape::BothWays => {
            let grid = (0..6).fold(col(), |c, r| {
                c.child((0..6).fold(row(), |row, i| {
                    row.child(el().key(format!("cell{r}-{i}")).w(100.0).h(100.0))
                }))
            });
            col().h(200.0).scroll_xy().child(grid)
        }
    };
    col()
        .w_full()
        .h_full()
        .child(viewport.key("view").w(200.0).scroll_room(ROOM))
}

fn app(s: Shape) -> Runtime<Shape> {
    let mut rt = Runtime::new(s, view, (300.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt
}

fn rect(rt: &Runtime<Shape>, key: &str) -> [f32; 4] {
    rt.elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some(key))
        .unwrap_or_else(|| panic!("{key}"))
        .rect
}

fn settle(rt: &mut Runtime<Shape>) {
    for _ in 0..600 {
        if !rt.frame(1.0 / 60.0) {
            return;
        }
    }
    panic!("never settled");
}

#[test]
fn a_vertical_viewport_gives_its_end_strip_only_while_its_rows_overflow() {
    let mut rt = app(Shape::Down { rows: 3 });
    assert_eq!(
        rect(&rt, "row0")[2],
        200.0,
        "rows that fit fill the viewport's width"
    );

    rt.update(|s, _| *s = Shape::Down { rows: 12 });
    settle(&mut rt);
    assert_eq!(
        rect(&rt, "row0")[2],
        200.0 - ROOM,
        "overflowing rows leave the strip at the end edge clear"
    );

    rt.wheel(100.0, 100.0, -10_000.0);
    settle(&mut rt);
    let [_, y, _, h] = rect(&rt, "row11");
    assert_eq!(
        y + h,
        200.0,
        "the last row still scrolls to the viewport's bottom"
    );

    rt.update(|s, _| *s = Shape::Down { rows: 2 });
    settle(&mut rt);
    assert_eq!(
        rect(&rt, "row0")[2],
        200.0,
        "once the rows fit again, the strip goes back to them"
    );
}

#[test]
fn a_sideways_viewport_grows_by_its_bottom_strip_only_while_its_content_overflows() {
    let mut rt = app(Shape::Sideways { wide: 150.0 });
    assert_eq!(
        rect(&rt, "view")[3],
        30.0,
        "content that fits leaves the viewport its own height"
    );

    rt.update(|s, _| *s = Shape::Sideways { wide: 500.0 });
    settle(&mut rt);
    assert_eq!(
        rect(&rt, "view")[3],
        30.0 + ROOM,
        "wide content puts the strip under itself"
    );
    assert_eq!(rect(&rt, "tile")[3], 30.0, "and keeps its own height");

    rt.wheel_xy(100.0, 10.0, -10_000.0, 0.0);
    settle(&mut rt);
    let [x, _, w, _] = rect(&rt, "tile");
    assert_eq!(
        x + w,
        200.0,
        "a sideways strip leaves the content's end at the viewport's end"
    );
}

#[test]
fn a_viewport_scrolling_both_ways_scrolls_its_content_clear_of_both_strips() {
    let mut rt = app(Shape::BothWays);
    rt.wheel_xy(100.0, 100.0, -10_000.0, 0.0);
    settle(&mut rt);
    rt.frame(0.2);
    rt.wheel_xy(100.0, 100.0, 0.0, -10_000.0);
    settle(&mut rt);
    let [x, y, w, h] = rect(&rt, "cell5-5");
    assert_eq!(
        (x + w, y + h),
        (200.0 - ROOM, 200.0 - ROOM),
        "the last cell stops short of the end strip and the bottom one"
    );
}
