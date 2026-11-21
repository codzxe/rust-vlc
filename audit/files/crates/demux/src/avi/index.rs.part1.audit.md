# AUDIT: crates/demux/src/avi/index.rs#part1 (lines 1-464)
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Chunk boundaries | part1 = lines 1-464 (this audit). `parse_idx1` (298-479) crosses the boundary: its damage-coalescing tail (465-479: `dropped_unknown_stream`/`dropped_not_packet`/`out_of_order` notes + return) is audited here for logic completeness but its lines belong to A02's part2 (465-930: `probe_offset_base`, `search_index`, linear-scan fallback, tests mod from :755). |
| Lines / functions | 464 / 15 (2 consts, 6 structs, `Ladders` impl ×5 methods, `replay_ladders`, `parse_idx1` [most of it]; no tests in this range — the tests mod starts at :755, part2) |
| File verdict (part1) | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → AVI container host → top-level walk `avi/mod.rs:1193` calls `parse_idx1`; `:1261` `Ladders::new`; seek path `:1070` `search_index`; re-anchor `:978-979` `replay_ladders`/`with_ticks` — all sed/rg-verified) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | full Read :1-559; `rg -n "ordinal\|classify_movi_chunk"` on header.rs/riff.rs (ordinal = sequential strl counter :472-494; fourcc digits 0-99 :127-137); `rg -n "MAX_STREAMS\|too many streams"` over avi/ (NO stream-count cap exists); `sed -n '470,500p'` header.rs (strl walk); sed of riff.rs `read_prefix_into` (short read = EOF only — loop-until-full contract) and `classify_movi_chunk`; `rg -n "index_too_large" error.rs`; baseline log greps (see §2) |

## 1. One-paragraph verdict
The best-engineered file audited so far in this wave: the idx1 parse is clamped-before-capped (a lying declared size meets the bytes that exist, damage-counted, never a fatal over-cap abort), block-batched (4 KiB reused buffer, no per-entry reads), bounded by a 524,288-entry cap, order-corrected by a stable file-order sort before the ladder timing pass, with coalesced per-reason damage accounting and a self-validating offset-base probe — and nearly every module-doc claim has a passing executed test in the baseline. The one serious hole is not in the idx1 logic at all: `Ladders::new` allocates one lane per `strl` ordinal, and the ordinal count is file-controlled with **no cap anywhere in the avi module** — every other file-controlled count in this very file is capped (`MAX_IDX1_ENTRIES`), making the missing stream ceiling an inconsistency with real memory-amplification consequences on crafted hdrls. Two minor findings round it out (a suppressed clippy lint over a 9-parameter signature, and an unsized `raw` Vec next to a comment celebrating "one allocation").

## 2. What actually works (verified)
— Executed, passing, in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt`:
  `matrix_avi_trunc4_plays_to_clean_ended_full_payout ... ok` (:2559 — the doc's "trunc4 loses exactly its last index entry and still demuxes fully"); `matrix_avi_noidx0_clean_ended_full_payout ... ok` (:2554 — index-less walk); `matrix_avi_absidx1_clean_ended_full_payout ... ok` (:2546) and `avi::tests::absolute_idx1_offsets_are_autodetected_and_seek ... ok` (:1617 — the offset-base probe); `avi::tests::lying_idx1_size_clamps_to_available_bytes ... ok` (:1637); `avi::tests::idx1_damage_coalesces_per_reason ... ok` (:1633); `avi::tests::idx1_keyframe_flag_is_the_truth_when_present ... ok` (:1634); `avi::index::tests::out_of_order_idx1_parses_to_the_file_ordered_table ... ok` (:1594); `avi::tests::seek_lands_on_last_keyframe_at_or_below_target ... ok` (:1659); `avi::index::tests::search_index_finds_last_keyframe_at_or_below ... ok` (:1595).
— Clamp-before-cap arithmetic re-derived: `payload = min(declared, available)` (:319) → `div_ceil(16)` (:326) → cap check (:327) — a 0xFFFFFFF0 declared size over 40 real bytes yields payload 40, 2 entries, one damage note, no fatal error. Correct per doc :314-317.
— Loop safety re-derived: `payload ≤ 0xFFFFFFFF` (min of a u32-derived value — the `(payload - offset) as usize` cast at :362 cannot truncate on any target); `offset += got` with `got ≥ 16` guaranteed progress; `got < block_want` breaks only on true EOF (`read_prefix_into` loops until full or `read_into` returns 0 — riff.rs sed-verified); `saturating_add` on every position advance (:390-391, :393).
— `filtered_ordinals.get(stream) == Some(&true)` silent-skip vs unknown-stream damage split (:445-458) matches the module doc's W5-14 rule exactly.

## 3. Findings
### F-001 [S2]
Where: crates/demux/src/avi/index.rs:170-171 (with the uncapped producer at crates/demux/src/avi/header.rs:472-494)
Code:
```rust
        let max_ordinal = streams.iter().map(|s| s.ordinal).max().unwrap_or(0);
        let mut lanes = vec![None; max_ordinal as usize + 1];
