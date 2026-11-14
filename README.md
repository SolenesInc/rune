# Rune

This is [SolenesInc/rune](https://github.com/SolenesInc/rune), attn's own fork of
[anomalyco/rune](https://github.com/anomalyco/rune). attn changes it freely and
takes upstream changes by cherry-pick. Apps pin revisions of `main`, which is
never rewritten; `dev` mirrors upstream. [FORK.md](FORK.md) lists every change
we made and every upstream commit we picked.

A native GPU-rendered Rust UI framework for macOS, Windows, Linux and the web, with its own renderer, text, layout and springs.

Every app can run headless, so an agent can render to PNG, dump the element tree and drive it with scripted input.

<p align="center">
  <a href="demo/rune.mp4">
    <img src="demo/rune-poster.png" alt="Rune in motion: real apps, rune-kit components and GPU effects" width="100%">
  </a>
</p>

<p align="center">
  <img src="demo/opencode/session-top.png" alt="A coding agent app built with Rune" width="100%">
</p>

<p align="center">
  <img src="demo/light/chat.png" alt="Lit chat interface" width="49%">
  <img src="demo/kit/diff-split-dark.png" alt="Split diff component" width="49%">
</p>
<p align="center">
  <img src="demo/fx/liquid-metal.png" alt="Liquid metal effect" width="49%">
  <img src="demo/settings-dark.png" alt="Settings window" width="49%">
</p>

```rust
fn view(s: &State, _: &mut Cx<State>) -> El {
    let t = tok::theme(true);
    let label = text(if s.on { "Lights on" } else { "Lights off" })
        .font(type_::TITLE).color(t.text);
    let knob = el().key("knob").size(18.0, 18.0).radius(9.0).fill(t.knob)
        .absolute(if s.on { 18.0 } else { 2.0 }, 2.0);
    let switch = el().key("switch").role("switch").label("Lights").checked(s.on)
        .focusable(true).size(38.0, 22.0).radius(11.0)
        .fill(if s.on { tok::accent(0, true) } else { t.control_off })
        .motion(motion::SNAPPY)
        .on_press(|s: &mut State| s.on = !s.on)
        .child(knob);
    col().w_full().h_full().center().gap(16.0).fill(t.bg).child(label).child(switch)
}
```

## Use it with your agent

Give your coding agent this repo. Have it start from [`examples/hello`](examples/hello) and [`crates/rune/README.md`](crates/rune/README.md), then check its own work headlessly by rendering PNGs and inspecting the element tree.

> Build me a <thing> with Rune (github.com/anomalyco/rune). Start from examples/hello and check your work headlessly.

### Claude Code

This repository is a plugin marketplace with one plugin, `rune`. Its skill tells an agent to build screens from rune-kit and to look at a headless render before it reports UI work done.

```
/plugin marketplace add SolenesInc/rune
/plugin install rune@rune
```

`claude plugin eval plugins/rune` runs its eval cases, each with and without the plugin.

## Add it

```toml
rune = { git = "https://github.com/SolenesInc/rune", rev = "<main rev>" }
```

Rune is pre-release. Pin a rev of `main`; `main` is never rewritten.

[Docs](crates/rune/README.md) · [Components](crates/rune-kit/README.md) · [Examples](examples) · [Changelog](CHANGELOG.md)

## Contributing

Changes to this fork follow [AGENTS.md](AGENTS.md) and attn's
`docs/internal/rune-fork.md`. Do not take this fork's work to upstream's issues,
pull requests or discussions.

MIT, © Anomaly.
