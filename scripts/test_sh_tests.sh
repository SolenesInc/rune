#!/usr/bin/env bash
# Checks scripts/test.sh's lock: a run whose cargo leaves a daemon behind (as a build that starts
# the sccache server does) must not keep the next run waiting, and cargo's exit code comes back.
# Uses a stand-in cargo and its own lock file; touches nothing of the machine's.
set -uo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
export PATH="$root/scripts/test_sh_tests:$PATH" CARGO_HOME=$(mktemp -d) DAEMONS=$(mktemp)
cleanup() {
  while read -r _ pid; do kill "$pid" 2>/dev/null; done < "$DAEMONS"
  rm -f "$DAEMONS" "$CARGO_HOME/rune-test.lock"; rmdir "$CARGO_HOME"
}
trap cleanup EXIT
fail=0
bash "$root/scripts/test.sh" >/dev/null 2>&1 || { echo "FAIL: the first run exited $?"; fail=1; }
timeout 10 bash "$root/scripts/test.sh" >/dev/null 2>&1
rc=$?
[ "$rc" -eq 0 ] || { echo "FAIL: the second run exited $rc (124: it waited on a lock the first run's daemon holds)"; fail=1; }
FAKE_RC=3 timeout 10 bash "$root/scripts/test.sh" >/dev/null 2>&1
rc=$?
[ "$rc" -eq 3 ] || { echo "FAIL: cargo exited 3, the script $rc"; fail=1; }
[ "$fail" -eq 0 ] && echo "ok: scripts/test.sh releases its lock when cargo exits and passes its exit code on"
exit "$fail"
