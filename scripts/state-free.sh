#!/usr/bin/env bash
# Checks that the runtime stays compiled once, whatever the app's state type. `Runtime<S>` is a
# typed wrapper that derefs to `RuntimeCore`; a method in an `impl` over `Runtime<S>` still
# compiles, but is built again for every state type that calls it, along with everything it
# reaches. In a test build with hundreds of state types that cost is what made the build peak
# at 24 GB. Only crates/rune/src/runtime/app_side.rs, the typed side, may implement on it.
# Usage: scripts/state-free.sh (also run by scripts/file-size.sh)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

allowed=crates/rune/src/runtime/app_side.rs
rule="an impl over Runtime<S> is built again for every state type; write it as \`impl RuntimeCore\` (only $allowed implements on Runtime<S>)"

hits=$(git ls-files -z 'crates/rune/src/*.rs' | xargs -0 rg -n --no-heading -U --multiline-dotall \
  '^[ \t]*impl\b[^{;]*?\bRuntime[ \t\n]*<' 2>/dev/null | grep -v "^$allowed:" | grep -E '^[^:]+:[0-9]+:' || true)

if [ -z "$hits" ]; then
  echo "state-free: no impl over Runtime<S> outside $allowed"
  exit 0
fi
while IFS= read -r hit; do
  file=${hit%%:*} rest=${hit#*:} line=${rest%%:*}
  echo "$file:$line: $rule"
  [ "${GITHUB_ACTIONS:-}" = true ] && echo "::error file=$file,line=$line::$rule"
done <<< "$hits"
exit 1
