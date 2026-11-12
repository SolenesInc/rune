"""Median [min-max] per label and role from measure.py results.

    python3 -I scripts/summarize.py <results.jsonl>...
"""
import json, statistics, sys
from collections import defaultdict

runs = defaultdict(list)
for path in sys.argv[1:]:
    for line in open(path):
        r = json.loads(line)
        runs[r["label"]].append(r)


def cell(values):
    if not values:
        return "-"
    return f"{statistics.median(values):.1f} [{min(values):.1f}-{max(values):.1f}]"


def role_of(k):
    return k if not k.startswith("other:") else "other"


for label in sorted(runs):
    rs = runs[label]
    roles = sorted({role_of(k) for r in rs for k in r["roles"]})
    print(f"\n{label}  runs={len(rs)}  load={cell([r['load'] for r in rs])}  focus kept={all(r['frontBefore'] == r['frontAfter'] for r in rs)}")
    for role in roles + ["total"]:
        mb, cpu = [], []
        for r in rs:
            if role == "total":
                mb.append(r["total"]["mb"]); cpu.append(r["total"]["cpu"])
            else:
                m = sum(v["mb"] for k, v in r["roles"].items() if role_of(k) == role)
                c = sum(v["cpu"] for k, v in r["roles"].items() if role_of(k) == role)
                mb.append(m); cpu.append(c)
        print(f"  {role:10s} MB {cell(mb):24s} CPU% {cell(cpu)}")
    frames = [t["frames"] for r in rs if r.get("stats") for t in r["stats"].get("tiles", [])]
    if frames:
        copy = [t["copyMs"] / max(t["frames"], 1) for r in rs if r.get("stats") for t in r["stats"].get("tiles", [])]
        print(f"  frames/run {cell(frames)}  copy ms/frame {cell(copy)}")
