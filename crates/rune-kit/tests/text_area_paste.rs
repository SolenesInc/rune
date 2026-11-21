//! `TextArea::on_paste`: a paste into a focused text area reaches the app first, which takes
//! an image and leaves plain text to the editor.
use rune::{ClipboardContent, Cx, El, Env, Runtime, TextEdit, col};
use rune_kit::{Kit, TextArea};

const ONE_RED_PIXEL: &[u8] = &[137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240, 31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130];

struct Form { edit: TextEdit, images: usize }

fn view(s: &Form, _: &mut Cx<Form>) -> El {
    col().size(400.0, 300.0).child(
        TextArea::new(&s.edit)
            .key("area")
            .width(None)
            .on_edit(|s: &mut Form| &mut s.edit)
            .on_paste(|s: &mut Form, content: &ClipboardContent, _| {
                let image = content.image().is_some();
                s.images += usize::from(image);
                image
            })
            .build(&Kit::neutral(true)),
    )
}

#[test]
fn a_text_area_hands_a_paste_to_the_app_before_its_editor() {
    let mut rt = Runtime::new(Form { edit: TextEdit::new(""), images: 0 }, view, (400.0, 300.0), Env::new(true, 1.0));
    rt.frame(0.0);
    let (x, y) = rt.find("area").unwrap();
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
    rt.paste_content(ClipboardContent::default().with_image(rune::ClipboardImage::from_png(ONE_RED_PIXEL).unwrap()));
    rt.frame(0.0);
    rt.paste("words");
    rt.frame(0.0);
    assert_eq!(rt.state().images, 1);
    assert_eq!(rt.state().edit.text, "words");
}
