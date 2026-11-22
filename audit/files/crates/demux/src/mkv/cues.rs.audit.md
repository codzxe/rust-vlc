# AUDIT: crates/demux/src/mkv/cues.rs

| Field | Value |
|---|---|
| Auditor | W05-A05 |
| Date | 2026-11-22 |
| Lines / functions | 213 / 5 (CueIndex::landing_before, parse_cues, parse_cue_point, parse_cue_track_positions, 1 test) |
| File verdict | REAL-UNPROVEN (parser) — with two doc-vs-code contradictions (F-001, F-002) and a consumer-side seek hole (F-003) |
| Reachable from shipped entry point? | YES — `apps/cli` → `containers/mod.rs:57 "matroska" => mkv::open` → `MkvDemuxer::new` → `Demuxer::start` (mod.rs:909) → `open()` walk → `mod.rs:416-422 ebml::ID_CUES => { self.cues = Some(cues::parse_cues(...)?) }` → `cues.rs:75 parse_cues`. Seek consumers: `mod.rs:840 landing()` ← `mod.rs:1003 seek()` / `mod.rs:1026 reanchor()`. |
| Findings | S0:0 S1:0 S2:3 S3:2 S4:4 |
| Commands run | Read (full file + mod.rs seek paths 770-1045); Grep for `relative_pos`, `cue_table_too_large`, `CuePoint` across the repo; sed -n verification of every quote. No cargo (adaptation) — W00 baseline (1751 tests pass) is the only execution evidence, per-test attribution UNVERIFIED. |

## 1. One-paragraph verdict

The cue parser itself is small, bounded (65 536-entry ceiling, typed refusal), and its binary-search `landing_before` is correct and unit-tested. But three things are wrong at the seams this file exists for. First, the module doc promises that a cue-bomb "degrades to `SeekSupport::SlowLinear`" — the code makes the over-ceiling refusal propagate through `?` in the open walk, so a cue-bombed file **fails to open entirely**; the promised degradation path does not exist anywhere. Second, the demuxer's `landing()` hard-errors (`seek_out_of_range`) whenever the cue index exists but has no point at-or-before the target — so a spec-legal empty `Cues` element advertises `SeekSupport::Fast { indexed: true }` while **every seek fails**, and seeking to t=0 in any file whose first cue point is later than 0 fails instead of falling back to the linear scan that sits unused directly below. Third, the file's own comments misdescribe the code twice ("first `CueTrackPositions`" — the code keeps the last; "keeps the *earliest* cluster per time" — no such logic exists). `CueRelativePosition` is parsed into a field nothing ever reads.

## 2. What actually works (verified)

— `landing_before` (cues.rs:50-66) is a correct last-element-≤-key binary search; the only test in the file (196-212) pins floor semantics at exact hits, before-first (None), and beyond-last.
— `cluster_pos` is bounds-checked at its only consumer: `mod.rs:843-847` `segment_data_start.checked_add(point.cluster_pos).filter(|p| *p < self.segment_end)` — a hostile cue offset cannot overflow or escape the segment (read + derivation).
— Allocation is genuinely capped: `cues.rs:91-96` refuses past `MAX_CUE_ENTRIES`; degenerate points (`Ok(None)`) push nothing, so memory tracks pushed points only.
— Points are sorted after parse (`cues.rs:103`), so `landing_before`'s precondition holds even for out-of-order muxer output.
— `mkv_corpus.rs:245-271` exercises real seek/reanchor over the corpus through this index (W00 baseline ran it green — per-test attribution UNVERIFIED).

## 3. Findings

