//! A frame at rest whose window paints as it did keeps the last frame's paint: an app's own
//! drawing, such as a terminal's cells, then costs no walk of the element tree, however many
//! elements are mounted. A checked runtime paints each kept frame anyway and fails when it
//! differs: Rune's own tests always, an app's tests through `Runtime::check_kept_paint`, and
//! debug builds with `RUNE_KEPT_PAINT_CHECK=1`.

use super::*;

/// The last paint, what it was built from (`paint_inputs`), and whether it repeated the paint
/// before it from the same inputs: state that settles a frame after a change has settled.
pub(crate) struct LastPaint {
    pub(super) inputs: Option<(u64, u64)>,
    instances: Vec<Instance>,
    pub(super) repeated: bool,
    pub(super) checked: bool,
}

impl Default for LastPaint {
    fn default() -> Self {
        let asked = cfg!(debug_assertions) && std::env::var_os("RUNE_KEPT_PAINT_CHECK").is_some_and(|v| v != "0");
        LastPaint { inputs: None, instances: Vec::new(), repeated: false, checked: cfg!(test) || asked }
    }
}

impl LastPaint {
    pub(super) fn follow(&mut self, inputs: (u64, u64), instances: &[Instance]) {
        self.repeated = self.inputs == Some(inputs) && same(&self.instances, instances);
        self.inputs = Some(inputs);
        self.instances.clear();
        self.instances.extend_from_slice(instances);
    }
}

fn same(a: &[Instance], b: &[Instance]) -> bool {
    bytemuck::cast_slice::<_, u8>(a) == bytemuck::cast_slice::<_, u8>(b)
}

pub(super) fn check(kept: &[Instance], painted: &[Instance], animating: bool) {
    if same(kept, painted) && !animating { return; }
    let first = kept.iter().zip(painted).position(|(a, b)| bytemuck::bytes_of(a) != bytemuck::bytes_of(b));
    panic!(
        "rune: a frame kept its paint, but painting it moves ({animating}) or draws {} instances where it kept {}, first differing at {first:?}",
        painted.len(),
        kept.len(),
    );
}
