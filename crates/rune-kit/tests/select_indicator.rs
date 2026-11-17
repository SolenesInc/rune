//! A kit that sets `glyph.select` draws that indicator in a Select, open or closed, as a macOS
//! pop-up button keeps its up-down chevron; without it the chevron points down and turns up open.
use rune::{Cx, El, Env, Runtime, col};
use rune_kit::{Kit, Select, SelectAppearance, glyphs, root};
use rune_ref::{Atlas, Frame, Primitive, Scene};

const W: u32 = 400;
const H: u32 = 300;

/// `(up_down, open)`: whether the kit sets the up-down indicator, and whether the select is open.
type S = (bool, bool);

fn view(&(up_down, open): &S, _: &mut Cx<S>) -> El {
    let mut k = Kit::neutral(true);
    if up_down {
        k.glyph.select = Some(&glyphs::CHEVRON_UP_DOWN);
    }
    let mut layer = Vec::new();
    let select = Select::new("hour", &["01", "02"])
        .current(Some("01"))
        .appearance(SelectAppearance::Field)
        .width(120.0)
        .open(open)
        .build(&k, &mut layer);
    root(col().padding(16.0).child(select), layer)
}

fn render(rt: &Runtime<S>) -> Frame {
    let scene = Scene {
        width: W,
        height: H,
        clear: [0.0, 0.0, 0.0, 1.0],
        primitives: rt
            .instances
            .iter()
            .filter(|p| p.rect[2] > 0.0 && p.rect[3] > 0.0)
            .enumerate()
            .map(|(i, p)| Primitive::from_instance(p, i as u32 + 1, format!("p/{i}")))
            .collect(),
        atlas: Atlas { width: rune::text::ATLAS_SIZE, height: rune::text::ATLAS_SIZE, data: rt.text.atlas.data.clone() },
    };
    scene.validate().unwrap();
    let mut frame = Frame::new(W, H, false);
    frame.render(&scene);
    frame
}

/// Whether the chevron's 16-point box has ink in its top and in its bottom third.
fn ink_above_and_below(up_down: bool, open: bool) -> (bool, bool) {
    let mut rt = Runtime::new((up_down, open), view, (W as f32, H as f32), Env::new(true, 1.0));
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
    let frame = render(&rt);
    let [x, y, w, _] = rt.presented("chevron").expect("a chevron").0;
    let inked = |rows: std::ops::Range<u32>| {
        rows.flat_map(|r| (x as u32..(x + w) as u32).map(move |c| (r, c)))
            .any(|(r, c)| frame.rgba[((y as u32 + r) * W + c) as usize * 4] > 60)
    };
    (inked(2..6), inked(10..14))
}

#[test]
fn a_kit_s_select_indicator_shows_open_and_closed() {
    assert_eq!(ink_above_and_below(true, false), (true, true), "closed: a chevron up over a chevron down");
    assert_eq!(ink_above_and_below(true, true), (true, true), "open: the same indicator");
}

#[test]
fn without_one_the_select_chevron_points_down_and_turns_up_open() {
    assert_eq!(ink_above_and_below(false, false), (false, false), "closed: one chevron in the middle");
    assert_eq!(ink_above_and_below(false, true), (false, false), "open: one chevron in the middle");
}
