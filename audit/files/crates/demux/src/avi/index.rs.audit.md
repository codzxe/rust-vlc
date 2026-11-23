# AUDIT: crates/demux/src/avi/index.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Lines / functions | 929 / 12 production + 3 test helpers (2 tests in-file; extensive cross-file idx1 suites) |
| File verdict | REAL (one executed S2: over-cap idx1 is fatal to open) |
| Reachable from shipped entry point? | YES: `lumen` CLI → `apps/cli/src/containers/mod.rs:55 "avi" => avi::open` → `AviDemuxer::start` → `parse_container` → `index::parse_idx1` (avi/mod.rs:1193); seek paths → `search_index`/`scan_movi_with` (avi/mod.rs:1070/1078) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 5 |
| Commands run | `cargo test -p demux --lib avi::index` → 2 passed; `cargo test -p demux --lib avi` → 86 passed; standalone rlink probe (audit/scratch/probe3_idx1_cap.rs) → over-cap open failure EXECUTED |

## 1. One-paragraph verdict
The idx1 parser and its seek search are genuinely implemented and unusually well-engineered for the dirty world: clamp-before-cap for lying declared sizes, per-entry positional validation against the movi region, coalesced damage events, ffmpeg-style relative/absolute offset-base probing verified against actual chunk ids, and a file-order sort + ladder re-timing pass that makes out-of-order tables time-correct (pinned by tests). The bounds math itself is overflow-safe (saturating adds everywhere, `div_ceil` entry counts, damage counters capped by the entry cap). The one executed defect is fatal-by-choice: an idx1 whose *real* payload exceeds 524,288 entries is a typed `index-too-large` error that the caller propagates with `?` — killing the entire open of an otherwise valid file, when the demuxer's own architecture (the movi walk is the extraction truth; no-index files fully demux via `scan_movi`) provides the obvious degrade path. That path is also untested anywhere in the tree. Secondary: the idx1 is validated against the movi's *declared* end while the walk is bounded by the *effective* (clamped) end, and the no-index seek scan hard-stops at 1000 packets / 2 MiB, so long no-index files cannot seek past the cap.

## 2. What actually works (verified)
- `cargo test -p demux --lib avi::index` → 2 passed (out-of-order file-order correction with exact table/pts/damage assertions; keyframe search at-or-below semantics).
- `cargo test -p demux --lib avi` → 86 passed, including `lying_idx1_size_clamps_to_available_bytes`, `absolute_idx1_offsets_are_autodetected_and_seek`, `idx1_damage_coalesces_per_reason`, `seek_lands_on_last_keyframe_at_or_below_target`, `seek_without_idx1_walks_the_movi`, `seek_does_not_move_the_read_position`, plus adversarial_gaps.rs shuffled/reversed/duplicate-offset idx1 suites.
- Standalone probe (control at exactly the cap) opened a 524,288-entry idx1 AVI and demuxed — the parser itself is correct at the cap boundary; only cap+1 fails (see F-001).

## 3. Findings
F-001 [S2]
Where: crates/demux/src/avi/index.rs:326-329 (mint) + crates/demux/src/avi/mod.rs:1193-1204 (fatal propagation)
Code:
```
    let entry_count = payload.div_ceil(16);
    if entry_count > MAX_IDX1_ENTRIES as u64 {
        return Err(error::index_too_large(MAX_IDX1_ENTRIES as u64, entry_count));
    }
```
```
                    if let Some(mut table) = index::parse_idx1(
                        &mut self.reader,
                        pos,
                        header.size,
                        region.form_pos,
                        region.data_start,
                        region.declared_end,
                        bound,
                        &self.streams,
                        &self.filtered,
                        &mut self.damage,
                    )? {
```
What is wrong: the over-cap refusal is `Err`, and the caller propagates it with `?` out of `parse_container` → `start()` fails → the file does not open. The cap is computed from the *clamped* payload, so it fires only when the file physically contains > 8 MiB of idx1 bytes — i.e. a valid long file (per-frame-interleaved audio+video past ~2.4 h at 30 fps, or ~1.2 h at 60 fps — real capture profiles), or any file with a junk-padded large idx1. The module's own salvage doctrine says the index is never load-bearing ("the `movi` walk is the packet-extraction truth — it works with or without an index"); the honest degrade is to drop the index and keep the open (seek then falls back to `scan_movi`), exactly the `noidx0` path. Why it matters: a structurally valid AVI is unopenable because of an optional seek table; ffmpeg opens it. The `index_too_large` path has zero test coverage anywhere in the tree (rg for `index_too_large` in tests: only the ASF error-shape test). Confidence: HIGH — Verification: ran (standalone probe, audit/scratch/probe3_idx1_cap.rs: control 524,288 entries → `start OK: 1 tracks`; 524,289 entries → `start FAILED: [demux.avi.index-too-large] resource limit: memory (budget 524288, attempted 524289)`).

