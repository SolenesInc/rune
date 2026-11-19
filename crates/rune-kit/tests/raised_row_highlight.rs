//! A Select's listbox and a PopupList sit on the raised surface, so the row that keys or the
//! pointer reach draws in `surface_raised_hover`, as a List and a standard Menu do. A kit whose
//! `hover` is the raised surface's own colour, as attn's is, still shows that row.
use rune::{Cx, El, Env, Key, KeyEvent, Mods, Runtime, col};
use rune_kit::{Kit, PopupItem, PopupList, Select, SelectAppearance, root};
use rune_ref::{Atlas, Frame, Primitive, Scene};

const W: u32 = 400;
const H: u32 = 300;

fn kit() -> Kit {
    let mut k = Kit::neutral(true);
    let surface = rune::color::oklch(0.25, 0.0, 0.0);
    k.c.layer_01 = surface;
    k.c.surface_raised = surface;
    k.c.hover = surface;
    k.c.surface_raised_hover = rune::color::oklch(0.40, 0.0, 0.0);
    k
}

struct Picker {
    open: bool,
    value: String,
}

fn select(s: &Picker, _: &mut Cx<Picker>) -> El {
    let k = kit();
    let mut layer = Vec::new();
    let select = Select::new("hour", &["01", "02", "03", "04"])
        .current(Some(&s.value))
        .appearance(SelectAppearance::Field)
        .width(120.0)
        .open(s.open)
        .on_toggle(|s: &mut Picker| s.open = !s.open)
        .on_close(|s: &mut Picker| s.open = false)
        .on_choose(|s: &mut Picker, v, _| s.value = v.into())
        .build(&k, &mut layer);
    root(col().w_full().h_full().child(select), layer)
}

fn popup(_: &(), _: &mut Cx<()>) -> El {
    let k = kit();
    let mut layer = Vec::new();
    PopupList::<()>::new("zones", "field")
        .open(true)
        .items((0..4).map(|i| PopupItem::new(format!("z{i}"), format!("Zone {i}"))))
        .active(Some(1))
        .build(&k, &mut layer);
    root(col().padding(16.0).child(rune::el().key("field").w(200.0).h(28.0)), layer)
}

fn settle<S>(rt: &mut Runtime<S>) {
    for _ in 0..30 {
        rt.frame(1.0 / 60.0);
    }
}

fn row_pixel<S>(rt: &Runtime<S>, frame: &Frame, key: &str) -> [u8; 3] {
    let e = rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap_or_else(|| panic!("no {key}"));
    let [x, y, _, h] = e.rect;
    let i = (((y + h / 2.0) as u32) * W + (x + 3.0) as u32) as usize * 4;
    [frame.rgba[i], frame.rgba[i + 1], frame.rgba[i + 2]]
}

/// An empty scroll bar track draws nothing, and the reference renderer refuses it.
fn render<S>(rt: &Runtime<S>) -> Frame {
    let scene = Scene {
        width: W,
        height: H,
        clear: [0.0, 0.0, 0.0, 1.0],
        primitives: rt
            .instances
            .iter()
            .filter(|p| p.rect[2] > 0.0 && p.rect[3] > 0.0)
            .enumerate()
            .map(|(i, p)| Primitive::from_instance(p, i as u32 + 1, format!("row/{i}")))
            .collect(),
        atlas: Atlas { width: rune::text::ATLAS_SIZE, height: rune::text::ATLAS_SIZE, data: rt.text.atlas.data.clone() },
    };
    scene.validate().unwrap();
    let mut frame = Frame::new(W, H, false);
    frame.render(&scene);
    frame
}

fn differ(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).any(|(x, y)| x.abs_diff(y) > 8)
}

#[test]
fn the_select_row_down_reaches_shows_on_a_surface_its_hover_matches() {
    let mut rt = Runtime::new(Picker { open: false, value: "01".into() }, select, (W as f32, H as f32), Env::new(true, 1.0));
    rt.frame(0.0);
    for key in [Key::Tab, Key::Down, Key::Down] {
        rt.key(KeyEvent { key, mods: Mods::default() });
        rt.frame(1.0 / 60.0);
    }
    settle(&mut rt);
    assert!(rt.elements().iter().any(|e| e.focused && e.key.as_deref() == Some("option1")), "Down reached the second hour");
    let frame = render(&rt);
    let (reached, resting) = (row_pixel(&rt, &frame, "option1"), row_pixel(&rt, &frame, "option2"));
    assert!(differ(reached, resting), "the reached row {reached:?} shows against a resting row {resting:?}");
}

#[test]
fn the_active_popup_list_row_shows_on_a_surface_its_hover_matches() {
    let mut rt = Runtime::new((), popup, (W as f32, H as f32), Env::new(true, 1.0));
    rt.frame(0.0);
    settle(&mut rt);
    let frame = render(&rt);
    let (active, resting) = (row_pixel(&rt, &frame, "zones-option-z1"), row_pixel(&rt, &frame, "zones-option-z2"));
    assert!(differ(active, resting), "the active row {active:?} shows against a resting row {resting:?}");
}
