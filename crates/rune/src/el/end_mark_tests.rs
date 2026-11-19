use crate::{Color, Cx, El, Env, Runtime, Span, col, rich};

const MARK: &str = "\u{a0}|";
const RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };

fn view(s: &String, _: &mut Cx<String>) -> El {
    col().key("root").streaming(true).child(rich(s, Vec::new()).key("body").color(Color::WHITE).reveal(0.2).end_mark(MARK, [Span::new(2, 3).color(RED)]))
}

/// The alpha of the mark's glyph, the one thing drawn in red.
fn mark_alpha(rt: &Runtime<String>) -> f32 {
    let marks: Vec<_> = rt.instances.iter().filter(|i| i.fill[0] > 0.0 && i.fill[1] == 0.0 && i.fill[2] == 0.0).collect();
    assert_eq!(marks.len(), 1, "one mark is drawn");
    marks[0].fill[3]
}

/// Words that arrive fade in; the mark after them is there at once, each time it moves.
#[test]
fn an_end_mark_draws_at_once_while_the_words_before_it_fade_in() {
    let mut rt = Runtime::new("one".to_string(), view, (300.0, 80.0), Env::new(false, 1.0));
    rt.frame(0.0);
    assert_eq!(mark_alpha(&rt), 1.0);
    for (k, word) in [" two", " three", " four"].into_iter().enumerate() {
        rt.state_mut().push_str(word);
        rt.invalidate();
        rt.frame(0.01);
        let body = rt.elements().into_iter().find(|e| e.key.as_deref() == Some("body")).unwrap();
        assert!(body.revealing, "word {k} is fading in");
        assert_eq!(mark_alpha(&rt), 1.0, "the mark after word {k} is not");
        assert_eq!(body.text.as_deref(), Some(format!("{}{MARK}", rt.state()).as_str()));
    }
}
