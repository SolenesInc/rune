---
type: llm
---

PASS if the code builds the toggle, the picker, the button and the sidebar from rune-kit components (such as `Switch`, `Select` or `SegmentedControl`, `Button`, `Sidebar` or `SplitView`), and takes its colours and text styles from a kit or theme value (such as `k.c.text_muted` or `k.ty.body`).
FAIL if it draws the toggle, the button or the sidebar by hand from bare elements, or if the view sets colours or font sizes as literals (hex values, `rgb(..)`, numeric font sizes).