### F-001 [S2] — Promised degradation to SlowLinear does not exist: a cue-bomb refuses the whole file
Where: crates/demux/src/mkv/cues.rs:10-14 vs crates/demux/src/mkv/mod.rs:416-422
Code (doc, verbatim):
```
//! The index is memory-bounded (MP §12): a `Cues` element declaring
//! more than [`MAX_CUE_ENTRIES`] entries is the typed
//! [`crate::mkv::error::cue_table_too_large`] refusal and the demuxer
//! degrades to `SeekSupport::SlowLinear` — never an unbounded
//! allocation.
```
Code (reality, verbatim):
```
                ebml::ID_CUES => {
                    self.cues = Some(cues::parse_cues(
                        &mut self.reader,
                        &child,
                        self.segment_end,
                    )?);
                }
```
What is wrong: `parse_cues` returns `Err(cue_table_too_large)` past the ceiling; the `?` propagates out of the open walk → `open()` fails → `start()` fails → the file does not open. Grep for `cue_table_too_large` across the crate: only the constructor (error.rs:158) and its unit test — **no caller catches it**. `SeekSupport::SlowLinear` is reachable only when `self.cues.is_none()` (a file with no Cues element, mod.rs:1019-1022), never as a degradation.
Why it matters: the documented hostile-input behavior is false in the direction that matters — the doc claims the file still plays (linear seek); the code refuses to open it. A legitimate long file (91 h at one cue per 5 s cluster) crosses the ceiling and becomes unopenable.
What correct looks like: in the walk, catch `cue_table_too_large`, leave `self.cues = None`, record damage, continue — which makes the module doc true and `seekability()` fall to SlowLinear naturally.
Confidence: HIGH — Verification: read + Grep (no catching caller exists)

