# AUDIT: crates/demux/src/mkv/cues.rs
| Field | Value |
|---|---|
| Auditor | W05-A01 |
| Date | 2026-11-21 |
| Lines / functions | 213 / 4 (+1 test, 1 const) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `apps/cli/src/containers/mod.rs:57` → `containers/mkv.rs:147-148` `MkvDemuxer::new`+`start()` → `crates/demux/src/mkv/mod.rs:416-421` `ID_CUES => self.cues = Some(cues::parse_cues(…)?)`; seek lane: `mod.rs:840-851` `landing()` → `CueIndex::landing_before`.) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 0 |
| Commands run | Read of cues.rs + relevant mod.rs/error.rs sections; `rg -n "cue_table_too_large\|cue_index\|landing_before\|seekability"` (mod.rs:213, 842, 1015-1023; error.rs:158); `rg -n "relative_pos" crates apps` (no consumer); `rg -n "MAX_CUE_ENTRIES\|65_536" crates/demux/tests/` (no ceiling test); `rg -n -i "cue" crates/demux/tests/*.rs` (wiring builders :136-153, :317-323; corpus :224-260); baseline `rg -n "mkv::cues\|mkv_corpus" audit/runs/baseline/test-workspace.log.txt`. |

## 1. One-paragraph verdict
The cue parser itself is real, bounded (65,536-entry ceiling enforced before allocation), and its floor binary-search is correct and tested; real corpus seek round-trips passed in baseline. But the file's central resilience promise is false: both its own module doc and the error constructor's doc claim that an over-ceiling `Cues` table degrades the demuxer to linear seek, while the only call site propagates the refusal with `?` — the whole MKV fails to open. Secondary: `CueRelativePosition` is parsed and documented as narrowing the landing but no code ever reads it; the "first `CueTrackPositions`" and "keeps the earliest cluster per time" doc claims contradict the code (last-wins assignment, no dedupe); and `CueTime` is cast `u64→i64` without a wrap guard.

