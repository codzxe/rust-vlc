# AUDIT: crates/demux/tests/mkv_corpus.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 292 / 6 (2 #[test]], 3 helpers, 1 struct) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — integration test. Worse: in the default lane both tests are green NO-OPS unless `LUMEN_CORPUS` is set or `/tmp/my-project/corpus-v4` exists — which it does not in this sandbox, yet the baseline records both as "ok". |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `ls /tmp/my-project/corpus-v4` (absent); baseline cargo-test-workspace.txt:2310-2314 (both tests "ok" — executed as skips); `rg` worklog.md for the promised PASS/FAIL table (no matches; worklog:2035 admits "corpus absent here"); `rg` docs/feature-matrix.toml (mkv row cites src/mkv/** + fixture lanes, NOT this file). |

## 1. One-paragraph verdict
The comparison logic is real — live ffprobe oracle, per-stream packet counts,
codec identity, kind, damage-free requirement, packet-storm guard, and a
cues-based seek round trip with a keyframe-safe landing bound. But the file's
evidence value in this tree is nil: the corpus directory does not exist, both
tests early-return green, and the recorded baseline "ok" for them proves
nothing. The module doc promises "the per-file PASS/FAIL table for this
instance lives in the worklog entry" — no such table exists anywhere in
worklog.md, and the worklog itself admits the corpus was absent ("corpus rows
[VERIFY] — corpus absent here", worklog.md:2035). A differential that never
ran, reported green, with its recorded-results pointer dangling.

## 2. What actually works (verified)
- read: the differential's judgment core is genuine — `assert_eq!(fail, 0)`
  after per-file comparison (line 221), packet-storm guard (176), damage
  breaks `clean` (165-167), track-count mismatch noted (181-183).
- Baseline cargo-test-workspace.txt:2313-2314: both tests "ok" — verified to
  be NO-OP skips (corpus absent), which is exactly the problem, not a pass.
- The CLI-side committed-fixture matroska lane DID run: baseline:3017
  `containers::mkv::tests::corpus_matroska_lanes_match_the_ffmpeg_truth ...
  ok` — the matrix row's evidence cites that lane, not this file (checked
  docs/feature-matrix.toml:138).

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/mkv_corpus.rs:103-108 (and 229-232)
Code:
```
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent (LUMEN_CORPUS unset, /tmp/my-project/corpus-v4 missing)");
        return;
    };
```
What is wrong: an absent corpus early-returns and the test is recorded as a
green PASS. In this tree the corpus is absent (verified: `/tmp/my-project/
corpus-v4` does not exist), so the baseline "ok" at 2313-2314 tested nothing.
The module doc's claim that the PASS/FAIL table "lives in the worklog entry"
(mkv_corpus.rs:9-11) is dangling — `rg matroska_differential|MKV DIFFERENTIAL
worklog.md` returns nothing, and worklog.md:2035 admits the corpus was
absent. Why it matters: a future reader sees two green corpus differentials
in the test log and believes MKV corpus parity is continuously verified; it
is not verified anywhere in the recorded evidence. What correct looks like:
`#[ignore]` with the harness activation string (the corpus.rs precedent), so
the default lane reports "ignored", not "ok".
Confidence: HIGH — Verification: read + ls + rg + baseline citation.

F-002 [S4]
Where: crates/demux/tests/mkv_corpus.rs:249-252
Code:
```
        if demuxer.cue_index().is_none() {
            println!("PASS {name}: seek (no cues — SlowLinear path untested here)");
            continue;
        }
```
What is wrong: a file that was NOT seek-tested prints a line starting with
"PASS" — the evidence log (stdout capture) then contains PASS lines for
untested files. It also does not increment `pass`, so an all-no-cues corpus
fails the final `assert!(pass > 0)` despite nothing being wrong.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/demux/tests/mkv_corpus.rs:33-39
Code:
```
    let dir = std::env::var("LUMEN_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/my-project/corpus-v4"));
```
What is wrong: the machine-specific default path of the original dev
sandbox — on any other machine the suite silently degrades to the F-001
no-op. The path also embeds a different project name (`my-project`) than the
repo.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/tests/mkv_corpus.rs:67-88, 193, 199
Code:
```
            codec_name: field("codec_name").unwrap_or_default(),
            kind: field("codec_type").unwrap_or_default(),
```
What is wrong: (a) the hand-rolled JSON scrape silently yields empty strings
on any shape change, and the comparisons at 193/199 SKIP empty fields — a
ffprobe JSON change would quietly weaken the differential to a packet-count
check instead of failing; (b) `.must()` on the ffprobe output (61-62) panics
untyped when ffprobe is absent while the corpus is present.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| corpus_dir | 33-39 | REAL | S4 | F-003 hardcoded path |
| ProbeStream / ffprobe_streams | 42-89 | REAL | S4 | F-004 brittle scrape, panic on missing ffprobe |
| kind_probe_name | 94-101 | REAL | — | total mapping |
| corpus_matroska_differential_vs_ffprobe | 103-222 | REAL-UNPROVEN | S3 | F-001 green no-op; logic itself sound |
| corpus_matroska_seek_roundtrip | 227-292 | REAL-UNPROVEN | S4 | F-002 PASS-for-untested |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_matroska_differential_vs_ffprobe | YES — when corpus present | fail counter + assert_eq!(fail, 0), damage-free required |
| corpus_matroska_seek_roundtrip | YES — when corpus present + cues | landed-after-target and no-packets asserts |
| Both with corpus absent (the recorded state) | NO | early return, green pass |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every .mkv/.mka/.webm lane ... compared against live ffprobe truth" | mkv_corpus.rs:1-6 | REAL-UNPROVEN | logic present; no execution evidence in tree |
| "the per-file PASS/FAIL table for this instance lives in the worklog entry" | mkv_corpus.rs:9-11 | FALSE | rg over worklog.md: no such table; worklog:2035 says corpus absent |
| "When the corpus is absent ... the suite skips loudly" | mkv_corpus.rs:11 | REAL (partially) | eprintln happens, but the test result is a green "ok", not an ignore/skip in the harness sense |

## 7. Dead / duplicated / unreachable code
Nothing dead; the whole file is effectively unreachable-as-evidence in this
tree (F-001).

## 8. Dependencies & flags
Requires LUMEN_CORPUS or the hardcoded path + ffprobe on PATH. No feature
gates. `let ... && ...` chains use let-chains (edition 2024) — fine.

## 9. Open questions for the next agent
- If a corpus-v4 instance is ever regenerated, do these two tests pass?
  Untested — the only runs that exist are the no-op skips.
- `ours` is sized by `tracks.len()` but filled by zipping
  `track_entries()` (144-146): if start() ever filters entries, the codec
  columns misalign with the probe comparison. Assert the two lengths agree.
