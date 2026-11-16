#!/usr/bin/env bash
# T17 - property-test catalogue and runner. Discovery only: does not add tests.
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

head1 "17a. existing property tests"
n=$(grep -rn --include=*.rs -E '#\[proptest\]|proptest!|quickcheck!' crates apps tools tests 2>/dev/null | wc -l)
say "property tests found: $n"
if [ "$n" -eq 0 ]; then
  say "NOTE: the repo has zero property-based tests (matches the documented"
  say "      'no-proptest status quo'). This is itself finding F-T17-01."
fi

head1 "17b. candidate pure functions (invariant-rich, no I/O)"
$PY - "$ROOT" > "$OUT/proptest-candidates.txt" <<'PYEOF'
import os, re, sys
root = sys.argv[1]
pat = re.compile(r'^\s*pub fn (\w+)\s*\(([^)]*)\)\s*(->\s*[^{]+)?\{', re.M)
interesting = re.compile(r'(timestamp|pts|dts|clock|timebase|rational|resample|sample_rate|'
                         r'frame|duration|tick|convert|parse|bits|read|scale|mix|channel)', re.I)
for dirpath, dirnames, files in os.walk(os.path.join(root, "crates")):
    dirnames[:] = [d for d in dirnames if d not in ("target", "tests")]
    for fn in files:
        if not fn.endswith(".rs"):
            continue
        p = os.path.join(dirpath, fn)
        src = open(p, encoding="utf-8", errors="replace").read()
        for m in pat.finditer(src):
            name, args, ret = m.group(1), m.group(2), (m.group(3) or "")
            if not interesting.search(name):
                continue
            if re.search(r'(&\s*mut|dyn\s|impl\s|Self\s*\(|&\s*self|&\s*mut\s*self)', args):
                continue
            line = src[:m.start()].count("\n") + 1
            rel = os.path.relpath(p, root).replace("\\", "/")
            print("%-28s %-28s %s:%d  %s" % (os.path.basename(dirpath), name, rel, line,
                                             (args + " " + ret).strip()[:60]))
PYEOF
head -40 "$OUT/proptest-candidates.txt"
say ""
say "candidates: $(wc -l < "$OUT/proptest-candidates.txt") -> $OUT/proptest-candidates.txt"

head1 "17c. catalogue (the 15 properties the fix program should add)"
cat <<'TXT'
 1 media-clock        monotonic under any callback sequence
 2 media-clock        no backward jump across resync
 3 media-clock        tick<->timestamp round-trip within 1 tick
 4 media-types        rational add/compare: no panic, no overflow
 5 media-types        timebase conversion round-trip
 6 demux              PTS/DTS output order non-decreasing (with B-frames)
 7 codecs             bit reader: never reads past buffer end
 8 codecs             bit reader: consumed bits <= 8*len
 9 codecs             ADTS/FLAC header parse: total function (never panics)
10 packet-queue       FIFO: push then pop returns the same item
11 packet-queue       capacity never exceeded
12 playback-engine    no illegal state transition for any command sequence
13 subtitles          every parsed cue has start <= end
14 transcode          output frame count within 1 of expected
15 audio-engine       resampler: finite input -> finite output, length within 1
TXT
