# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs
| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-23 |
| Lines / functions | 392 / 10 callables (1 test) |
| File verdict | REAL (AVI half) / SHALLOW (MP4 half pins the harness's own refusal) |
| Reachable from shipped entry point? | N/A (test) — runs in the workspace lane: baseline cargo-test-workspace.txt:4398 `scenario_09_corrupt::corrupt_media_is_typed_bounded_and_memory_neutral ... ok` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |

## 1. One-paragraph verdict
The AVI half of this corrupt-media matrix is genuinely strong: 12 committed
malformed fixtures plus 3 byte-level patches driven through the command layer
over the real engine, pinned to specific observed flavors (refusal stage.code,
damage-event counts, frame floors/ceilings), hang-bounded (512 pumps), and
memory-neutral by an exact-equality double run. I executed the shipped release
binary on the same fixtures and the AVI pins match production (trunc0 →
demux.corrupt, trunc2 → open lands then fatal first read, flip16 →
probe.unsupported, garbage0 → clean play). The MP4 half is the defect: all 10
mp4 rows pin `Refused("probe","unsupported")` — a flavor produced by THIS
HARNESS's AVI-only provider, not by the shipped host, which I executed and
which answers mp4 corruption with typed `demux.corrupt` (and plays valid
mp4+mjpeg+pcm media outright). The comment claiming production parity is
false. The cited evidence report `docs/scenarios/scenarios-b-report.md` does
not exist.

## 2. What actually works (verified)
- Baseline: `test scenario_09_corrupt::corrupt_media_is_typed_bounded_and_memory_neutral ... ok`
  (cargo-test-workspace.txt:4398).
- Fixtures exist: tests/fixtures/generated/malformed/avi_mjpeg_pcm__*.avi
  (12) and mp4_mjpeg_pcm__*.mp4 (10) — verified by ls; magic bytes inspected
  with od (flip1.mp4 carries an intact `ftyp isom` box, 424,452 bytes).
- Production cross-check (prebuilt target/release/lumen, the shipped entry):
  - `avi__trunc0` → exit 65, `error: [demux.corrupt]: truncated data ... (demux.avi.truncated)` — matches pin Refused("demux","corrupt").
  - `avi__trunc2` → open lands (Opening→Buffering, tracks 2 ready), then one
    fatal `error [demux.corrupt] truncated data ... found 324`, walk
    `Buffering → Error → Idle` — matches pin FatalFirstRead("demux.corrupt").
  - `avi__flip16` → exit 65 `[probe.unsupported] ... (probe.no-demuxer)` — matches pin Refused("probe","unsupported").
  - `garbage0` (avi + mp4 valid) → opens and plays (Buffering → Playing).
- Memory-neutrality: exact `assert_eq!(after_second, after_first)` over the
  counting allocator, with the one-time warm-up delta honestly named in the
  message (:385-391).

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:139-153
Code:
```
/// The committed class-B MP4 matrix: every row refuses at the probe
/// stage (this build's first vertical serves AVI only, D-018 — the
/// same typed answer the real host gives an mp4).
```
What is wrong: both justifications are false. (a) The shipped build does NOT
"serve AVI only": I ran the release binary on the VALID mp4 fixture and it
opens and plays it (`tracks 2 ready ... Buffering → Playing`,
tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4 — mjpeg + pcm_s16le are
first-vertical decoders and the mp4 demuxer is wired). (b) The real host does
NOT give `probe.unsupported` for corrupt mp4: `lumen open
mp4_mjpeg_pcm__flip1.mp4` → exit 65 `error: [demux.corrupt]: corrupt
structure at byte 1291 (demux.mp4.box-overruns-parent)`; `__trunc0.mp4` →
`[demux.corrupt] ... (demux.mp4.no-moov)`. The pinned flavor comes from the
LOCAL harness's provider (host.rs wires `demux` AVI only), so 10 of the 22
malformed rows assert the harness's own limitation while claiming production
parity — and the engine's REAL mp4 corruption behavior (typed, byte-precise
demux.corrupt) goes unpinned by this scenario. Why it matters: doc 10 §6.4
row 9's evidence is "per-fixture results matrix"; the mp4 half of that matrix
misrepresents what the product does. What correct looks like: either wire the
mp4 demuxer into the scenarios_b provider and pin the real flavors
(`demux.corrupt`/`demux.mp4.*`), or state plainly "the local harness has no
mp4 demuxer; these rows pin the harness refusal, not production".
Confidence: HIGH — Verification: RAN the shipped binary (4 invocations, exit
codes 65/0/65/0 recorded above); read host.rs:23-30 (AVI-only provider
composition).

