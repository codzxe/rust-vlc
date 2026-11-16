#!/usr/bin/env bash
# T20 - bounded model checking with Kani (proofs, not samples).
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

head1 "20a. kani availability"
if ! need cargo-kani; then
  if command -v kani >/dev/null 2>&1; then :; else
    say "install: cargo install --locked kani-verifier && cargo kani setup"
    say "(Kani needs its own toolchain and does a full re-build with --cfg kani)"
  fi
fi

head1 "20b. target catalogue (harnesses to write in the fix program)"
cat <<'TXT'
 #  crate            harness                              property
 1  media-clock      clock_tick_to_timestamp_no_overflow  no panic/overflow for any u64 input
 2  media-clock      clock_monotonic_any_callback_seq      time never decreases
 3  media-types      rational_add_no_overflow              a/b + c/d fits in range
 4  media-types      rational_no_div_by_zero               denominator != 0 checked
 5  codecs           bit_reader_bounds                     read(n) never indexes past end
 6  codecs           adts_header_total                     parse never panics
 7  codecs           flac_streaminfo_total                 parse never panics
 8  subtitles        srt_timestamp_parse_total             no panic on any input
TXT

head1 "20c. run (only if a harness exists)"
if command -v cargo-kani >/dev/null 2>&1 && [ "$#" -gt 0 ]; then
  cargo kani --harness "$1" --output-format terse 2>&1 | tee "$OUT/kani-$1.log" | tail -30
else
  not_run "kani" "no harness committed yet and/or kani not installed - harnesses are source changes (fix program)"
fi