```
What is wrong: `ordinal` is incremented once per `LIST strl` in the header walk (header.rs:472 `let mut ordinal: u32 = 0;` … :494 `ordinal = ordinal.saturating_add(1);`) with **no ceiling** — `rg "MAX_STREAMS|too many streams"` over the whole avi module returns nothing. A minimal ordinal-incrementing strl is ~12-16 file bytes (`LIST`+size+`strl` with size=4, no children — `peek_form` recognizes it, `parse_strl` returns None, ordinal still advances). `Ladders::new` then allocates `max_ordinal+1` × `sizeof(Option<LaneState>)` (LaneState = u32 + TimeBase{u32,u32} + i64 ≈ 24 B, Option ≈ 32 B) — roughly **2-3 bytes of RAM per file byte** of a crafted hdrl — plus the parallel `filtered_ordinals: Vec<bool>` in header.rs. A 500 MB crafted AVI whose hdrl is stuffed with empty strl lists allocates >1 GB in a *streaming* demuxer that holds no other per-file state.
Why it matters: this is exactly the "unbounded allocation from file-controlled counts" checklist item. The file's own discipline proves the authors know the pattern: `MAX_IDX1_ENTRIES` (:84), `MAX_COMM_BYTES` (aiff/chunks.rs:37), `MAX_STRN_BYTES` (header.rs:342), `MAX_PCM_CHANNELS` (header.rs:349) cap every OTHER file-controlled count — the strl/ordinal count is the one hole, and it lands in this line. OOM/abort on malformed input, no typed refusal. (VLC-class demuxers cap track counts.)
What correct looks like: a `MAX_STRL_LISTS` ceiling in the header walk (typed `demux.avi.too-many-streams` damage/refusal past it — e.g. 4096, far above any real AVI), which bounds `lanes`, `filtered_ordinals`, `streams` and the `streams.iter().find` scans in mod.rs:392/869/1043 simultaneously.
Confidence: HIGH on the missing cap (exhaustive rg) and the allocation math; UNVERIFIED by execution (no cargo; no fixture with millions of strl lists exists).
Severity note: S2 not S1 because the allocation is proportional (≤ ~3× file size), not unbounded, and requires a deliberately crafted large file.

### F-002 [S4]
Where: crates/demux/src/avi/index.rs:297-298
Code:
```rust
#[allow(clippy::too_many_arguments)]
pub fn parse_idx1(
```
What is wrong: nine positional parameters, four of which (`movi_form_pos`, `movi_data_start`, `movi_data_end`, `bound`) are one concept — the movi geometry — and a fifth (`streams` + `filtered_ordinals`) always travel together. The lint is suppressed rather than the signature fixed.
Why it matters: `#[allow]` is the checklist's "hiding a real lint" smell; every future parameter (e.g. a damage budget) joins an already-unreadable call site (mod.rs:1193).
What correct looks like: `struct Idx1Geometry { movi_form_pos, movi_data_start, movi_data_end, bound }` (or a `MovlRegion` type shared with the walk), dropping the allow.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/demux/src/avi/index.rs:333-335 (vs 433-435)
Code:
```rust
    let mut raw: Vec<RawIdxEntry> = Vec::new();
```
with the contrast at 433-435:
```rust
    let mut ladders = Ladders::new(streams);
    // One allocation, sized to the raw table (D-1: the parse is a
    // one-time start cost, never per-frame; the entry cap bounds it).
    let mut entries: Vec<IndexEntry> = Vec::with_capacity(raw.len());
```
What is wrong: `entries` is pre-sized and the comment celebrates "One allocation" (D-1), but `raw` — the same-sized table one step earlier — is grown by doubling: at the 524,288-entry cap that is ~20 reallocations and ~2× the table copied (≈ 25 MB churn). The comment describes the second Vec, not the first.
Why it matters: an inconsistency between the file's own allocation discipline and its comments; harmless at fixture sizes, measurable at the cap.
What correct looks like: `Vec::with_capacity(usize::try_from(entry_count).unwrap_or(0))` after the cap check (entry_count already bounds raw exactly).
Confidence: HIGH — Verification: read (entry_count bound re-derived).

## 4. Per-function table (part1 range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const `AVIIF_KEYFRAME` | 79 | REAL | — | 0x10, the spec bit; only bit consumed (test-pinned, baseline :1634). |
| const `MAX_IDX1_ENTRIES` | 81-84 | REAL | — | 524,288 = 8 MiB raw payload; honest `[PROPOSAL]` marker with the 2-hour-math. |
| `IndexEntry` / `IndexTable` / `Landing` | 88-117 | REAL | — | Plain data; absolute positions + per-stream ticks. |
| `LadderStep` | 121-137 | REAL | — | Carries base+sample_size so the walk needs no stream-table scan (D-3); pts = position *before* advance — correct presentation semantics. |
| `LaneState` / `Ladders` | 143-165 | REAL | — | Option-per-ordinal lanes; see F-001 for the count issue. |
| `Ladders::new` | 169-192 | REAL | S2 | F-001. `time_base(0).ok()` leaves no lane on degenerate base → walk's ordinary unknown-stream skip (no panic path — verified :174 bounds-check + Option). |
| `Ladders::has_lane` | 196-198 | REAL | — | O(1); `.get()` OOB-safe for any u32 ordinal. |
| `Ladders::advance` | 203-215 | REAL | — | `ticks.saturating_add(delta_ticks)` — no overflow panic; unaligned flag propagated for caller damage. |
| `Ladders::ticks_snapshot` | 219-224 | REAL | — | None → 0; `with_ticks` only writes Some lanes, so the 0s are inert. |
| `Ladders::with_ticks` | 230-240 | REAL | — | Length-guard mismatch keeps fresh ladders (documented unreachable-within-instance; defensive, no guess). |
| `replay_ladders` | 249-262 | REAL | — | Break at first `chunk_pos >= before_pos` (entries file-ordered — correct); `let _ =` discards None advances = intentional per-stream skip, not a swallowed error. |
| `RawIdxEntry` | 267-272 | REAL | — | Positionally-validated pre-sort record. |
| `parse_idx1` (1-464 portion) | 298-464 | REAL | S4 ×2 | F-002 (allow), F-003 (raw sizing). Logic verified: empty idx1 → None (:310-313); clamp→cap→batch→sort→time pipeline matches module docs; base probe once on first packet entry (:374-389), absolute detection damage-noted (:384-388); outside-movi/out-of-order counters saturating; stable sort (:431) preserves duplicate order. Boundary: coalescing notes + return at :465-479 (A02's lines, logic sound). |

## 5. Test honesty
No tests live in lines 1-464 (tests mod starts :755, part2). Part1's functions are covered end-to-end by avi/tests.rs and the first-vertical recovery matrix — 11 distinct passing baseline tests enumerated in §2, including the exact doc-claim pins (trunc4 salvage, lying-size clamp, absolute-offset autodetect, out-of-order correction, damage coalescing). Mutation reasoning: if the clamp-before-cap order were swapped (cap on declared size), `lying_idx1_size_clamps_to_available_bytes` fails (declared 0xFFFFFFF0 would abort typed); if the sort were unstable/missing, `out_of_order_idx1_parses_to_the_file_ordered_table` fails; if the probe preferred absolute on ties, `absolute_idx1_offsets_are_autodetected_and_seek` and the relative fixtures both cannot pass together. The suite is honest and load-bearing.

## 6. Doc claims touching this file (module docs :1-69)
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A truncated trailing entry is dropped, never fatal" | :17-19 | REAL | Loop + partial-entry damage (:423-425); `matrix_avi_trunc4...` ok (baseline :2559). |
| "A lying declared size is clamped to the bytes that actually exist before any cap is applied" | :20-21 | REAL | :314-329; `lying_idx1_size_clamps_to_available_bytes` ok (:1637). |
| "coalesced to one damage event per reason … 16 bytes per dropped entry" | :23-27 | REAL | :338-344, 460-473; `idx1_damage_coalesces_per_reason` ok (:1633). |
| "Entries … declared but … filtered … are silent-skipped … genuinely-unknown … stay damage" | :30-39 | REAL | :445-458; W5-14 rationale consistent with header.rs:412-423 doc. |
| "orders them by chunk_pos … stable … duplicates keep their relative order" | :41-57 | REAL | :431 sort_by_key; `out_of_order_idx1...` ok (:1594). |
| "a base is accepted when the candidate position sits inside the movi data region and the chunk id actually stored there matches" | :63-67 | REAL | probe_offset_base :503-527 (read; body in part2); `absolute_idx1_offsets_are_autodetected_and_seek` ok (:1617) + `matrix_avi_absidx1` ok (:2546). |
| "index and walk agree on time by construction" | :97-99 (IndexEntry doc) | REAL structurally | Both advance through the same `Ladders` (mod.rs walk + :432 ladder pass). Standing counter-evidence at the integration level: `journey_seek` FAILED in baseline (orchrun-test-journey-seek.log.txt: "the decode accounting keeps the pre-seek deliveries — observed 90, expected 154", scenarios.rs:529-532) — but that check asserts *engine decode-accounting continuity across seek* (frames_accepted), not index math; the seek itself landed and played. Attribution to this file: UNPROVEN; flagged for the playback-engine wave. |
| "~524k entries; ~8k-tick event storm" | :24-26 | UNVERIFIED | Internal budget arithmetic; no committed run isolates it. |

## 7. Dead / duplicated / unreachable code
None in part1. Every function has a caller (verified via mod.rs call sites :978-979, :1070, :1193, :1261). The `with_ticks` length-guard is documented-unreachable defensive code, correctly inert.

## 8. Dependencies & flags
`media-types` (EngineError, TimeBase), `super::header` (DamageLog, StreamDesc, ticks_for_sample_size), `super::riff` (FourCC, SourceReader, classify_movi_chunk, MoviChunkKind), `crate::error`. No feature gates. No unsafe, no unwrap/expect/panic in part1 (all Option/Result-typed; saturating arithmetic throughout).

## 9. Open questions for the next agent (A02 owns part2 = :465-930)
- F-001's cap belongs in header.rs's strl walk — the header.rs auditor should file/merge it there; `mod.rs:392/869/1043` `streams.iter().find` scans are O(strl-count) per call and amplify the same missing cap.
- `journey_seek` (baseline failing test #4): determine whether the 90-vs-154 frames accounting is engine-side (session/decode-ahead accounting across seek) or a short re-anchor (replay_ladders under-counting pre-landing advances on this fixture). The seek lane's only RED test — do not let it hide behind the MP4 headline.
- `probe_offset_base`'s `Some(0)`-means-absolute sentinel depends on movi_form_pos ≠ 0 always (true: RIFF header precedes) — worth a debug_assert when A02 audits :489-528.