### F-002 [S2] — Cue index present but no landing point ⇒ hard seek error; empty `Cues` advertises Fast while every seek fails
Where: crates/demux/src/mkv/mod.rs:841-851 + 1015-1024 (consumer of this file's index)
Code (verbatim, mod.rs:841-851):
```
        if let Some(index) = self.cues.as_ref() {
            if let Some(point) = index.landing_before(target_ticks) {
                let pos = self
                    .segment_data_start
                    .checked_add(point.cluster_pos)
                    .filter(|p| *p < self.segment_end)
                    .ok_or_else(error::seek_out_of_range)?;
                return Ok((point.time_ticks, pos));
            }
            return Err(error::seek_out_of_range());
        }
```
Code (verbatim, mod.rs:1019-1022):
```
        if self.cues.is_some() {
            SeekSupport::Fast { indexed: true }
        } else {
            SeekSupport::SlowLinear
        }
```
What is wrong: (a) A spec-legal empty `Cues` element (payload size 0) parses to `Ok(CueIndex { points: [] })` and is stored as `Some(..)` — `seekability()` then reports `Fast { indexed: true }` while `landing_before` always returns `None` and every `seek`/`reanchor` errors `seek_out_of_range`. (b) More generally, any target at/before the first cue point's time (e.g. seek to t=0 in a file whose first cue point is at t>0 — legal, e.g. a stream opening with non-keyframe material) hard-errors instead of falling through to the bounded linear cluster scan that sits directly below (mod.rs:852-883).
Why it matters: this is the file's whole job — cue seek correctness. The index's presence turns "no point before target" from a solvable problem (land at first cluster) into a user-visible seek failure, and lies about capability via `seekability()`.
What correct looks like: on `landing_before == None`, fall through to the linear scan (or clamp to the first cluster); store an empty index as `None`.
Confidence: HIGH — Verification: read (control flow); no corpus file in the W00 run is known to hit it — trigger likelihood UNVERIFIED
Filed here (not in mod.rs) because the index semantics are this file's contract; flagged for the mkv/mod.rs agent.

### F-003 [S2] — `CueTime` u64→i64 wrap: hostile values silently become negative ticks
Where: crates/demux/src/mkv/cues.rs:126-132
Code (verbatim):
```
            ebml::ID_CUE_TIME => {
                if len > 8 {
                    return Err(error::bad_structure(child.data_pos));
                }
                let bytes = reader.read_exact_at(len, child.data_pos, child_end)?;
                time = ebml::uint_from_be(&bytes).map(|v| v as i64);
            }
```
What is wrong: `CueTime` is an unsigned vint on disk; `v as i64` wraps values above `i64::MAX` (a 5-second edit: `0xFFFF_FFFF_FFFF_FFFF` → `-1`). The wrapped point sorts first (`sort_by_key` on the now-negative value) and `landing_before` will hand it back for any target ≥ it, silently seeking to an arbitrary (bounds-checked, so not unsafe) cluster. The crate's own discipline for out-of-range untrusted values is a typed refusal (`bad_structure`/`timecode_overflow` patterns elsewhere) — here it is a silent wrap.
Why it matters: demuxer input is untrusted by definition (§3.2); this is a silent wrong-value on a hostile field, and it also poisons the "time-ascending" assumption downstream code may hold.
What correct looks like: `i64::try_from(v).map_err(|_| error::bad_structure(child.data_pos))?` — refuse what does not fit the segment clock.
Confidence: HIGH — Verification: read; trigger requires a hostile file (no corpus evidence — UNVERIFIED whether any W00 fixture carries such a CueTime)
Severity note: bumped S3→S2 per the "when unsure, rate higher" rule because it is unchecked arithmetic on untrusted demuxer input with a silent wrong-seek outcome.

### F-004 [S3] — Blanket `len > 8` refusal rejects spec-legal `CueTrackPositions` children
Where: crates/demux/src/mkv/cues.rs:169-172
Code (verbatim):
```
        let len = usize::try_from(child.size).map_err(|_| error::bad_structure(child.data_pos))?;
        if len > 8 {
            return Err(error::bad_structure(child.data_pos));
        }
```
What is wrong: the check runs before the ID match, so **any** child of `CueTrackPositions` larger than 8 bytes refuses the whole Cues parse (→ whole open, per F-001's propagation). `CueCodecState` (ID 0x6B) and `CueReference` (0xDB) are legal master children of `CueTrackPositions` in the Matroska spec and can legitimately exceed 8 bytes — such files fail to open.
What correct looks like: apply the width check only to the uint children (`CueTrack`, `CueClusterPosition`, `CueRelativePosition`); skip unknown/masters by declared size like every other parser in this crate does.
Confidence: MEDIUM-HIGH — Verification: read; the spec-legality of CueCodecState/CueReference in this position is from the Matroska elements table, no such fixture in the corpus to run (UNVERIFIED)
Severity: S3 rather than S2 only because both elements are rare in the wild; the rejection is still wrong-on-valid-input.

### F-005 [S4] — Comment says "the first `CueTrackPositions`", code keeps the last
Where: crates/demux/src/mkv/cues.rs:107 vs 133-135
Code (verbatim, 107):
```
/// One `CuePoint`: `CueTime` + the first `CueTrackPositions`.
```
Code (verbatim, 133-135):
```
            ebml::ID_CUE_TRACK_POSITIONS => {
                positions = parse_cue_track_positions(reader, &child, child_end)?;
            }
```
What is wrong: plain assignment in the child loop — the **last** `CueTrackPositions` wins. For a multi-track file (the normal ffmpeg/mkvtoolnix layout: one CuePoint per keyframe time with one CueTrackPositions per track) the index keeps only the final track's cluster. Usually harmless (tracks share clusters), but the comment describes the opposite of the code, and no test covers multi-positions points (the wiring test builder `mkv_extras_wiring.rs:141-151` writes exactly one).
Confidence: HIGH — Verification: read + Grep (no multi-positions fixture)

### F-006 [S4] — "the demuxer keeps the *earliest* cluster per time" — no such logic exists
Where: crates/demux/src/mkv/cues.rs:35-37
Code (verbatim):
```
    /// The track the entry indexes (multi-track cues repeat targets;
    /// the demuxer keeps the *earliest* cluster per time).
    pub track: u64,
```
What is wrong: nothing in this file or mod.rs deduplicates by time or selects an earliest cluster; `sort_by_key` is stable (file order for ties) and `landing_before` returns the **last** of equal-time points. The doc describes an implementation that was never written.
Confidence: HIGH — Verification: read + Grep (no dedup/earliest logic in mod.rs)

### F-007 [S4] — `CueRelativePosition` is parsed into a field no consumer reads
Where: crates/demux/src/mkv/cues.rs:33-34 + 180; consumer mod.rs:840-851
Code (verbatim, 33-34):
```
    /// The in-cluster byte offset when `CueRelativePosition` declared
    /// one (rare; 0 = cluster start).
    pub relative_pos: u64,
```
What is wrong: grep for `relative_pos` across the repo: only this file (field, parse, its own test fixture). `landing()` uses `point.cluster_pos` only — the seek always lands at the cluster start. Safe (conservative) but the parse work and the module-doc claim "`CueRelativePosition` narrows inside it" (line 6) describe precision that is never applied.
Confidence: HIGH — Verification: read + Grep

### F-008 [S4] — "three orders of headroom" is 91×, not 1000×
Where: crates/demux/src/mkv/cues.rs:21-23
Code (verbatim):
```
/// The cue-entry ceiling (MP §12 index memory; ~1 h at one cue per
/// 5 s cluster is 720 entries — three orders of headroom).
pub const MAX_CUE_ENTRIES: usize = 65_536;
```
What is wrong: 65 536 / 720 ≈ 91 — under two orders of magnitude. The number itself is fine; the claimed headroom is off by ~11×.
Confidence: HIGH — Verification: arithmetic

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MAX_CUE_ENTRIES | 21-23 | REAL | S4 | ceiling enforced (91); doc's headroom math wrong (F-008) |
| struct CuePoint | 26-38 | REAL | S4 | relative_pos dead (F-007); track doc false (F-006) |
| struct CueIndex | 42-46 | REAL | — | plain model |
| CueIndex::landing_before | 48-67 | REAL | — | correct floor binary search; tested |
| parse_cues | 75-105 | REAL-UNPROVEN | S2 | ceiling refusal propagates to open failure (F-001); sorts post-parse (good); unknown children skipped by size (good) |
| parse_cue_point | 109-149 | REAL-UNPROVEN | S3/S4 | CueTime `as i64` wrap (F-003); last-positions-wins vs doc (F-005); degenerate → Ok(None), siblings survive (good) |
| parse_cue_track_positions | 153-189 | REAL-UNPROVEN | S3 | blanket len>8 refusal (F-004); closure capture of loop vars is correct; requires both track+cluster else None (degenerate skip, good) |
| tests::landing_before_is_floor_binary_search | 196-212 | REAL | — | honest, would fail on a broken search |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| landing_before_is_floor_binary_search (196) | YES | pins None-before-first, exact-hit floor, mid-range floor, beyond-last |

The parse functions have **no unit tests in this file** — no test for: over-ceiling refusal, degenerate points, multi-positions points, unknown-size children, CueTime width >8, empty Cues. Coverage exists only indirectly (mkv_corpus.rs reanchor loop over corpus files; mkv_extras_wiring.rs single-positions fixtures). None of the five defects F-001..F-005 can be caught by the current suite: the wiring builder writes one CueTrackPositions per point (mkv_extras_wiring.rs:141-151) and wraps negative times to 0 (`u64::try_from(*time).unwrap_or(0)`, line 148) — the exact hostile shapes are the ones the builders cannot emit.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the demuxer degrades to `SeekSupport::SlowLinear`" past the ceiling | cues.rs:12-14 | FALSE | F-001: `?` at mod.rs:421 propagates; no catch anywhere (Grep) |
| "`CueRelativePosition` narrows inside it" | cues.rs:5-6 | FALSE (as applied) | F-007: parsed, never consumed |
| "the first `CueTrackPositions`" | cues.rs:107 | FALSE | F-005: last wins |
| "the demuxer keeps the *earliest* cluster per time" | cues.rs:36-37 | FALSE | F-006: no such logic |
| "three orders of headroom" | cues.rs:21-22 | FALSE | 91× (F-008) |
| "time-ascending — muxers write ascending, and the parser enforces nothing about order" | cues.rs:40-41 | REAL | sort at line 103 makes it ascending regardless |

## 7. Dead / duplicated / unreachable code

- `CuePoint.relative_pos` + the `ID_CUE_RELATIVE_POSITION` parse branch: dead data (F-007).
- The bounded linear cluster scan (mod.rs:852-883) is unreachable whenever `self.cues` is `Some` — including the `Some(empty)` case (F-002), where it is exactly the fallback that would save the seek.

## 8. Dependencies & flags

No feature gates; unconditionally compiled (`lib.rs:50 pub mod mkv`), reachable from the shipped CLI (chain in header). Deps: `media_types::EngineError`, in-crate `ebml`/`error`. `#[cfg(test)] mod tests` at line 191 is test-only; no test_support usage.

## 9. Open questions for the next agent

- mkv/mod.rs agent: F-001/F-002 are behavior bugs in your file's walk/landing — the catch-and-degrade and the no-point fallback. Also confirm whether any test in mod.rs's own suite pins `cue_table_too_large` propagation as *intended* open-failure (if a test asserts the file fails to open, the doc is the only thing wrong; if not, the behavior is too).
- mkv_corpus.rs / mkv_extras_wiring.rs agents: does any corpus file carry multi-`CueTrackPositions` points, empty `Cues`, or first-cue-time > 0? (Determines whether F-002/F-005 fire on real files — UNVERIFIED here.)
- Wave 19: feature-matrix rows citing MKV seek — do they cite "Fast indexed seek"? If so, F-002's Fast-while-broken case matters to the row verdict.
