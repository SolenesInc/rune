#!/bin/bash
# The CEF tile under Xvfb with software Vulkan (lavapipe), driven by real X input from xdotool,
# then the cost of no tile, one idle tile and one scrolling tile.
#   scripts/linux-xvfb.sh <binary> <out>
# Writes <out>/drive.log (page state each second), screenshots, and <out>/cost.jsonl.
set -uo pipefail
here=$(cd "$(dirname "$0")" && pwd)
bin=$1; out=$2
mkdir -p "$out"
export DISPLAY=:97
Xvfb :97 -screen 0 1280x800x24 -nolisten tcp &
xvfb=$!
app=""
trap '[ -n "$app" ] && kill $app 2>/dev/null; kill $xvfb 2>/dev/null' EXIT
sleep 1
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json
export LD_LIBRARY_PATH="$CEF_PATH${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

"$bin" cef --tiles 1 --watch --shot "$out" > "$out/drive.log" 2>&1 &
app=$!
sleep 12
win=$(xdotool search --onlyvisible --name "browser tiles" | head -1)
echo "window $win" >> "$out/drive.log"
xdotool windowmove "$win" 0 0
xdotool windowfocus --sync "$win"
sleep 1
import -window root "$out/loaded.png"
mark() { echo "{\"mark\":\"$1\"}" >> "$out/drive.log"; sleep 1.5; }
xdotool mousemove 425 85 click 1; mark page-click
xdotool type --delay 80 "hi"; mark page-typed
import -window root "$out/typed.png"
xdotool mousemove 115 62 click 1; xdotool type --delay 80 "r"; mark rune-field-typed
xdotool mousemove 425 85 click 1; xdotool type --delay 80 "o"; mark back-in-page
for i in $(seq 1 15); do xdotool mousemove 500 400 click 5; sleep 0.05; done; mark scrolled
import -window root "$out/scrolled.png"
kill $app; wait $app 2>/dev/null; app=""

for scenario in none idle1 scroll1; do
  case $scenario in
    none) args="--tiles 0" ;;
    idle1) args="--tiles 1" ;;
    scroll1) args="--tiles 1 --scroll" ;;
  esac
  "$bin" cef $args --quit-after 40 > "$out/$scenario.log" 2>&1 &
  app=$!
  python3 -I "$here/linux-cost.py" "$app" "$scenario" 15 20 >> "$out/cost.jsonl"
  wait $app 2>/dev/null; app=""
  grep '"stats"' "$out/$scenario.log" | tail -1 | sed "s/^/{\"scenario\":\"$scenario\",\"app\":/; s/\$/}/" >> "$out/cost.jsonl"
done
cat "$out/cost.jsonl"
