---
type: llm
---

PASS if the answer renders the app headless to a PNG with the app's own `--snap` flag, tells the reader to open and inspect the PNG, names at least two concrete things to check in it (for example alignment, spacing, clipped or overflowing text, long names, the empty state), and covers more than the default state (another theme, size, scene or story).
FAIL if it relies on a display, Xvfb, a screenshot tool or a window, or if it only says to run the tests, or if it never says to look at the picture.
