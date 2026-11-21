# AUDIT: crates/demux/src/avi/tests.rs#part5 (lines 2501–3128 of 3128)
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-11-21 |
| Lines / functions | 628 / 18 items (15 tests + 3 helpers + tail of a part4 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A for test code; the subject under test (`AviDemuxer`) is the shipped first-vertical lane (baseline INDEX: `avi-firstvertical-open-session.txt` full lifecycle; `mp4-smoke-bench-avi-mjpeg-pcm.txt` 60/60 frames decoded) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:3 |
| Commands run | grep for every `#[test]` ≥ line 2501; for-loop grep of `orchrun-test-workspace-nofailfast.log.txt` → all 15 part5 tests `ok` (1 hit each); sed -n on all quoted ranges; read of avi/mod.rs locate/seek/reanchor (947-1090), index.rs search_index (532-555), error.rs tests (225-254), time.rs convert doc (160) to validate the semantics the tests pin |

Chunk boundary note: parts 1–4 = lines 1–2500 (A10/A08/A07/A06). My range opens inside
`seek_does_not_move_the_read_position` (2428–2525, owned by A06/part4 — I read its tail
2501-2525 for context only). First item wholly mine: `lying_hdrl_size_does_not_swallow_the_movi`
(2528).

## 1. One-paragraph verdict
This is the strongest kind of test file in the repo so far: every test builds real AVI byte
streams via the shared builder, mutates them surgically (lying hdrl size at bytes[16..20],
garbage past the RIFF extent, duplicate movi/idx1, non-keyframe idx1 flags), and asserts
hardcoded absolute expectations (landing ticks, resumed packet tuples, exact damage totals).
All 15 tests executed and passed in the nofailfast baseline. The reanchor suite pins the
properties that matter — keyframe-safe landing, ladder replay to pre-landing state, D4
position preservation on refusal, EOS revival, pending-event invalidation, SlowLinear
budget — with mutation-sensitive assertions. The residue is hygiene: three blocks of dead
scaffolding, one test that duplicates an exhaustive error-table test while its name claims
"every error", and SUT-as-oracle "truth" helpers whose absolute anchor lives in part1-4.

## 2. What actually works (verified)
— All 15 part5 tests `ok` in audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt
(verified individually by grep, one hit each: lying_hdrl_size…, riff_extent_clamps…,
duplicate_movi…, eos_is_idempotent…, error_module_constructors…, reanchor_moves…,
reanchor_on_interleaved…, reanchor_without_idx1…, reanchor_failure…_d4,
reanchor_from_eof…, reanchor_clears_pending…, reanchor_at_end…, reanchor_landing_agrees…,
slow_linear_scan…, seekability_reports…).
— Landing-semantics trace: `locate` (avi/mod.rs:1029-1074) converts the target via
`Timestamp::convert` (media-types/src/time.rs:160 — "inexact ratios round to the nearest
tick (ties … away from zero)") then `search_index` (avi/index.rs:532-555, "last keyframe
entry … whose pts ≤ target_ticks"). The tests' arithmetic checks out by hand:
65 ms → 1.95 → 2 (ties/round-half-up), 50 ms → 1.5 → 2, at-end 133.33 ms → tick 4 → last
keyframe 3. The tests pin these absolute values hardcoded — a change to either the rounding
or the at-or-below rule fails them.
— Mutation reasoning (no edits made): flip the `entry.keyframe` filter in search_index →
`reanchor_moves…` lands on tick 1 → assert ticks==2 fails. Break ladder replay (reset audio
ladder on reanchor) → `reanchor_on_interleaved…` resumed a2 tick ≠ truth[5] fails. Stop
clearing the pending queue in reanchor → `reanchor_clears_pending…` sees stale packet b →
len assert fails. Remove the extent clamp → `riff_extent_clamps…` damage_total()==0 fails.
Raise SEEK_SCAN_CAP above 1002 → `slow_linear_scan…` unreachable!("far target…") panics.

## 3. Findings

### F-001 [S4] Dead scaffolding left in three tests
Where: crates/demux/src/avi/tests.rs:2597-2602 (also 2730-2733, 2805)
Code (2597-2602):
```
    let extra_start = bytes.len();
    chunk(&mut bytes, b"LIST", &[]);
    let _ = extra_start;
    // Recompute: a LIST with form 'movi' needs the form inside payload.
    bytes.truncate(extra_start);
```
Code (2732-2733):
```
    let mut truth_indexed = truth;
    truth_indexed.clear();
```
Code (2804-2806):
```
            [video, audio].map(|chunk| {
                let _ = index;
                chunk
```
What is wrong: three blocks of no-op residue: (a) `duplicate_movi…` appends an empty LIST
chunk then truncates it away before building the real duplicate — the append is dead; (b)
`reanchor_moves…` computes `truth` via `from_start_truth(...)` (2721-2729) then discards it
with `let mut truth_indexed = truth; truth_indexed.clear();` — the whole first computation
is dead (the comment at 2730-2731 admits the re-derivation); (c) the interleaved builder's
closure carries `let _ = index;` — a no-op from a refactor.
Why it matters: noise in the exact tests future maintainers must trust; the (b) case burns
a full demux-open-drain for nothing and misleads a reader into thinking the un-indexed
truth matters here.
What correct looks like: delete all three blocks (`let mut truth_indexed = Vec::new();`).
Confidence: HIGH — Verification: read; all three verified verbatim by sed.

### F-002 [S4] `error_module_constructors_stay_demux_staged` duplicates the exhaustive error-table test under an over-promising name
Where: crates/demux/src/avi/tests.rs:2643-2657
Code:
```
fn error_module_constructors_stay_demux_staged() {
    // A cross-check tied to this module's usage sites: every error the
    // walk can mint carries the Demux stage and a stable code.
    let err = error::no_movi();
```
What is wrong: the comment claims "every error the walk can mint" but the body checks two
constructors (`no_movi`, `not_started`) of the fourteen in `crates/demux/src/error.rs` (14
`pub fn`, counted). The exhaustive table already exists — `errors_carry_demux_stage_and_stable_codes`
(error.rs:228-253) pins all 14 constructors with codes and Display shape, and it runs in
the same baseline (`error::tests::errors_carry_demux_stage_and_stable_codes … ok` in the
nofailfast log). This test adds nothing but a misleading name.
Why it matters: a reader auditing error coverage may cite this test as proof for errors it
never checks.
What correct looks like: either extend the body to the walk's actual mint sites or rename
to `two_error_samples_stay_demux_staged`; better, delete it in favor of error.rs's table.
Confidence: HIGH — read both tests.

### F-003 [S4] SUT-as-oracle "truth" helpers; one near-tautological agreement test; one untested seekability branch
Where: crates/demux/src/avi/tests.rs:2671-2692 (helper), 3012-3036 (test), 3100-3128 (test)
Code (2678-2679, the oracle body):
```
    let truth = from_start_truth(
        &[video_stream()],
```
```
    let built = build(streams, movi_chunks, with_idx1);
    let mut avi = demuxer_over(built.bytes);
```
What is wrong: (a) `from_start_truth` derives the expected ladder by running the same
`AviDemuxer` — a systematic stamping bug (wrong base, off-by-one ticks) reproduces itself
into the expectation. The blast radius is contained because part1-4 pins the absolute
ladder hardcoded (`packets_carry_the_avi_ladder_timing`, tests.rs:1100-1157: audio ticks
`vec![0, 2]`, video `index as i64`, durations `33_333_333`/`41_667` ns) — but the helper's
name ("truth") oversells SUT output as ground truth. (b) `reanchor_landing_agrees_with_the_pure_query`
(3013-3036) compares `seek(t)` and `reanchor(t)` on an **indexed** file where both funnel
into the same `locate()` → `search_index` call (avi/mod.rs:949-951 vs 971-973) — it pins
only that the commit path returns the same span, near-tautology for the landing rule
itself. (c) `seekability_reports_the_container_shape` (3100-3128) never exercises the
sticky-error branch (`self.sticky.is_some() → SeekSupport::None`, avi/mod.rs:959-961).
What correct looks like: rename the helper `from_start_ladder` (or hardcode the small
ladders); make the agreement test use the unindexed path (where seek and reanchor
genuinely differ: query restores position, reanchor commits); add a
seekability-after-terminal-error case.
Confidence: HIGH — read helper, test, and both implementation sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (would it fail on a real bug?) |
|---|---|---|---|---|
| (tail of) `seek_does_not_move_the_read_position` | 2501-2525 | REAL (part4's) | — | boundary only; asserts read-position restore via a lock-logged stream |
| `lying_hdrl_size_does_not_swallow_the_movi` | 2528-2543 | REAL | — | patches hdrl LIST size at bytes[16..20] (correct RIFF offset); hardcodes 1 track + payload `b"a"`; YES fails if recognition-is-authority regresses |
| `riff_extent_clamps_the_walk_to_real_bytes` | 2544-2590 | REAL | — | two halves: garbage past extent invisible (damage 0) + shrunk extent → typed Truncated kind; YES |
| `duplicate_movi_and_duplicate_idx1_count_as_damage` | 2591-2622 | REAL | S4 | exact damage 36 pinned; dead scaffolding (F-001) |
| `eos_is_idempotent_and_terminates` | 2623-2641 | REAL | — | 3× EndOfStream after drain; YES |
| `error_module_constructors_stay_demux_staged` | 2642-2658 | REAL | S4 | duplicate+over-promise (F-002) |
| `video_stream_of` (helper) | 2660-2668 | REAL | — | parameterized stream factory |
| `from_start_truth` (helper) | 2671-2692 | REAL | S4 | SUT-as-oracle (F-003a) |
| `reanchor_moves_the_stream_and_packets_continue_from_the_landing` | 2694-2782 | REAL | S4 | hand-built idx1 with frame-1 non-key; landing tick 2 + resumed [(0,2),(0,3)] hardcoded; strongest reanchor test; dead truth computation (F-001b) |
| `drain_through` (helper) | 2783-2795 | REAL | — | exact-count drain |
| `reanchor_on_interleaved_streams_keeps_both_ladders_true` | 2796-2843 | REAL | S4 | ladder replay pinned against from-start ladder + absolute first (0,2); `let _ = index;` no-op (F-001c) |
| `reanchor_without_idx1_scans_and_continues` | 2844-2874 | REAL | — | scan-mode landing tick 2, resumed == [truth[2]]; YES for scan landing |
| `reanchor_failure_leaves_the_demuxer_at_its_position_d4` | 2876-2906 | REAL | — | past-end typed `demux.avi.seek.out-of-range`; D4 continuation == truth[1..]; YES |
| `reanchor_from_eof_revives_the_stream` | 2907-2933 | REAL | — | EOS → backward reanchor → 3 packets again; YES |
| `reanchor_clears_pending_events_stale_packets_never_reappear` | 2934-2983 | REAL | — | JUNK chunk → Recovered event; reanchor drops pending packet b; resumed == [c@tick2]; YES |
| `reanchor_at_end_lands_last_and_past_end_is_typed` | 2984-3011 | REAL | — | at-end lands keyframe 3; past-end typed; YES |
| `reanchor_landing_agrees_with_the_pure_query` | 3012-3036 | REAL | S4 | near-tautology on indexed path (F-003b) |
| `slow_linear_scan_is_bounded_and_typed` | 3038-3098 | REAL | — | 1002 packets vs cap 1000: near target ok, far target `demux.avi.seek.too-slow` + ResourceLimit kind + D4 playable + pure query unbounded (tick 1001) + past-end typed; YES — raises the cap and the test fails |
| `seekability_reports_the_container_shape` | 3100-3128 | REAL | S4 | None/Fast{indexed}/SlowLinear pinned; sticky branch untested (F-003c) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 15 | YES (see per-function table) | hardcoded absolute expectations or exact damage totals; mutation reasoning per test in §2/§4 |
| reanchor_landing_agrees_with_the_pure_query | only for commit-path bugs | both arms call the same locate() on the indexed path (F-003b) |
| reanchor_on_interleaved / without_idx1 / failure_d4 | YES for relative-ladder bugs; NO for absolute-stamp bugs | expectations are SUT-derived truth slices (F-003a), anchored only by part1-4's hardcoded ladder test |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "keyframe-safe position at-or-below the target" (doc 03 §4.3(f)) | index.rs:532-533 | REAL | search_index implements exactly that; tests pin 65 ms→tick 2 via round-to-nearest (time.rs:160) — the round-then-floor compound is pinned by 4 hardcoded landings |
| "D4: demuxer continues from its pre-seek position" on refusal | mod.rs doc + tests 2876-2906, 3038-3098 | REAL | both D4 tests assert original-timestamp continuation after typed refusals (baseline ok) |
| "the pure query is unbounded / reanchor bounded by SlowLinear cap" | test 3038-3098 | REAL | cap refusal + unbounded query both asserted, executed in baseline |
| "AVI first-vertical full lifecycle" | baseline INDEX mp4-smoke contrast | REAL | avi-firstvertical-open-session.txt + bench 60/60 — the demuxer these tests exercise is the one shipped lane |

## 7. Dead / duplicated / unreachable code
See F-001 (three dead blocks) and F-002 (duplicated error-table test). Nothing unreachable:
every test ran in the baseline.

## 8. Dependencies & flags
Uses only crate-internal helpers (build/chunk/patch_size/demuxer_over/drain_packets,
defined part1) and media_types — no feature gates, no unsafe, no I/O outside the in-memory
stream (the position-logging stream in part4's tail).

## 9. Open questions for the next agent
1. `reanchor` on an **unindexed** file whose movi region is huge: `scan_movi_with` walks
   chunk headers — the D-3/D-9 clamp interplay with the scan cap deserves a test where the
   cap is hit mid-resync (current test only hits it on a clean file).
2. Negative-target reanchor (ticks < 0 → out_of_range, mod.rs:1051-1053) has no part5 test
   — check parts 1-4; if absent, that branch is untested.
3. `patch_size` uses wrapping_add — intentional (shrink via negative delta at 2570) but a
   comment there would prevent a "fix" to checked arithmetic.
