# AUDIT: crates/demux/examples/walk_bench.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Lines / functions | 534 / 9 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — cargo example (dev bench tool); not built by `cargo build --release -p lumen-cli -p desktop` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 5 |
| Commands run | prebuilt `target/debug/examples/walk_bench` executed 6×: usage (no args); `--mode mem --walks 3 --warmup 1` → WALKBENCH line, exit 0; `--sizes` → 2 WALKSIZES lines, exit 0; `--allocs` → WALKALLOCS line, exit 0; relative path `--mode file` → exit 2 "uri.file.relative-path"; absolute path `--mode file` → exit 0; `/nonexistent` → exit 66; `--walks 0` → exit 64 |

## 1. One-paragraph verdict
A real, working bench harness that I executed end-to-end on the committed AVI fixture
in all four protocols (timing mem, timing file, --allocs, --sizes): 120 packets /
422,244 payload bytes consistently, cross-walk disagreement guard armed, typed exit
codes (2/3/4/5/64/66) all distinct and reachable. The core numbers it prints are
self-consistent (mem median 119 µs vs file 233 µs for the same 120 packets in a
debug build). The defects are documentation/protocol-precision ones: the module doc's
"~0.9 alloc events/packet" is stale — the current binary measures 0.033/pkt (27×
better than documented); the documented "walk" definition includes open+new which
the timer and the alloc guard both exclude; a relative FILE path dies with a cryptic
typed error; --sizes median semantics differ from the documented exact median.

## 2. What actually works (verified)
- `./target/debug/examples/walk_bench tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
  --walks 3 --warmup 1 --mode mem` -> "packets=120 payload_bytes=422244 recovered=0
  damage=0 median_us=119" — matches the fixture's known geometry (60 video + 60 audio
  samples; expectation toml records video_sample_count = 60).
- `--sizes` -> track 0: 60 packets min 1097 med 1161 max 1201; track 1: 60 packets
  all 5880 (PCM constant-rate — exactly right for s16le stereo 44.1 kHz,
  5880 = 4 bytes × 1470... consistent with the AVI PCM interleaving).
- `--allocs --walks 3 --warmup 1` -> "median_alloc_events=4 events_per_pkt=0.0333".
- Absolute-path `--mode file` -> WALKBENCH line exit 0; `/nonexistent` -> exit 66;
  `--walks 0` -> exit 64 "needs a positive integer"; no-args -> usage + exit 64.
- Cross-walk counter-agreement guard (exit 4) read: compares (packets, payload_bytes,
  recovered, damage) tuples per walk — a flaky demuxer would abort the bench.

## 3. Findings
F-001 [S4]
Where: crates/demux/examples/walk_bench.rs:53-57
Code:
```
//! counter would hide allocations — D-5). Timing runs therefore carry
//! the counter's per-event cost (W5-15 measured 17.7–22.4 ns per
//! alloc+dealloc pair); with the post-W6-04 walk at ~0.9 alloc
//! events/packet that is ~0.01% of a walk — but state the protocol
```
What is wrong: the current binary measures 0.0333 alloc events/packet (4 events per
120-packet walk, median of 3 measured walks) — the documented "~0.9" is ~27× stale.
The instrument-cost argument still holds (real cost is lower), but the stated number
is false for the code as it stands. The 17.7–22.4 ns pair cost is a historical
W5-15 measurement — UNVERIFIED (no way to re-measure the allocator pair cost here).
Confidence: HIGH on the discrepancy — Verification: ran (`--allocs` output above).

