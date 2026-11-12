"""CPU and RSS of a process and all its descendants over a window, from /proc.

    python3 -I scripts/linux-cost.py <pid> <label> <warmup s> <window s>

CPU is user+system time of the tree over the window as a percentage of one core; processes
that exit inside the window count up to their last sample. RSS is summed at the window's end.
"""
import json, os, sys, time

TICK = os.sysconf("SC_CLK_TCK")
PAGE = os.sysconf("SC_PAGE_SIZE")


def table():
    rows = {}
    for d in os.listdir("/proc"):
        if not d.isdigit():
            continue
        try:
            with open(f"/proc/{d}/stat") as f:
                s = f.read()
            with open(f"/proc/{d}/statm") as f:
                rss = int(f.read().split()[1]) * PAGE
        except OSError:
            continue
        name = s[s.index("(") + 1:s.rindex(")")]
        rest = s[s.rindex(")") + 2:].split()
        rows[int(d)] = {"ppid": int(rest[1]), "cpu": (int(rest[11]) + int(rest[12])) / TICK, "rss": rss, "name": name}
    return rows


def tree(rows, root):
    kids = {}
    for p, r in rows.items():
        kids.setdefault(r["ppid"], []).append(p)
    out, todo = [], [root]
    while todo:
        p = todo.pop()
        if p in rows:
            out.append(p)
            todo.extend(kids.get(p, []))
    return out


root, label, warmup, window = int(sys.argv[1]), sys.argv[2], float(sys.argv[3]), float(sys.argv[4])
time.sleep(warmup)
t0, first = time.time(), table()
seen = {p: first[p] for p in tree(first, root)}
time.sleep(window)
t1, last = time.time(), table()
now = {p: last[p] for p in tree(last, root)}
cpu = sum(now[p]["cpu"] - seen.get(p, {"cpu": 0})["cpu"] for p in now)
by_name = {}
for p, r in now.items():
    e = by_name.setdefault(r["name"], {"rss_mb": 0.0, "n": 0})
    e["rss_mb"] = round(e["rss_mb"] + r["rss"] / 1048576, 1)
    e["n"] += 1
print(json.dumps({"scenario": label, "cpu_pct": round(cpu / (t1 - t0) * 100, 1), "rss_mb": round(sum(r["rss"] for r in now.values()) / 1048576, 1),
                  "processes": len(now), "by_name": by_name, "load": os.getloadavg()[0]}))
