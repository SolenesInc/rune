# SolenesInc/rune

This fork carries the patches attn's native app needs on top of upstream
[anomalyco/rune](https://github.com/anomalyco/rune). The `attn` branch is
upstream's `dev` with these patches; upstream is merged into it weekly and it is
never rebased. attn's `docs/internal/rune-fork.md` has the rules.

A patch belongs here only when an app cannot do the job through Rune's public
API or the raw window handle. Each line says what the patch changes, why the app
cannot do it, and its test. Remove a line, and its patch, once upstream makes
the patch unneeded.

| Patch | Why the app cannot do it | Test |
|---|---|---|
| A press that reaches a just-shown overlay before its first frame lays out the view first, so the press and its drag reach the element under the pointer (`overlay/input.rs`, `overlay/lifetime.rs`) | The overlay's event order is inside Rune; the app sees no press at all | `overlay::tests::a_press_before_the_first_frame_after_show_reaches_the_view` |
| An overlay that gives its GPU back when hidden removes its CAMetalLayer from the view, which otherwise keeps the last drawable (macOS, `overlay/surface.rs`, `platform/macos/surface.rs`) | The surface and its layer are Rune's | `platform::macos::surface::tests::a_released_surface_leaves_no_layer_in_its_view` (macOS) |
| The macOS app menu names the app from the bundle (`CFBundleDisplayName`, then `CFBundleName`, then the process), for Rune's app items and for winit's default menu, which Rune now adopts and labels (`native_menu/`) | Rune builds and relabels the app menu; an app's own titles are overwritten | `native_menu::standard::tests::the_app_is_named_as_appkit_names_it` |
| `Menu::label` names an open menu for accessibility, as the menu button that opens it does (`rune-kit/src/menu.rs`) | The menu's frame is built inside `Menu::build` and pushed into the layer; the app has no element to name | `menu::tests::an_open_menu_carries_its_label_as_its_accessible_name` |
| `RUNE_OVERLAY_FRAME_LOG=FILE` writes one JSON line per presented overlay frame: name, time since the first, and `draw_frame` time in ms (`overlay/draw.rs`) | Overlays have no `on_frame_encoded`, and `RUNE_FRAME_LOG` covers windows only | `overlay::tests::each_presented_frame_is_one_line_in_the_log` |

Dropped when the fork moved to upstream `f3ab74c1`:

- `RUNE_DESKTOP_FRAME_LOG` from `lens-fixes`: upstream's `--desktop --trace FILE`
  writes each frame's `cpu_ms`.

The `fork macOS tests` workflow (`.github/workflows/fork-macos.yml`) runs the
patched areas' tests on macOS for each push to `attn`.
