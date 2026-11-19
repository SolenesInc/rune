#!/usr/bin/env bash
# Runs `cargo test` with the given arguments. AGENTS.md and attn's scripts/rune-bump run the
# fork's tests through it, so a change to how tests run is made once, here.
set -euo pipefail
exec cargo test "$@"