F-002 [S3]
Where: crates/demux/src/avi/index.rs:604-609 and 703-707
Code:
```
pub const SEEK_SCAN_CAP: ScanCap = ScanCap {
    max_packets: 1_000,
    max_bytes: 2 * 1024 * 1024,
};
```
```
        if let Some(cap) = cap
            && (packets > cap.max_packets || bytes > cap.max_bytes)
        {
            return Err(error::scan_too_slow(cap.max_bytes, bytes));
        }
```
What is wrong: the no-index seek scan refuses after 1000 packet chunks or 2 MiB — for a no-index AVI longer than that (33 s at 30 fps), any seek past the budget is a typed `scan-too-slow` failure, every time. The demuxer fully *demuxes* such files (the walk has no cap) but cannot *seek* in most of them. It is a documented proposal cap (doc 03 §6) and the refusal is typed with the position untouched, so it is honest — but the practical effect is that the `noidx0` class of files has seek support for only the first ~1000 packets. ffmpeg scans the whole movi for the landing. What correct looks like: raise the cap for local seekable sources (the scan is cheap positional header peeks — 2 MiB walked in ms) or make it a perf-tier constant. Confidence: HIGH — Verification: read (cap values + Err arm cited; budget check placement verified: fires on the (cap+1)-th packet before evaluating it). Severity S3 because it is documented, typed, and non-destructive.

F-003 [S4]
Where: crates/demux/src/avi/mod.rs:1199 passes `region.declared_end`; index.rs:391-392 validates against it
Code:
```
                        region.declared_end,
```
```
                    let data_end = chunk_pos.saturating_add(8 + u64::from(size));
                    if chunk_pos < movi_data_start || data_end > movi_data_end {
```
What is wrong: idx1 entries are validated against the movi's *declared* end (unclamped, may run past EOF/RIFF bound) while the walk itself is bounded by `region.effective_end` (avi/mod.rs:461, 813). On a file whose movi size lies past the physical end, idx1 entries pointing into the phantom region are accepted into the seek table; a seek can then report a landing the walk can never legitimately reach (the walk errors `truncated` at the effective end). Consequences are contained to a typed error, never wrong data — an inconsistency between two bounds for the same region. What correct looks like: pass `region.effective_end` like `scan_movi_with` does (avi/mod.rs:1083). Confidence: MEDIUM — Verification: read (both call sites cited; walk bound at mod.rs:461 `unwrap_or(region.effective_end)`).

F-004 [S4]
Where: crates/demux/src/avi/index.rs:519-528 and 384-388
Code:
```
    let relative = movi_form_pos.saturating_add(u64::from(rel));
    if candidate_matches(relative) {
        return movi_form_pos;
    }
    let absolute = u64::from(rel);
    if candidate_matches(absolute) {
        return 0;
    }
    movi_form_pos
```
What is wrong: the absolute-offset variant is signaled by return value `0`, which is indistinguishable from `movi_form_pos == 0` (a genuinely relative base when the movi form sits at offset 0). The caller's `if base == Some(0)` then records a false `"idx1-absolute-offsets"` damage note. Real AVIs always have RIFF+hdrl before the movi (`movi_form_pos ≥ ~80`), so this fires only on synthetic fixtures — the in-file regression test itself (tests.rs:828, `movi_form_pos = 0`) would trigger the false note if it asserted absence. What correct looks like: return an enum or `Option<bool>`. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/demux/src/avi/index.rs:431 and crates/demux/src/avi/mod.rs:1205
Code:
```
    raw.sort_by_key(|entry| entry.chunk_pos);
```
```
                        table.entries.sort_by_key(|entry| entry.chunk_pos);
```
What is wrong: `parse_idx1` already emits entries sorted by `chunk_pos` (stable sort at 431); the caller re-sorts the identical order — a redundant O(n log n) pass over up to 524k entries at open, plus misleading duplication of the ordering invariant in two places. Confidence: HIGH — Verification: read (both sites cited).

F-006 [S4]
Where: crates/demux/src/avi/index.rs:708-725
Code:
```
                if s == stream {
                    // Capture the pre-advance state: the landing chunk
                    // must re-emit its recorded pts after re-anchoring.
                    let snapshot = ladders.ticks_snapshot();
                    if let Some(step) = ladders.advance(s, u64::from(header.size)) {
                        if step.pts_ticks > target_ticks {
                            // Past the target on the seek stream: the
                            // scan's answer is already final.
                            break;
                        }
```
What is wrong: the linear scan treats *every* chunk of the seek stream as a landing candidate with no keyframe discrimination — correct only while the v0 codec filter keeps MJPEG+PCM (all-keyframe grammars, documented at 566-569). The assumption is implicit and untested: widening the filter (e.g. an intercodec) silently turns no-index seeks into non-keyframe landings. A comment is not an invariant. Confidence: MEDIUM — Verification: read.

