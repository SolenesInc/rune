---
type: llm
---

PASS if the answer adds the `rune-kit` crate at the same git revision as `rune`, creates a `Kit` (for example `Kit::neutral(..)` or `Kit::from_palette(..)`), and builds the new toggle and the new button from rune-kit components (`Switch`, `Button`).
FAIL if it draws the new toggle or the new button by hand from `el()`, or gives the new controls literal colours or font sizes, or never adds `rune-kit` to `Cargo.toml`.