F-002 [S3]
Where: crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:13-15
Code:
```
//! The observed flavors (all real, pinned by the tuning run this
//! file was adjusted against — the matrix is the evidence in
//! `docs/scenarios/scenarios-b-report.md`):
```
What is wrong: `docs/scenarios/scenarios-b-report.md` does not exist (no
docs/scenarios/ directory at all; rg over docs/ finds no such file). The
document that would carry the tuning-run derivation of the min_frames floors
(59/60/100/118) and damage counts is phantom — the pins' provenance is
unverifiable in-tree. Same defect family as media-clock's phantom
tests/virtual_time.rs. Confidence: HIGH — Verification: ls + rg over docs/.

F-003 [S4]
Where: crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:290-293
Code:
```
            assert!(
                *frames <= 121,
                "{name}: bounded recovery, not a runaway ({frames} frames)"
            );
```
What is wrong: 121 is an unexplained magic ceiling (parent = 120 frames; the
+1 slack for concealment is plausible but unstated). A comment-derived bound
from the phantom report (F-002) would be better than a bare literal.
Confidence: HIGH — Verification: read; parent frame count 120 asserted at :319-323.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Pinned / Row / Observed (types) | 38-70 | REAL | — | pinned-flavor vocabulary |
| avi_rows | 73-137 | REAL | — | 12 rows; flavors cross-checked against the shipped binary (4/4 sampled match) |
| mp4_rows | 142-153 | SHALLOW | S2 | pins the harness's probe refusal, not production (F-001) |
| Patch / patches | 157-212 | REAL | — | 3 byte-level corruptions, documented offsets (movi payload 532, avih 32, idx1 lie 423_604) |
| open_and_drive | 216-268 | REAL | — | command-layer dispatch, settle, bounded run_until_terminal(512), refusal invariants (Idle, no session, no events) |
| assert_row | 273-302 | REAL | S4 | flavor drift = unreachable! failure; magic 121 (F-003) |
| corrupt_media_is_typed_bounded_and_memory_neutral | 307-392 | REAL | — | warm-up parent, double-run matrix, exact memory-neutrality; S09_ONLY filter |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corrupt_media_is_typed_bounded_and_memory_neutral | YES (AVI half) | pinned flavors are specific (stage.code strings, exact damage counts, frame floors); any recovery-behavior drift flips a flavor assert — verified against the production binary for trunc0/trunc2/flip16/garbage0 |
| same, MP4 rows | NO (production-wise) | the pins match the local harness's provider composition; a production mp4-dmx regression (e.g. demux.corrupt → panic) would NOT fail this suite's mp4 rows — they'd still see probe.unsupported |
Memory-neutrality would catch per-session leaks (exact equality on repeat).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "corrupt file: typed Corrupt; bounded recovery count; no hang/crash; per-fixture results matrix" | docs/10-testing-ci-quality.md:689 (§6.4 row 9) | REAL for AVI / HALF-FALSE for MP4 | matrix exists in-test; AVI flavors production-verified; MP4 rows pin harness behavior (F-001) |
| "the same typed answer the real host gives an mp4" | scenario_09_corrupt.rs:140-141 | FALSE | real host answers demux.corrupt (executed, exit 65); harness answers probe.unsupported |
| "this build's first vertical serves AVI only" | scenario_09_corrupt.rs:140 | FALSE (demux layer) | real binary plays valid mp4_mjpeg_pcm.mp4 (executed) |
| "the matrix is the evidence in docs/scenarios/scenarios-b-report.md" | scenario_09_corrupt.rs:14-15 | FALSE | the file does not exist |

## 7. Dead / duplicated / unreachable code
None in-file. Note the `S09_ONLY` env knob (:338) is a debugging filter that
defaults to running everything.

## 8. Dependencies & flags
commands catalog, testkit (fixture_path, Must, CountingGlobalAlloc
serialization), crate::host (local dispatch harness). No features. Runs in the
plain workspace test lane (no ignore).

## 9. Open questions for the next agent
- The trunc2 production trail shows `error [demux.corrupt] the engine
  recovered 848456 damaged unit(s) while resynchronizing` on a 425 KB file —
  848,456 "units" from a 425,516-byte file needs a units definition check
  (demux-side; suspicious count or suspicious naming).
- scenario_12_device_loss (another agent's file) — does the "vanish" sink fake
  exercise the REAL clock device-loss path or only the harness's? Given the
  media-clock master-loss surface is unwired (my backends.rs audit), what does
  "device loss" even reach in production?
- Host teardown: the real CLI tears down at stdin EOF before natural Ended
  (garbage0 trail ends Playing→Stopped→Idle) — scenarios_b's host pumps to
  terminal instead; the harness/production pacing difference is benign here
  but should be kept in mind when reading scenario evidence.
