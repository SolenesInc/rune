//! `TextArea::on_blur`: a press beside a text area runs its `on_blur` once, in the frame the
//! press lands.
use rune::{Cx, El, Env, Runtime, TextEdit, col, text};
use rune_kit::{Kit, TextArea};

struct Form { edit: TextEdit, blurs: usize }

fn view(s: &Form, _: &mut Cx<Form>) -> El {
    col().size(400.0, 300.0)
        .child(TextArea::new(&s.edit).key("area").width(None).on_edit(|s: &mut Form| &mut s.edit)
            .on_blur(|s: &mut Form, _| s.blurs += 1).build(&Kit::neutral(true)))
        .child(text("beside").key("beside"))
}

#[test]
fn a_press_beside_a_text_area_runs_its_on_blur_once() {
    let mut rt = Runtime::new(Form { edit: TextEdit::new("text"), blurs: 0 }, view, (400.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    for key in ["area", "beside"] {
        let (x, y) = rt.find(key).unwrap();
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
    }
    assert!(!rt.state().edit.focused);
    assert_eq!(rt.state().blurs, 1);
}