F-007 [S4]
Where: crates/demux/src/avi/index.rs:536-556 and 249-262
Code:
```
    for entry in &table.entries {
        if entry.stream != stream {
            continue;
        }
        if entry.pts_ticks > target_ticks {
            break;
        }
```
What is wrong: `search_index` is a linear scan from entry 0 and `replay_ladders` re-advances every entry before the landing on each re-anchor — O(n) per seek for up to 524,288 entries (both are cheap arithmetic, ~ms worst case, but ffmpeg's `av_index_search_timestamp` is a binary search over the same data; a sorted table with monotone per-stream pts admits one). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Ladders::new | 169-192 | REAL | — | lane-per-ordinal, degenerate base → lane-less skip (no panic) |
| Ladders::has_lane | 196-198 | REAL | — | O(1) gate |
| Ladders::advance | 203-215 | REAL | — | saturating tick add; unaligned PCM flagged |
| Ladders::ticks_snapshot | 219-224 | REAL | — | |
| Ladders::with_ticks | 230-240 | REAL | — | length-guarded restore |
| replay_ladders | 249-262 | REAL | S4 | O(n) per re-anchor (F-007) |
| parse_idx1 | 298-479 | REAL | S2 | over-cap fatal (F-001); clamp-before-cap, block-batched reads, coalesced damage all correct |
| probe_offset_base | 489-528 | REAL | S4 | 0-sentinel ambiguity (F-004); fourcc-verified probing works (tested) |
| search_index | 536-556 | REAL | S4 | linear scan (F-007); early-break sound (per-stream pts monotone by construction) |
| scan_movi | 575-593 | REAL | — | thin wrapper |
| scan_movi_with | 633-654 | REAL | — | position restored on every path (tested) |
| scan_movi_inner | 658-752 | REAL | S3/S4 | budget cap (F-002); keyframe-blind landing (F-006); rec-list bound-clamp parity with walk; no infinite loop (pos strictly advances) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| out_of_order_idx1_parses_to_the_file_ordered_table | YES | exact (pos, pts) pairs + landing + damage byte count |
| search_index_finds_last_keyframe_at_or_below | YES | keyframe skip, past-target break, per-stream isolation, before-first None |
| tests.rs idx1 family (clamp/absolute/coalesce/keyframe-truth/duplicates) | YES | exact seek landings and damage reasons asserted |
| adversarial_gaps idx1 suites (shuffled/reversed/duplicate/mid-table-reversed) | YES | walk+seek equivalence vs canonical file |
| dribbled_no_index_scan_seeks_and_lands_within_tolerance | YES | no-index seek path |
| NOTHING tests `index_too_large` | — | zero coverage of the over-cap refusal (F-001) — my probe is the first execution of that path |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A truncated trailing entry is dropped, never fatal" | index.rs:17-19 | REAL | truncation → partial-entry damage note (423-425); tests |
| "A lying declared size is clamped to the bytes that actually exist before any cap is applied" | index.rs:20-21 | REAL | 318-325; test lying_idx1_size_clamps |
| "Entries pointing outside the movi data region are dropped" | index.rs:22-23 | PARTIAL | dropped, yes — but validated against *declared* end, not the effective end the walk uses (F-003) |
| "coalesced to one damage event per reason" | index.rs:23-27 | REAL | 462-473; test idx1_damage_coalesces |
| "the dwOffset identifies its chunk; pts is a property of FILE order" | index.rs:44-52 | REAL | sort at 431 + ladder pass; out-of-order tests |
| "Relative wins ties (spec default)" | index.rs:67 | REAL | probe order 519-527 |
| (implicit) the index is optional and never load-bearing | index.rs:9-13 | FALSE at the cap | over-cap index IS load-bearing: it kills the open (F-001, executed) |

## 7. Dead / duplicated / unreachable code
- Double sort (F-005): mod.rs:1205 re-sorts what parse_idx1:431 already sorted.
- `AdifInfo::descriptor`-style duplication: none in this file; the ladder math (`ticks_for_sample_size`) is shared with the walk — single source, good.

## 8. Dependencies & flags
- Depends on `super::header::{DamageLog, StreamDesc, ticks_for_sample_size}` and `super::riff` — intra-crate. No feature gates. `#[allow(clippy::too_many_arguments)]` ×2 (parse_idx1 8 args, scan_movi_inner 7) — a parameter-object refactor is owed but the lint is annotated, not hidden.

## 9. Open questions for the next agent
- F-001 degrade decision: confirm with the engine owner whether `index-too-large` should demote to "no index" (the noidx0 path) — the current fatal is architecturally inconsistent with the module's own salvage doctrine.
- F-003: if a fixture with a lying movi size + idx1 entries in the phantom zone exists in the corpus, does its seek land or truncate? Worth a fixture in adversarial_gaps.
- Cross-reference for the ADTS audit (W04-A06 F-001): avi/mod.rs:819-849 `scan_for_landing` explicitly re-windows the 8-byte straddle tail ("no candidate position is ever skipped") — the AVI module solved the exact window-boundary hole the ADTS resync still has.
