---
type: llm
---

PASS if the answer writes a JSON input script whose events have a time `t`, a `type` of `tap` (or `click`, or `down` then `up`) and `"key": "more"`, and runs the app's binary with `--script <file>` and `--snap <png>` at a time after the tap (with `--at`, for example `--at mark:end` or a number of seconds).
FAIL if it uses a display, Xvfb, xdotool, a browser or another screenshot tool, or invents a script format without timed events, or takes the snapshot with no way to make it land after the tap.