## 2. What actually works (verified)
— `landing_before` floor binary search: correct implementation (:50-66), unit-tested with boundary probes (test :196-212), executed in baseline (`audit/runs/baseline/test-workspace.log.txt:1783` `test mkv::cues::tests::landing_before_is_floor_binary_search ... ok`).
— Ceiling guard counts `CuePoint` children before parsing (:91-96) — degenerate points included; memory bounded at 65,536 × 32 B.
— Integration seek on real files: `test corpus_matroska_seek_roundtrip ... ok` (baseline :2143, mkv_corpus.rs:224-260 — seek to mid-point, land at-or-before, keep demuxing); hand-built two-point cue tables drive `demuxer.seek()` in mkv_extras_wiring.rs:317-323, :653-663 (baseline :2166+ 9 tests ok).
— Walk-loop safety: `pos = child_end` strictly advances; unknown children skipped by declared size; unknown-size child → typed `bad_structure` (:87-89).

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/mkv/cues.rs:8-14 + crates/demux/src/mkv/mod.rs:416-421 (+ crates/demux/src/mkv/error.rs:154-157)
Code:
```rust
//! The index is memory-bounded (MP §12): a `Cues` element declaring
//! more than [`MAX_CUE_ENTRIES`] entries is the typed
//! [`crate::mkv::error::cue_table_too_large`] refusal and the demuxer
//! degrades to `SeekSupport::SlowLinear` — never an unbounded
//! allocation.
```
```rust
                ebml::ID_CUES => {
                    self.cues = Some(cues::parse_cues(
                        &mut self.reader,
                        &child,
                        self.segment_end,
                    )?);
```
What is wrong: the documented degradation does not exist. `parse_cues` returns `Err(cue_table_too_large)` past 65,536 entries (cues.rs:91-96) and the only call site (`mod.rs:416-421`) propagates it with `?` out of `start()` — the entire open fails. `error.rs:154-157` doubles down: "the index is refused (the seek falls back to the bounded cluster scan)". There is no fallback: `self.cues` stays `None` only when the element is absent, and `seekability()` (mod.rs:1015-1023) reports `SlowLinear` only for a demuxer that *opened*.
Why it matters: >65,536 cue points is realistic — an all-intra or long MKV (the ceiling's own doc math: 1 cue/5 s ≈ 720/h ⇒ 65,536 ≈ 91 h, but all-keyframe encodes emit one cue per keyframe: a 2 h 24 fps all-intra file carries ~172,800). Such a file refuses to open entirely, which is exactly the class of "MKV cannot be opened" failure this audit exists for, and two doc comments claim the opposite behavior. No test feeds an over-ceiling cue table (`rg MAX_CUE_ENTRIES crates/demux/tests/` → no hits), so the divergence is uncaught.
What correct looks like: at mod.rs:416-421, match on the error kind/code and on `cue_table_too_large` leave `self.cues = None` (the existing `SlowLinear` path) instead of `?`; or fix both doc comments to say the file is refused.
Confidence: HIGH — Verification: read of all three sites; caller sweep shows no other `parse_cues` caller.

### F-002 [S3]
Where: crates/demux/src/mkv/cues.rs:32-34 (and :6, :180) vs crates/demux/src/mkv/mod.rs:843-848
Code:
```rust
    /// The in-cluster byte offset when `CueRelativePosition` declared
    /// one (rare; 0 = cluster start).
    pub relative_pos: u64,
```
```rust
                let pos = self
                    .segment_data_start
                    .checked_add(point.cluster_pos)
                    .filter(|p| *p < self.segment_end)
                    .ok_or_else(error::seek_out_of_range)?;
```
What is wrong: `CueRelativePosition` is parsed (:180), stored, and documented as narrowing the landing "inside [the cluster]" (:5-6, :32-34), but the only consumer (`landing`, mod.rs:843-848) adds `cluster_pos` alone — `relative_pos` is never read (`rg relative_pos crates apps` → cues.rs definition/assignment/test-literal only). Landings stay cluster-granular.
Why it matters: parsed-then-dropped field: dead data on the hot index struct and a documented precision feature that does not exist; on files that carry relative positions the demuxer re-scans the cluster from its start — a correctness-neutral but real precision/perf gap, and a doc lie.
What correct looks like: `checked_add(point.cluster_pos)?.checked_add(point.relative_pos)` (with the same segment-bound filter), or delete the field + parsing arm.
Confidence: HIGH — Verification: read + repo-wide consumer sweep.

### F-003 [S3]
Where: crates/demux/src/mkv/cues.rs:107 and :133-135 (claim :35-37)
Code:
```rust
/// One `CuePoint`: `CueTime` + the first `CueTrackPositions`.
/// `Ok(None)` = a degenerate point (skip — the others survive).
```
```rust
            ebml::ID_CUE_TRACK_POSITIONS => {
                positions = parse_cue_track_positions(reader, &child, child_end)?;
            }
```
```rust
    /// The track the entry indexes (multi-track cues repeat targets;
    /// the demuxer keeps the *earliest* cluster per time).
    pub track: u64,
```
What is wrong: three doc/code divergences on the multi-track form. (a) :107 says "the first `CueTrackPositions`" but the loop at :133-135 *overwrites* `positions` per child — the LAST positions master of the point wins. (b) :35-37 claims "the demuxer keeps the *earliest* cluster per time" — no such selection exists anywhere (`landing` at mod.rs:841-848 takes the binary-search hit directly; with duplicate `CueTime`s the stable sort + floor search returns the LAST duplicate, not the earliest cluster). (c) `CuePoint.track` is written by the parser and read by no consumer (`rg` sweep: only Debug/PartialEq/test literals) — ffmpeg/mkvtoolnix write one `CueTrackPositions` per track inside a single `CuePoint`, and all but the last are silently dropped with the track field then unused.
Why it matters: multi-track cues are the spec's normal shape for multi-track files; whichever positions master comes last decides the landing for every track — an arbitrary, undocumented choice that can send video seeks to the audio track's cluster (same cluster in practice, but the contract says otherwise and nothing verifies it).
What correct looks like: either parse all `CueTrackPositions` into a `Vec` and let `landing` pick per-track/earliest, or fix the three comments to say "last positions master wins, track field retained for diagnostics".
Confidence: HIGH — Verification: read of both loops and the sole consumer.

### F-004 [S3]
Where: crates/demux/src/mkv/cues.rs:126-132
Code:
```rust
            ebml::ID_CUE_TIME => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                time = ebml::uint_from_be(&bytes).map(|v| v as i64);
            }
```
What is wrong: `CueTime` is an unsigned EBML uint up to 8 bytes; `v as i64` wraps values ≥ 2^63 to negative `time_ticks` instead of a typed refusal. A hostile cue table with `CueTime = 0xFFFF…` produces negative index entries that sort first and can be returned by `landing_before` for negative queries.
Why it matters: unchecked `as` cast on a parsed value (the audit checklist's own class); downstream `landing` returns the negative ticks as a seek landing (`Ok((point.time_ticks, pos))`, mod.rs:848). No crash, hostile-input-only, wrong-by-wrap instead of refused.
What correct looks like: `i64::try_from(v).map_err(|_| error::bad_structure(child.data_pos))?` (mirroring the `usize::try_from` refusals the same function already uses at :124).
Confidence: HIGH — Verification: read; wrap semantics of `as` are defined Rust behavior.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CuePoint` (struct) | 26-38 | REAL | F-002, F-003 | `relative_pos` dead, `track` write-only, doc claims diverge |
| `CueIndex` (struct) | 42-46 | REAL | — | plain Vec |
| `CueIndex::landing_before` | 50-66 | REAL | — | correct floor binary search; unit-tested; duplicates → last (F-003b context) |
| `parse_cues` | 75-105 | REAL | F-001 | ceiling guard before allocation (good); sort normalizes order (doc :40-41 says "enforces nothing" — stale wording, sort at :103 does enforce ascending); unknown-size child refused |
| `parse_cue_point` | 109-149 | REAL | F-003, F-004 | last-positions-wins; `as i64` wrap; degenerate `Ok(None)` skip honest |
| `parse_cue_track_positions` | 153-189 | REAL | — | len>8 refused for every child (:170-172); missing track/cluster → `Ok(None)` (degenerate, honest) |
| `MAX_CUE_ENTRIES` | 21-23 | REAL | F-001 | guard works; its failure mode is the file-killing refusal |
| tests: `landing_before_is_floor_binary_search` | 196-212 | REAL | — | honest, bites (boundary probes -1/0/150/300/999) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `landing_before_is_floor_binary_search` (in-file) | YES | exact floor semantics probed at 5 boundaries incl. None-leg |
| (integration) mkv_extras_wiring seek tests | YES | two-point cue tables through the real demuxer; wrong landing fails "lands at or before" (:653-663) |
| (integration) `corpus_matroska_seek_roundtrip` | YES for cue files; SOFT for cue-less ones | mkv_corpus.rs:249-250 `println!("PASS … no cues")` + `continue` — a corpus without indexes passes without testing seek (test-side softness, not this file's) |
Untested: the >65,536 ceiling refusal (F-001 uncaught), multi-`CueTrackPositions` order (F-003 uncaught), `CueRelativePosition` parse (F-002 uncaught — `rg ID_CUE_RELATIVE_POSITION` → cues.rs only), the `as i64` wrap (F-004 uncaught).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the demuxer degrades to `SeekSupport::SlowLinear`" on over-ceiling Cues | cues.rs:8-14 | FALSE | mod.rs:416-421 `?` propagates; no fallback (F-001) |
| "the seek falls back to the bounded cluster scan" | error.rs:154-157 | FALSE | same (F-001) |
| "`CueRelativePosition` narrows inside it" | cues.rs:5-6 | FALSE (parsed, never applied) | mod.rs:843-848 (F-002) |
| "the first `CueTrackPositions`" | cues.rs:107 | FALSE (last wins) | :133-135 (F-003) |
| "the demuxer keeps the *earliest* cluster per time" | cues.rs:35-37 | FALSE (no such logic) | mod.rs:841-848 (F-003) |
| "the parser enforces nothing about order" (index doc) | cues.rs:40-41 | MISLEADING (stale) | :103 `sort_by_key` normalizes to ascending — which the binary search requires |
| matrix `container-matroska-webm` "cues-based seeking" | docs/feature-matrix.toml:132 (description) | REAL (cluster-granular) | corpus seek roundtrip ok, baseline :2143; relative-position narrowing not implemented |

## 7. Dead / duplicated / unreachable code
- `CuePoint.relative_pos` — parsed, stored, never read (F-002).
- `CuePoint.track` — stored, never read by any consumer (F-003).
- No `unsafe`, no unwrap/expect/panic/todo in the file (house rule kept).

## 8. Dependencies & flags
`media_types::EngineError`, `crate::mkv::ebml`, `crate::mkv::error` — std-only, no external deps; no feature gates (module unconditional, `pub mod mkv` lib.rs:50). In-test only: `testkit::Must`.

## 9. Open questions for the next agent
- mod.rs owner: confirm whether any caller above `start()` maps `cue-table-too-large` to a non-fatal path (I found none — apps/cli `wire_error` converts and surfaces it).
- If F-001 is fixed by degrading, `seekability()` should be re-checked: `cues = None` yields `SlowLinear`, and the bounded scan budget (`MAX_SEEK_SCAN_BYTES`) becomes the new ceiling on long files — does it refuse past-budget scans with `scan_too_slow` (mod.rs:866-867)? That error's blast radius is unmeasured here.
