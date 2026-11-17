//! An accent badge's label takes the ink set for the accent fill, not the contrast surface's.
use rune::{Color, Cx, El, Env, Runtime};
use rune_kit::{Badge, BadgeVariant, Kit};

fn view(_: &(), _: &mut Cx<()>) -> El {
    let mut k = Kit::neutral(false);
    k.c.text_contrast = Color::WHITE;
    k.c.text_on_accent = Color::BLACK;
    rune::row().child(Badge::new("Customized").variant(BadgeVariant::Accent).build(&k).key("badge"))
}

#[test]
fn an_accent_badge_labels_in_the_ink_on_accent() {
    let mut rt = Runtime::new((), view, (300.0, 100.0), Env::new(false, 2.0));
    rt.frame(0.0);
    let label = rt.elements().into_iter().find(|e| e.path.ends_with("badge/label")).expect("the badge's label");
    assert_eq!(label.text_color, Some(Color::BLACK));
}