F-002 [S4]
Where: crates/demux/examples/walk_bench.rs:7-10 vs 134-136 and 163-165
Code:
```
//! One **walk** = open the source (through `input`'s `FileAccess` — the
//! production byte path — or over the file's bytes in memory, the
//! I/O-floor mode) → `AviDemuxer::new` → `start` → `next_event` to
```
vs walk() starting its clock after construction:
```
fn walk(demuxer: &mut AviDemuxer) -> Result<WalkStats, EngineError> {
    let t0 = Instant::now();
    demuxer.start()?;
```
and walk_counted bracketing only the drain:
```
    demuxer.start()?;
    let guard = ALLOC.state().guard();
```
What is wrong: the documented walk includes open + `AviDemuxer::new`, but the timing
clock starts at `start()` and the alloc guard is taken after `start()` — open/new
costs (and start()'s allocations) are excluded from both reported numbers while the
doc defines them into the walk. AviDemuxer::new is construction-only (verified at
crates/demux/src/avi/mod.rs:360-377), so the exclusion is small, but the protocol
description and the measurement bracket disagree. What correct looks like: state the
bracket in the doc, or move the clock/guard to cover the documented span.
Confidence: HIGH — Verification: read + ran.

F-003 [S4]
Where: crates/demux/examples/walk_bench.rs:119-129 with usage at 61-64/452
Code:
```
    let uri = media_types::ids::SourceUri::from_file_path(text)?;
    input::FileAccess::new().open(&uri)
```
What is wrong: a relative FILE path (the natural invocation shape shown in the usage
doc) is typed-refused by SourceUri ("[open] unsupported (uri.file.relative-path)",
exit 2) with no hint that an absolute path is required; the doc at :115-118 discusses
POSIX/Windows behavior but never the relative-path refusal. What correct looks like:
canonicalize the path or document the requirement. Confidence: HIGH — Verification:
ran (both invocations).

F-004 [S4]
Where: crates/demux/examples/walk_bench.rs:413-414 vs 23-24
Code:
```
        let med = sizes[n / 2];
```
What is wrong: --sizes reports the upper-middle element as "med" while the module
doc promises an exact median (mean of two middles for even N) for the timing arrays;
same tool, two median semantics, even-count rows differ silently (track rows with
even packet counts get the higher of the two middles). Confidence: HIGH —
Verification: read (track 1 output med=5880 unaffected; constant sizes).

F-005 [S4]
Where: crates/demux/examples/walk_bench.rs:505-519
Code:
```
    // The file's bytes, read once outside every measured protocol.
    let bytes: Arc<[u8]> = match std::fs::read(&path) {
```
What is wrong: the whole file is read into memory unconditionally — including
`--mode file` runs that never use `bytes` (memory cost = file size for nothing);
and when `--sizes` and `--allocs` are combined, sizes runs, then allocs runs and
returns SUCCESS, silently skipping the timing modes — combination semantics are
undocumented. Confidence: HIGH — Verification: read + ran (allocs return path).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| or_exit | 105-113 | REAL | — | typed exit-2 wrapper, no unwrap |
| open_file | 119-129 | REAL | S4 | relative-path refusal undocumented (F-003) |
| walk | 134-157 | REAL | S4 | clock bracket vs doc (F-002) |
| walk_counted | 163-193 | REAL | S4 | guard excludes start() (F-002) |
| median | 197-205 | REAL | — | exact; non-empty guaranteed by --walks > 0 |
| bench_mode | 210-290 | REAL | — | agreement guard exit 4; zero-packet exit 3; executed |
| bench_allocs | 295-362 | REAL | — | executed; feeds F-001 discrepancy |
| bench_sizes | 373-446 | REAL | S4 | upper-middle median (F-004); repeat-within-4 logic verified |
| main | 448-533 | REAL | S4 | unconditional read + flag-combination semantics (F-005) |

## 5. Test honesty
N/A (no tests in this file; it is a measurement tool). Its own honesty devices are
real: cross-walk counter agreement (exit 4), zero-packet refusal (exit 3), raw
per-walk data on stderr, median/min/max all printed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "post-W6-04 walk at ~0.9 alloc events/packet" | walk_bench.rs:55-56 | FALSE (stale) | executed: 0.0333/pkt (F-001) |
| "W5-15 measured 17.7–22.4 ns per alloc+dealloc pair" | walk_bench.rs:54-55 | UNVERIFIED | historical measurement, not re-measurable here |
| walk = open → new → start → drain | walk_bench.rs:7-10 | SHALLOW | bracket excludes open+new (F-002) |
| "Median is exact: middle … mean of the two middle" | walk_bench.rs:23-24 | REAL for timing arrays / SHALLOW for --sizes | two semantics (F-004) |
| counting allocator unconditional, cannot be mode-gated | walk_bench.rs:51-52 | REAL | `#[global_allocator]` at :80-81 is unconditional |

## 7. Dead / duplicated / unreachable code
None found; every function is called from main; all flags wired.

## 8. Dependencies & flags
Example pulls dev-deps input + testkit (alloc_count) — legitimate for an example;
not part of any shipped artifact.

## 9. Open questions for the next agent
- Was the ~0.9 allocs/pkt number true at some commit? (Worklog archaeology; does not
  change the current-code verdict.)
- The claimed docs/perf/demux-spill-fix.md evidence base (line 47) — someone auditing
  docs/perf should cross-check the committed numbers against a fresh release run
  (release build not permitted here).
