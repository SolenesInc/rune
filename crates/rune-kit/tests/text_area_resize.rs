//! A resizable TextArea drags taller and back by the grip in its bottom-right corner, never
//! below its least height, and the height it was dragged to stays in its TextEdit.
use rune::{Cx, El, Env, Ime, Runtime, TextEdit, col};
use rune_kit::{Kit, TextArea};

fn view(s: &TextEdit, _: &mut Cx<TextEdit>) -> El {
    let k = Kit::neutral(true);
    col().padding(20.0).child(TextArea::new(s).key("notes").resizable().on_edit(|s: &mut TextEdit| s).build(&k))
}

fn height(rt: &Runtime<TextEdit>) -> f32 {
    rt.presented("notes-resizable").expect("the resizable box").0[3]
}

fn drag_grip(rt: &mut Runtime<TextEdit>, by: f32) {
    let (x, y) = rt.find("notes-grip").expect("a grip");
    rt.pointer_down(x, y);
    rt.frame(1.0 / 60.0);
    rt.pointer_move(x, y + by);
    rt.frame(1.0 / 60.0);
    rt.pointer_up(x, y + by);
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
}

#[test]
fn a_resizable_text_area_drags_taller_and_back_down_to_its_least_height() {
    let mut rt = Runtime::new(TextEdit::default(), view, (600.0, 600.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.frame(1.0 / 60.0);
    let least = height(&rt);
    assert_eq!(least, 80.0, "an empty three-row area is a textarea's 80 points");

    drag_grip(&mut rt, 120.0);
    assert_eq!(height(&rt), least + 120.0);
    assert_eq!(rt.state().dragged_height(), Some(least + 120.0));

    drag_grip(&mut rt, -400.0);
    assert_eq!(height(&rt), least, "a drag stops at the least height");
}

#[test]
fn a_dragged_text_area_keeps_its_height_as_text_outgrows_it() {
    let mut rt = Runtime::new(TextEdit::default(), view, (600.0, 600.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.frame(1.0 / 60.0);
    drag_grip(&mut rt, 20.0);
    let dragged = height(&rt);
    let (x, y) = rt.find("notes").expect("the editor");
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(1.0 / 60.0);
    rt.ime(Ime::Commit("line\n".repeat(20)));
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(height(&rt), dragged, "longer text scrolls inside the dragged box");
}

fn tall(s: &TextEdit, _: &mut Cx<TextEdit>) -> El {
    let k = Kit::neutral(true);
    col().padding(20.0).child(TextArea::new(s).key("notes").min_height(120.0).resizable().on_edit(|s: &mut TextEdit| s).build(&k))
}

#[test]
fn a_min_height_raises_the_box_and_the_least_a_drag_reaches() {
    let mut rt = Runtime::new(TextEdit::default(), tall, (600.0, 600.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(height(&rt), 120.0);
    drag_grip(&mut rt, -60.0);
    assert_eq!(height(&rt), 120.0);
}

fn styled(s: &TextEdit, _: &mut Cx<TextEdit>) -> El {
    let k = Kit::neutral(true);
    let area = TextArea::new(s)
        .key("notes")
        .font(rune::TextStyle { line_height: 18.0, ..rune::TextStyle::new(13.0, 400.0) })
        .padding(10.0, 8.0)
        .min_height(0.0)
        .resizable()
        .on_edit(|s: &mut TextEdit| s)
        .build(&k);
    col().padding(20.0).child(area)
}

#[test]
fn an_area_set_to_its_own_type_padding_and_min_height_is_its_rows_tall_and_drags_back_to_them() {
    let mut rt = Runtime::new(TextEdit::default(), styled, (600.0, 600.0), Env::new(true, 1.0));
    rt.frame(0.0);
    rt.frame(1.0 / 60.0);
    let three_rows = 3.0 * 18.0 + 2.0 * 8.0;
    assert_eq!(height(&rt), three_rows);
    let boxed = rt.presented("notes-resizable").expect("the resizable box").0;
    let editor = rt.presented("notes").expect("the editor").0;
    assert_eq!((editor[0] - boxed[0], editor[1] - boxed[1]), (10.0, 8.0));

    drag_grip(&mut rt, 40.0);
    assert_eq!(height(&rt), three_rows + 40.0);
    drag_grip(&mut rt, -400.0);
    assert_eq!(height(&rt), three_rows, "a drag stops at the rows the type and padding make");
}
