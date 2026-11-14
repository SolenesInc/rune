#!/usr/bin/env bash
# Runs `cargo test` with the given arguments, one run at a time on this machine. Compiling
# crates/rune's lib tests peaks at about 24 GB (attn's docs/internal/rune-fork.md has the
# measurement), so two at once can exhaust a shared machine's memory.
set -euo pipefail

lock="${CARGO_HOME:-$HOME/.cargo}/rune-test.lock"
mkdir -p "$(dirname "$lock")"
exec perl -MFcntl=:flock -e '
  $^F = 255;
  my $lock = shift;
  open(my $fh, "+>>", $lock) or die "scripts/test.sh: cannot open $lock: $!\n";
  unless (flock($fh, LOCK_EX | LOCK_NB)) {
    seek($fh, 0, 0);
    chomp(my $holder = <$fh> // "");
    print STDERR "scripts/test.sh: waiting for the Rune test run that holds $lock",
      ($holder eq "" ? "" : " (pid $holder)"), "; one runs at a time per machine\n";
    flock($fh, LOCK_EX) or die "scripts/test.sh: cannot lock $lock: $!\n";
  }
  truncate($fh, 0);
  print $fh "$$ in ", $ENV{PWD} // "", "\n";
  $fh->flush;
  exec @ARGV or die "scripts/test.sh: cannot run $ARGV[0]: $!\n";
' "$lock" cargo test "$@"
