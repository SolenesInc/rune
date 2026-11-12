#!/bin/bash
# Measured grid: <runs> rounds of each scenario for one engine, appended to <results.jsonl>.
#   scripts/grid.sh <app> <engine> <results.jsonl> <runs> [extra args...]
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
app=$1; engine=$2; results=$3; runs=$4; shift 4
park=${GRID_PARK---park 3300,300}
for r in $(seq 1 "$runs"); do
  for scenario in none idle1 scroll1 idle2; do
    case $scenario in
      none) args="--tiles 0" ;;
      idle1) args="--tiles 1" ;;
      scroll1) args="--tiles 1 --scroll" ;;
      idle2) args="--tiles 2" ;;
    esac
    python3 -I "$here/measure.py" "$app" "$results" "$engine-$scenario" 10 20 -- $engine $args $park "$@" > /dev/null
    tail -1 "$results" | python3 -I -c 'import json,sys; r=json.loads(sys.stdin.read()); print(r["label"], r["total"], "front", r["frontBefore"], "->", r["frontAfter"], "load", round(r["load"],1))'
  done
done
