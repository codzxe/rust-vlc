#!/usr/bin/env bash
# T19 - concurrency model checking (loom / shuttle).
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

head1 "19a. existing concurrency models"
n_loom=$(grep -rn --include=*.rs -E 'loom::|#\[cfg\(loom\)\]' crates apps tools 2>/dev/null | wc -l)
n_shuttle=$(grep -rn --include=*.rs -E 'shuttle::' crates apps tools 2>/dev/null | wc -l)
say "loom usages:    $n_loom"
say "shuttle usages: $n_shuttle"
if [ "$n_loom" -eq 0 ] && [ "$n_shuttle" -eq 0 ]; then
  say "NOTE: no concurrency model exists yet -> this task is PARTIAL:"
  say "      writing a loom model is a source change, which this program does not do."
fi

head1 "19b. concurrency surface census (where a model is needed)"
grep -rn --include=*.rs -E '\b(Arc|Mutex|RwLock|AtomicU?[0-9]+|Condvar|mpsc|crossbeam)\b' crates 2>/dev/null \
  | grep -v '/tests/' \
  | awk -F: '{print $1}' | sort | uniq -c | sort -rn | head -20 \
  | tee "$OUT/concurrency-surface.txt"

head1 "19c. model catalogue (for the fix program)"
cat <<'TXT'
 target                  invariant
 media-clock             time never decreases; one epoch per tick
 packet-queue            bounded capacity; no lost/duplicated item
 playback-engine         no illegal state transition under concurrent commands
 audio-output            shutdown while playing: no deadlock, no UAF
 renderer-wgpu           frame submission ordering
TXT
