"""One measured run: launch BrowserTiles.app parked through launchd, sample footprint and CPU of
the app and every process it is responsible for, and append one JSON line to the results file.

    python3 -I scripts/measure.py <app> <results.jsonl> <label> <warmup s> <window s> -- <app args>...

Footprint is ri_phys_footprint (what `footprint` reports), the median of 1 s samples in the
window. CPU is user+system time over the window as a percentage of one core. Each process is
put in a role by its name.
"""
import json, os, statistics, subprocess, sys, time

here = os.path.dirname(os.path.abspath(__file__))
PMEM = os.path.join(here, "pmem")


def snapshot():
    rows = {}
    for line in subprocess.run([PMEM], capture_output=True, text=True).stdout.splitlines():
        p = line.split("\t")
        if len(p) < 6:
            continue
        rows[int(p[0])] = {"ppid": int(p[1]), "resp": int(p[2]), "fp": int(p[3]), "cpu": int(p[4]), "name": p[5]}
    return rows


def role(name, pid, main):
    if pid == main:
        return "host"
    n = name.lower()
    if "gpu" in n:
        return "gpu"
    if "renderer" in n or "webcontent" in n:
        return "content"
    if "network" in n:
        return "network"
    return "other:" + name


def frontmost():
    if os.environ.get("MEASURE_FRONT") == "0":
        return "unchecked"
    out = subprocess.run(["osascript", "-e", 'tell application "System Events" to get name of first process whose frontmost is true'], capture_output=True, text=True)
    return out.stdout.strip()


def main():
    app, results, label, warmup, window = sys.argv[1], sys.argv[2], sys.argv[3], float(sys.argv[4]), float(sys.argv[5])
    args = sys.argv[sys.argv.index("--") + 1:]
    exe = os.path.join(app, "Contents/MacOS/BrowserTiles")
    job = f"dev.attn.browser-tiles.measure.{os.getpid()}"
    front_before = frontmost()
    out_log = f"/tmp/browser-tiles-{os.getpid()}.log"
    subprocess.run(["launchctl", "submit", "-l", job, "-o", out_log, "-e", out_log, "--", exe, *args, "--quit-after", str(warmup + window + 3)], check=True)
    pid = 0
    for _ in range(100):
        out = subprocess.run(["launchctl", "list", job], capture_output=True, text=True).stdout
        for line in out.splitlines():
            if '"PID"' in line:
                pid = int(line.split("=")[1].strip().rstrip(";"))
        if pid:
            break
        time.sleep(0.1)
    time.sleep(warmup)
    front_after = frontmost()
    samples = []
    t0 = time.time()
    while time.time() - t0 < window:
        samples.append((time.time(), snapshot()))
        time.sleep(1.0)
    samples.append((time.time(), snapshot()))
    subprocess.run(["launchctl", "remove", job], capture_output=True)

    def mine(rows):
        return {p: r for p, r in rows.items() if p == pid or r["resp"] == pid or r["ppid"] == pid}

    first_t, first = samples[0]
    last_t, last = samples[-1]
    roles = {}
    for p, r in mine(last).items():
        k = role(r["name"], p, pid)
        fps = [s[p]["fp"] for _, s in samples if p in s]
        cpu = (r["cpu"] - first[p]["cpu"]) / 1e9 / (last_t - first_t) * 100 if p in first else None
        e = roles.setdefault(k, {"mb": 0.0, "cpu": 0.0, "n": 0})
        e["mb"] += statistics.median(fps) / 1048576
        e["cpu"] += cpu or 0.0
        e["n"] += 1
    total = {"mb": sum(e["mb"] for e in roles.values()), "cpu": sum(e["cpu"] for e in roles.values())}
    log = open(out_log).read() if os.path.exists(out_log) else ""
    stats = [json.loads(l)["stats"] for l in log.splitlines() if l.startswith('{"stats"')]
    rec = {"label": label, "args": args, "pid": pid, "roles": {k: {"mb": round(v["mb"], 1), "cpu": round(v["cpu"], 2), "n": v["n"]} for k, v in roles.items()},
           "total": {"mb": round(total["mb"], 1), "cpu": round(total["cpu"], 2)}, "frontBefore": front_before, "frontAfter": front_after,
           "window": round(last_t - first_t, 1), "stats": stats[-1] if stats else None, "load": os.getloadavg()[0]}
    with open(results, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(json.dumps(rec))


main()
