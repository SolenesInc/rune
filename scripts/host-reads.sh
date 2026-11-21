#!/usr/bin/env bash
# Checks that a run without a window acts the same on every host. Code under
# crates/rune/src/runtime and crates/rune/src/text asks `Platform::current()`, which a test run
# can set, not the build's OS (`cfg!(target_os = ...)`, `#[cfg(target_os = ...)]`,
# `target_vendor`). Lint attributes (`cfg_attr(..., allow(...))`) and test files don't count.
# The files that talk to the real OS are listed in scripts/host-read-files.txt.
# Usage: scripts/host-reads.sh (also run by scripts/file-size.sh)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

allowed=scripts/host-read-files.txt
rule="asks the build's OS; ask rune::Platform::current() for platform behavior, and list the file in $allowed only when it talks to the real OS"

hits=$(git ls-files -z 'crates/rune/src/runtime.rs' 'crates/rune/src/runtime/*.rs' 'crates/rune/src/text.rs' 'crates/rune/src/text/*.rs' \
  | xargs -0 rg -n --no-heading '(cfg!\(|#\[cfg\().*target_(os|vendor)' 2>/dev/null \
  | grep -Ev '^[^:]*(_tests\.rs|/tests\.rs|/tests/)' \
  | grep -vFf <(grep -v '^#' "$allowed" | sed 's/$/:/') || true)

if [ -z "$hits" ]; then
  echo "host-reads: no host read in runtime/ or text/ outside $allowed"
  exit 0
fi
while IFS= read -r hit; do
  file=${hit%%:*} rest=${hit#*:} line=${rest%%:*}
  echo "$file:$line: $rule"
  [ "${GITHUB_ACTIONS:-}" = true ] && echo "::error file=$file,line=$line::$rule"
done <<< "$hits"
exit 1
