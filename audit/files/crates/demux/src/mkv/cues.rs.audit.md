# AUDIT: crates/demux/src/mkv/cues.rs
| Field | Value |
|---|---|
| Auditor | W05-A10 |
| Date | 2026-11-23 |
| Lines / functions | 213 / 4 prod (+1 test) |
| File verdict | REAL (with one FALSE doc claim and a dead-data seam) |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/mod.rs:57 `"matroska" => mkv::open` -> MkvDemuxer::start -> mod.rs:417 `cues::parse_cues`; seek path: demux_host.rs:615 `self.inner.reanchor(target)` -> mod.rs:1026 reanchor -> mod.rs:840 `landing()` -> cues.rs:50 `landing_before` -> `point.cluster_pos` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | `./target/debug/deps/demux-036dd705535b2fb3 mkv::cues` -> 1 passed, exit 0; `./target/debug/deps/mkv_extras_wiring-23e4f7993cf3807f` -> 9 passed, exit 0; rg for `cue_table_too_large` (no catch site), `relative_pos`/`point.track` (no consumer), `reanchor` (wiring); baseline cargo-test-workspace.txt:2310-2316 (mkv_corpus 2 passed incl. corpus_matroska_seek_roundtrip) |

## 1. One-paragraph verdict
The cue parser is real, bounds-disciplined (monotone walk, unknown-size children refused, MAX_CUE_ENTRIES=65,536 before push, 8-byte field ceiling, `usize::try_from` on sizes) and the binary search `landing_before` is correct and pinned by a discriminating test; the seek path that consumes it bounds-checks `segment_data_start + cluster_pos < segment_end` before landing (mod.rs:843-847), so cues -> cluster position cannot escape the segment. The defects are a promised-but-missing degradation path (the module doc claims an over-ceiling cue table degrades to `SeekSupport::SlowLinear`; the code propagates the typed refusal through `start()` and the file refuses to open — no caller catches it), a doc that says "the first CueTrackPositions" while the code keeps the last (multi-track cue points lose all but one track's positions), and two dead fields (`relative_pos`, `track`) that no consumer reads.

## 2. What actually works (verified)
— verified by `./target/debug/deps/demux-036dd705535b2fb3 mkv::cues` -> `test result: ok. 1 passed` (floor search: None below first, exact 0/300, floor 150->100, past-end 999->300).
— verified by `./target/debug/deps/mkv_extras_wiring-23e4f7993cf3807f` -> `9 passed; 0 failed` (synthetic Cues-before-clusters fixtures exercise parse_cues + landing end-to-end).
— verified by baseline cargo-test-workspace.txt:2310-2316: `corpus_matroska_seek_roundtrip ... ok` (real ffmpeg-authored corpus, cue-indexed seek to last/2 with packet flow after reanchor).
— verified by hand: walk progress is strict (`pos = child_end`, `child_end >= data_pos + size >= pos + 2`), the ceiling fires before push (91-96), CueTime width >8 is refused (127-129).

## 3. Findings
F-001 [S2]
Where: crates/demux/src/mkv/cues.rs:10-14 (claim) vs crates/demux/src/mkv/mod.rs:416-421 (code)
Code (cues.rs:10-14):
```
//! The index is memory-bounded (MP §12): a `Cues` element declaring
//! more than [`MAX_CUE_ENTRIES`] entries is the typed
//! [`crate::mkv::error::cue_table_too_large`] refusal and the demuxer
//! degrades to `SeekSupport::SlowLinear` — never an unbounded
//! allocation.
```
Code (mod.rs:416-421):
```
                ebml::ID_CUES => {
                    self.cues = Some(cues::parse_cues(
                        &mut self.reader,
                        &child,
                        self.segment_end,
                    )?);
```
What is wrong: The claim "the demuxer degrades to SeekSupport::SlowLinear" is false. The `?` propagates `cue_table_too_large` out of `start()`; rg over the whole repo finds no catch site for `cue_table_too_large` / code `demux.mkv.cue-table-too-large` (only error.rs's constructor and its tests). A file whose Cues element carries more than 65,536 points — realistic for long or high-keyframe-rate content (65,537 cues ≈ 91 h at one per 5 s, or ~2 h at one keyframe per 100 ms) — refuses to OPEN instead of playing with linear seek. `SeekSupport::SlowLinear` is only ever reported when cues are absent (mod.rs:1019-1023). Why it matters: the module's stated bounded-memory design ("never an unbounded allocation" AND playable) is half-implemented: the allocation bound exists, the degradation does not; the user-visible behavior contradicts the file's contract. What correct looks like: catch the typed refusal at mod.rs:417, leave `self.cues = None`, damage-note, continue start (the SlowLinear path already exists).
Confidence: HIGH — Verification: read + rg (no catch site) + ran (seek tests above prove the normal path; the refusal path has no test either)

F-002 [S3]
Where: crates/demux/src/mkv/cues.rs:107-108 and 133-134; struct doc 35-37
Code:
```
/// One `CuePoint`: `CueTime` + the first `CueTrackPositions`.
/// `Ok(None)` = a degenerate point (skip — the others survive).
```
```
            ebml::ID_CUE_TRACK_POSITIONS => {
                positions = parse_cue_track_positions(reader, &child, child_end)?;
            }
```
What is wrong: The doc says "the first CueTrackPositions"; the code keeps the LAST (every `ID_CUE_TRACK_POSITIONS` child overwrites `positions`). Standard Matroska (mkvmerge) writes ONE CueTrackPositions PER TRACK inside each CuePoint — all but the final track's entry are silently discarded. The struct doc then claims "the demuxer keeps the *earliest* cluster per time" (lines 35-37) — no such logic exists: mod.rs:841-848 takes the single retained point, and `landing_before` returns the LAST of equal-time duplicates after the stable sort (cues.rs:103), not the earliest. Impact is usually masked (same-time keyframes share a cluster), but a point whose tracks point at different clusters lands at the last track's cluster, which can sit after another track's keyframe cluster. Zero test coverage: every fixture builder writes exactly one positions per point (tests/mkv_extras_wiring.rs:138-154). What correct looks like: keep the minimum cluster_pos across all positions children (matching the doc), or fix both docs and document the last-wins rule.
Confidence: HIGH (doc-vs-code contradiction, overwrite semantics, no consumer dedup) / MEDIUM (real-world divergence frequency) — Verification: read + rg
F-003 [S3]
Where: crates/demux/src/mkv/cues.rs:33-37 (fields), module doc 4-6
Code:
```
    /// The in-cluster byte offset when `CueRelativePosition` declared
    /// one (rare; 0 = cluster start).
    pub relative_pos: u64,
    /// The track the entry indexes (multi-track cues repeat targets;
    /// the demuxer keeps the *earliest* cluster per time).
    pub track: u64,
```
What is wrong: `relative_pos` and `track` are parsed and stored, then read by nothing outside this file: rg shows the only CuePoint field consumers are `point.cluster_pos` and `point.time_ticks` (mod.rs:845, 848). The module doc advertises "`CueRelativePosition` narrows inside it" (lines 5-6) — the landing never applies it (mod.rs:843-848 lands at the cluster start). Landing at cluster start is seek-safe (earlier is safe), so this is dead data plus a doc implying functionality that does not exist, not a seek bug. The `cue_index()` accessor (mod.rs:213) exposes the whole structure; its only caller checks `is_none()` (tests/mkv_corpus.rs:249).
Confidence: HIGH — Verification: rg (field consumers repo-wide)
F-004 [S4]
Where: crates/demux/src/mkv/cues.rs:40-41 vs 103; and 131
Code:
```
/// The parsed index (time-ascending — muxers write ascending, and the
/// parser enforces nothing about order; the seek binary-searches).
```
```
    index.points.sort_by_key(|p| p.time_ticks);
```
What is wrong: The doc describes a parser that "enforces nothing about order" — under that description the binary search in `landing_before` would be broken on non-ascending muxer output; the actual code sorts (line 103), which is what makes the search valid. Stale doc narrating a pre-sort design; the code is safer than the doc. Also line 131 `time = ebml::uint_from_be(&bytes).map(|v| v as i64)` — an unchecked uint->i64 cast: a CueTime above i64::MAX wraps negative (practically unreachable at any real timestamp scale, but the cast is silent).
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CueIndex::landing_before | 48-67 | REAL | — | correct floor binary search; pinned by test incl. None/floor/past-end |
| parse_cues | 75-105 | REAL | S2 | bounds good; ceiling refusal propagates as open failure, not the documented degradation (F-001); sorts (doc says it doesn't, F-004) |
| parse_cue_point | 109-149 | REAL | S3 | last-wins positions vs "first" doc; multi-track data loss (F-002); degenerate points skipped OK |
| parse_cue_track_positions | 153-189 | REAL | — | 8-byte ceiling, try_from, unknown-size children refused |
| tests::landing_before_is_floor_binary_search | 196-212 | REAL | — | discriminating; parse functions untested in-file (integration covers the single-positions shape only) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| landing_before_is_floor_binary_search | YES | 5 asserts pin None-below-first, exact hit, floor, past-end |
| (no parse tests in-file) | N/A | parse_cues/point/positions covered only by integration fixtures with one positions per point (multi-positions untested anywhere) and the corpus roundtrip (environment-conditional) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the demuxer degrades to SeekSupport::SlowLinear" (over-ceiling cues) | cues.rs:12-13 | FALSE | mod.rs:417-421 `?` propagates; no catch site repo-wide (rg) |
| "CueTime + the first CueTrackPositions" | cues.rs:107 | FALSE | line 134 overwrites; last wins |
| "the demuxer keeps the *earliest* cluster per time" | cues.rs:35-36 | FALSE | no min/dedup logic in mod.rs:840-850 or here; landing_before returns last duplicate |
| "CueRelativePosition narrows inside it" | cues.rs:5-6 | FALSE (parsed, never applied) | rg: relative_pos has no consumer |
| "memory-bounded ... never an unbounded allocation" | cues.rs:13-14 | REAL | ceiling check before push (91-96); 65,536 × 32 B ≈ 2 MB |
| "the parser enforces nothing about order; the seek binary-searches" | cues.rs:40-41 | FALSE (stale — contradicts the sort at 103) | read |

## 7. Dead / duplicated / unreachable code
- `relative_pos`, `track` fields (F-003): dead repo-wide.
- `CueIndex` exposed wholesale via `cue_index()`; only presence is consumed.

## 8. Dependencies & flags
- crate-internal only (ebml, error) + media_types; no feature gates; no unsafe; no unwrap/expect outside tests. MAX_CUE_ENTRIES carries a `[PROPOSAL]`-style rationale comment (verified arithmetic: 720/h does fit with 3 orders of headroom).

## 9. Open questions for the next agent
- mkv/mod.rs owner: confirm whether the `?` at mod.rs:417 (cues) is intended refusal (then fix the cues.rs doc) or intended degradation (then catch it) — F-001.
- Does any corpus fixture actually carry multi-CueTrackPositions points (per-track positions)? If yes, corpus_matroska_seek_roundtrip partially covers F-002's shape; if no, the data-loss path is untested everywhere. UNVERIFIED (corpus bytes not inspected).
- The Demuxer trait's `seek` (mod.rs:1003-1012) computes the landing and discards `_pos` — repositioning happens only in `reanchor` (mod.rs:1026). Verify the engine never calls `seek` expecting motion (out of this file's scope).
