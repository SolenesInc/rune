#!/usr/bin/env bash
# Runs `cargo test` with the given arguments, one run at a time on this machine. Compiling
# crates/rune's lib tests peaks at about 24 GB (attn's docs/internal/rune-fork.md has the
# measurement), so two at once can exhaust a shared machine's memory.
#
# The lock is held by this script's own process, which runs cargo as a child and waits for it.
# The lock's file descriptor is closed on exec, so nothing cargo starts can hold it: a daemon a
# build starts, such as the sccache server, outlives the run, and with the descriptor it would
# keep every later run on the machine waiting.
set -euo pipefail

lock="${CARGO_HOME:-$HOME/.cargo}/rune-test.lock"
mkdir -p "$(dirname "$lock")"
exec perl -MFcntl=:flock -e '
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
  defined(my $pid = fork) or die "scripts/test.sh: cannot fork: $!\n";
  if ($pid == 0) { exec @ARGV or die "scripts/test.sh: cannot run $ARGV[0]: $!\n" }
  # Ctrl-C reaches cargo too, in the same process group; wait for it to finish rather than
  # dropping the lock while it still builds.
  $SIG{INT} = "IGNORE";
  $SIG{TERM} = sub { kill "TERM", $pid };
  waitpid($pid, 0);
  exit($? & 127 ? 128 + ($? & 127) : $? >> 8);
' "$lock" cargo test "$@"
