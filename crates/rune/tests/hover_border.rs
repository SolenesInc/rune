//! `El::hover_border` turns a box's border to another colour while the pointer is over it, as
//! CSS `:hover { border-color }` does, with or without a hover fill.
use rune::text::TextSystem;
use rune::{Color, Cx, El, Env, Runtime, el};

const GREY: Color = Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 };
const BLUE: Color = Color { r: 0.0, g: 0.4, b: 1.0, a: 1.0 };

fn view(_: &(), _: &mut Cx<()>) -> El {
    el().size(60.0, 40.0).fill(Color::BLACK).child(
        el().size(40.0, 20.0).fill(Color::BLACK).border(1.0, GREY).hover_border(BLUE).on_press(|_: &mut ()| {}),
    )
}

fn runtime() -> Runtime<()> {
    let text = TextSystem::new(&[], ("Inter", "Inter"));
    let mut rt = Runtime::with_text((), view, (60.0, 40.0), Env::new(true, 1.0), text);
    rt.set_srgb_blend(false);
    rt.frame(0.0);
    rt
}

fn border(rt: &Runtime<()>) -> [f32; 4] {
    rt.instances
        .iter()
        .find(|i| i.rect[2] == 40.0 && i.rect[3] == 20.0 && i.border[3] > 0.0)
        .expect("the box's border")
        .border
}

#[test]
fn a_box_takes_its_hover_border_under_the_pointer_and_gives_it_back_when_it_leaves() {
    let mut rt = runtime();
    assert_eq!(border(&rt), GREY.premul(1.0), "at rest the border keeps its own colour");
    rt.pointer_move(10.0, 10.0);
    rt.frame(1.0 / 60.0);
    assert_eq!(border(&rt), BLUE.premul(1.0), "hovered, the border takes the hover colour");
    rt.pointer_move(55.0, 35.0);
    rt.frame(1.0);
    assert_eq!(border(&rt), GREY.premul(1.0), "once the pointer leaves, the border fades back");
}
