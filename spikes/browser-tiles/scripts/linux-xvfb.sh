#!/bin/bash
# The CEF tile under Xvfb with software Vulkan (lavapipe), driven by real X input from xdotool.
# Writes <out>/watch.jsonl (page state each second), screenshots, and <out>/cost.txt.
#   scripts/linux-xvfb.sh <binary> <out>
set -uo pipefail
bin=$1; out=$2
mkdir -p "$out"
export DISPLAY=:97
Xvfb :97 -screen 0 1280x800x24 -nolisten tcp &
xvfb=$!
trap 'kill $app $xvfb 2>/dev/null' EXIT
sleep 1
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.x86_64.json
"$bin" cef --tiles 1 --watch --shot "$out" > "$out/app.log" 2>&1 &
app=$!
sleep 12
win=$(xdotool search --onlyvisible --name "browser tiles" | head -1)
echo "window $win" >> "$out/app.log"
xdotool windowmove "$win" 0 0
sleep 1
# The tile's page starts at x=240+, y=42+ in window points; its input sits 184,42 into the page.
xdotool mousemove 424 84 click 1; sleep 1
xdotool type --delay 80 "hi"; sleep 1
import -window root "$out/typed.png"
xdotool mousemove 115 62 click 1; sleep 1
xdotool type --delay 80 "r"; sleep 1
xdotool mousemove 424 84 click 1; sleep 1
xdotool type --delay 80 "o"; sleep 1
for i in $(seq 1 15); do xdotool mousemove 500 400 click 5; sleep 0.05; done
sleep 1
import -window root "$out/scrolled.png"
# Cost while the page scrolls itself.
"$bin" cef --tiles 1 --scroll --quit-after 40 > "$out/scroll.log" 2>&1 &
scroller=$!
sleep 15
pid_cpu() { ps -o pid=,ppid=,rss=,time=,comm= --ppid "$scroller" -p "$scroller"; }
pid_cpu > "$out/cost-before.txt"
sleep 20
pid_cpu > "$out/cost-after.txt"
wait $scroller
grep stats "$out/scroll.log" > "$out/scroll-stats.txt"
kill $app 2>/dev/null
