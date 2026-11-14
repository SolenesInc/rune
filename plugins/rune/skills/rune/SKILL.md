---
name: rune
description: Build and change UI in a Rust app on Rune (the `rune` and `rune-kit` crates). Use when writing or changing a view, a component, a theme or a UI test in a crate that depends on `rune`, when asked how to check or screenshot a Rune UI, or when asked to make one look better.
---

# Building UI on Rune

Rune is a GPU UI framework for Rust: a view is a function from the app's state to a tree of elements (`El`). Two rules make a Rune screen look finished.

## 1. Build from `rune-kit`, not from `el()`

`rune-kit` components already have the spacing, type, states, focus rings, motion and accessibility of a finished control. Look for the component before you draw one:

- **Controls**: `Button`, `IconButton`, `Switch`, `Checkbox`, `RadioGroup`, `SegmentedControl`, `TextInput`, `TextArea`, `Field`, `Select`, `Combobox`.
- **Overlays**: `Menu`, `Popover`, `Tooltip`, `Dialog`, `Toast`.
- **Content**: `Card`, `List`, `Accordion`, `Collapsible`, `Badge`, `Divider`, `Spinner`, `Progress`, `Tree`, `Table`, `Markdown`, `DiffView`.
- **The window's frame** (`rune_kit::shell`): `SplitView`, `Sidebar`, `SidebarRow`, `Toolbar`, `Page`. Never build a sidebar, a toolbar or a title bar by hand.

```rust
let k = &Kit::neutral(cx.system_dark());           // or the app's own kit
let mut layer = Vec::new();                        // overlays mount here
let body = col().gap(12.0)
    .child(text("Sync").font(k.ty.body_medium).color(k.c.text_base))
    .child(Switch::new(s.sync).label("Sync").on_toggle(|s: &mut App| s.sync = !s.sync).build(k))
    .child(Button::new("Save").variant(ButtonVariant::Contrast).on_press(|s: &mut App| s.save()).build(k))
    .child(Select::new("model", &MODELS).current(Some(&s.model)).build(k, &mut layer));
rune_kit::root(body, layer)                        // the view's root
```

- An app that depends on `rune` alone has no kit yet. Add `rune-kit` to its `Cargo.toml` from the same git revision as `rune`, and make the kit once per view: `Kit::neutral(cx.system_dark())`, or `Kit::from_palette(PaletteSeed { bg, fg, accent })` for the app's own colours. Then replace hand-drawn controls (an `el()` with `.role("button")`) with the kit's as you touch them.
- Take every colour, text style, radius and shadow from the `Kit` (`k.c.text_muted`, `k.ty.body`, `k.radius.md`, `k.elevation.raised`). A literal colour or font size in a view is a defect.
- Overlays build with `.build(k, &mut layer)` and need `rune_kit::root(body, layer)`, or they never show.
- Give each element you will script, test or crop a `.key("name")`.
- The API changes between revisions, so read the source of the Rune the app pins before you write against it. `cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "rune-kit") | .manifest_path'` prints its path; three directories up are `crates/rune-kit/README.md` (every component, with samples), `crates/rune/README.md` and `examples/kit-gallery` (each component in each state).

## 2. Look at what you built

Every Rune app renders headless, with no window and no display. Before you report UI work done, render it and open the PNG:

```sh
APP=target/debug/my-app
$APP --list-scenes                                    # the named states the app can render
$APP --snap out.png --theme dark                      # the default state
$APP --snap out.png --theme light --scene NAME --size 900x600
$APP --snap out.png --playground "Group/Story" --variant "Long names"
$APP --script input.json --snap out.png --at mark:end # a state behind a click or a key
$APP --tree tree.json                                 # every element's key, text, rect and colours
```

A script is timed input: `[{"t": 0.1, "type": "tap", "key": "open-menu"}, {"t": 0.6, "type": "type", "text": "hi"}, {"t": 1.0, "type": "mark", "name": "end"}]`.

Render each screen you changed in dark and light, at its normal size and a small one, and in its hard states (empty, many items, long names, an error). In each picture check that edges and baselines align, the same relationship has the same gap, nothing clips or overflows, and one thing is primary. Fix what you see, render again, and attach the final PNGs to your report.

A state you can't reach from the command line doesn't get looked at. When you add a screen or a component, add a scene (`AppSpec { scenes: &[("empty", |s| s.items.clear())], .. }`) or a playground story (`Story::new("Sidebar/Row", init, view).variant("Long names", |s| ..)`) for each state. "Agent tooling" and "Playground" in `crates/rune/README.md` have the rest.
