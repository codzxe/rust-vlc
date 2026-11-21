# AUDIT: crates/video-engine/src/scheduler.rs (part3, lines 1141–1711 of 1711)
> Chunk scope: part3 = lines 1141–1711 — the tail of the in-file `#[cfg(test)]
> mod tests` (declared at 1086, part2's range). One test straddles the boundary:
> `early_frames_are_held_until_their_slot` (1133–1157) starts 8 lines inside part2
> (571–1140, W03-A02) and completes inside part3; it is audited here in full because
> its body is majority-in-range, and A02 is directed to it in §9. Parts 1–2
> (production code, lines 1–1140) are A02/A01's; this audit read them as context for
> mutation reasoning but files no findings against them. File name carries `.part3`.

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-21 |
| Lines / functions | 571 (of 1711) / 18 test fns (17 fully in-range + 1 straddling) |
| File verdict | REAL (test chunk: every test is behavioral, executed green in baseline, mutation-resistant per §5) |
| Reachable from shipped entry point? | YES for the code under test — `PresentationScheduler::new` constructed at `apps/cli/src/pipeline_host.rs:1324`, `scheduler.tick()` driven at `pipeline_host.rs:769` (the `lumen` binary's video limb); the tests themselves run under `cargo test --workspace` (baseline evidence below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Read of full file (1–1711) incl. production code for mutation reasoning; `sed -n` quote verification (1180-1188, 1237-1241, 1591-1598); read `src/test_support.rs` (VirtualVideoClock = real `MediaClock` over a settable monotonic source — not a clock mock); `rg` for `PresentationScheduler` consumers (pipeline_host.rs:1324, :769); baseline cites: `orchrun-test-workspace-nofailfast.log.txt:5915-5963` (video_engine lib: 48 tests, all 18 in-range tests `ok`), `:5969-5993` (tests/scheduling.rs: 21 ok incl. the boundary pair), `:5996-6006` (tests/sync_scenarios.rs: 5 ok), `:6007-6012` (zero_alloc_scheduler: 1 ok); read of `tests/scheduling.rs:272-296` (boundary tests); `lib.rs:181-182` (`#[cfg(test)] mod test_support` — never ships). |

## 1. One-paragraph verdict
This chunk is the in-file test module of the presentation scheduler, and it is the strongest test code audited so far in this wave: 18 behavioral tests driving the real `PresentationScheduler` over a real `MediaClock` fed by a settable monotonic source (no mock of the subject, no mock of the clock trait — `VirtualVideoClock` wraps `MediaClock::system_monotonic`, test_support.rs:37-41), every test asserting specific observable outcomes (presented PTS, drop trails, counters, next_due, held/repeat/background actions), and all 18 executed green in the no-fail-fast baseline (log :5915-5963). I reasoned through one concrete mutation per production behavior the tests touch — selection largest-due, in-order presentation under shuffled arrival, repeat budget, generation purge + repeat invalidation, missing-PTS inference and hold, window bound with frame handed back, unrepresentable-PTS refusal, drop-trail truncation, and the full M10 pre-roll hold trilogy — and found none that would survive this suite; the one boundary property not pinned in-file (strict `<` at the late threshold) is pinned by the integration suite `tests/scheduling.rs::ac_boundary_pts_exactly_on_the_late_threshold_presents` (:272-277), also green in baseline. Zero findings; §5 documents exactly what was checked and how.

## 2. What actually works (verified)
— All 18 in-range tests executed `ok` in baseline `orchrun-test-workspace-nofailfast.log.txt:5915-5963` (video_engine lib, 48 passed / 0 failed): `scheduler::tests::{early_frames_are_held_until_their_slot, late_frames_are_dropped_counted_never_presented, the_largest_due_frame_wins_and_supersedes, out_of_order_arrivals_present_in_pts_order, repeat_budget_then_background, generation_change_purges_the_window_and_kills_repeats, missing_pts_is_inferred_from_the_previous_sequence_position, uninferable_missing_pts_is_held_never_presented_never_lost_silently, window_bound_is_enforced_with_the_frame_handed_back, unrepresentable_pts_is_unschedulable_with_the_error_verbatim, stream_base_pts_are_normalized_before_scheduling, drop_trail_records_to_capacity_then_counts_only, drop_trail_debug_shows_only_retained_state, late_storm_beyond_the_trail_cap_keeps_exact_counts, superseded_storm_beyond_the_trail_cap_keeps_exact_counts, preroll_holds_are_not_judged_while_the_clock_lags_the_generation, preroll_holds_present_in_pts_order_once_the_generation_publishes, held_preroll_frames_are_discarded_when_a_later_generation_publishes}` — every name in my range appears with `ok`.
— The production code under test is shipped and exercised: `apps/cli/src/pipeline_host.rs:1324` constructs `PresentationScheduler::new(cfg, clock)` and `:769` runs `self.scheduler.tick()` inside the `lumen` video limb; the AVI first-vertical smoke (`audit/runs/baseline/avi-firstvertical-open-session.txt`, `v_frames=60 v_decoded=60`) passes through this path.
— The D-1 zero-allocation claim on the tick path is executed by a separate suite: `zero_alloc_scheduler.rs::dropping_ticks_allocate_nothing_after_warm_up` → `ok` (log :6007-6012).
— The clock double is the real clock: `test_support.rs:37-41` builds `MediaClock::system_monotonic(source, …)` over `FakeMonotonic(AtomicU64)` — `set_ns` is `fetch_max` (monotonic-safe, test_support.rs:62-64). No `PlaybackClock` impl is faked anywhere in the crate's tests.

## 3. Findings
None in range. Zero-finding justification (what was checked, how):
1. **Fakeness scan:** no `todo!`/`unimplemented!`/constant-assert/`assert!(true)`; every `assert!` binds a computed outcome to an expected literal. The 14 `let _ = sched.push(...)` discard sites in range (e.g. 1136, 1192, 1248, 1289, 1643, 1690) were each traced: the discarded `PushOutcome` is indirectly pinned by a later behavioral assert in the same test (a refused push would flip the asserted tick action or `pending_len`), and the two sites where the outcome itself is the scenario assert it directly (1592-1595). No silent-loss hole found.
2. **Boundary honesty:** the in-range arithmetic was hand-recomputed: late bound (1180-1187: now 100 − threshold 10 → frames 0..9 all < 90), due bound (1146: `Some(95)` = 100 − lead 5), largest-due selection (1211), shuffle order (1237-1241: `[10,12,13,15]` with exactly 1 supersede), repeat budget (1263-1264: 2/3), purge counts (1293: 3), inference (1320: 10+10=20), hold-replacement accounting (1364: 1 dropped), window handed-back frame (1381: pts 2000 verbatim), storm truncation (1509-1516: 40 recorded / 16 retained / onset 0..15), preroll trilogy (1600-1628: 16 ticks × {Background, no late, no superseded, no stale, next_due None, pending 4}; 1660-1671: [100,120,140] in PTS order; 1700-1709: 3 stale, 0 presented). All recomputations match the asserted literals.
3. **Mock-of-subject scan:** the only doubles are `VirtualVideoClock` (real MediaClock + fake time source — the documented injection seam, doc 03 §8) and synthetic `VideoFrame` fixtures (test_support.rs:94-119). The scheduler itself is never reimplemented or stubbed.
4. **Mutation resistance:** one mutation reasoned per behavior — see §5. All caught either in-file or by an integration test that ran green in baseline.

## 4. Per-function table (per-test; production fns are parts 1–2's)
| Test | Lines | Verdict | Severity | Notes (what breaks it / mutation caught by) |
|---|---|---|---|---|
| `early_frames_are_held_until_their_slot` † | 1133-1157 | REAL | — | Straddles part2 boundary (starts 1133). Pins: pre-slot Background, `next_due = pts − lead`, held-not-dropped, due-exactly-at-bound presents. Breaks if `due_at`/lead off-by-one. |
| `late_frames_are_dropped_counted_never_presented` | 1160-1200 | REAL | — | 10/10 late drops + no late present + no resurrection. Breaks if purge skips or presents late. On-bound (`pts == now−threshold`) NOT in-file — pinned by tests/scheduling.rs:272-283 (baseline :5976-5977 ok). |
| `the_largest_due_frame_wins_and_supersedes` | 1203-1220 | REAL | — | Present(3) over 0; superseded trail `[0]`. Breaks if selection is FIFO-first-due. |
| `out_of_order_arrivals_present_in_pts_order` | 1223-1242 | REAL | — | Shuffle [12,10,11,15,13] → presented [10,12,13,15], 1 superseded. Breaks `insert_sorted` or in-order rule. |
| `repeat_budget_then_background` | 1245-1272 | REAL | — | Exactly 2 RepeatLast then 3 Background; fresh present resets budget. Breaks repeat-budget off-by-one or reset-on-present. |
| `generation_change_purges_the_window_and_kills_repeats` | 1275-1309 | REAL | — | 3 stale purged, old `last` unrepeatable (Background), boundary Stale{current}, new-gen flows. Breaks step-1 purge or repeat invalidation. |
| `missing_pts_is_inferred_from_the_previous_sequence_position` | 1312-1335 | REAL | — | Inferred at last+duration; frame's `Stamp::Missing` untouched (1328-1331). Breaks inference base/duration. |
| `uninferable_missing_pts_is_held_never_presented_never_lost_silently` | 1338-1365 | REAL | — | Held without frame_duration; second missing replaces first, counted. Breaks the hold contract. |
| `window_bound_is_enforced_with_the_frame_handed_back` | 1368-1396 | REAL | — | 9th push → WindowFull{capacity:8} with pts-2000 frame returned verbatim; drain-then-retry works. Breaks the hand-back contract. |
| `unrepresentable_pts_is_unschedulable_with_the_error_verbatim` | 1399-1412 | REAL | — | 1/1-base i64::MAX ticks → `TimeError::Overflow` verbatim, counted, window empty. Breaks checked conversion. |
| `stream_base_pts_are_normalized_before_scheduling` | 1415-1426 | REAL | — | 9000 ticks @1/90000 → presents at 1e8 ns. Breaks base normalization. |
| `drop_trail_records_to_capacity_then_counts_only` | 1431-1467 | REAL | — | 16 retained / 17 recorded / 1 truncated; equality = count+prefix. Breaks DropTrail cap or eq. |
| `drop_trail_debug_shows_only_retained_state` | 1470-1485 | REAL | — | Debug shows `recorded: 19`, retained ticks, not truncated ones. Breaks the custom Debug. |
| `late_storm_beyond_the_trail_cap_keeps_exact_counts` | 1488-1528 | REAL | — | 40-frame storm: recorded 40, len 16, onset 0..15, stats 40, window empty; second storm → 80. Breaks truncation or accounting. |
| `superseded_storm_beyond_the_trail_cap_keeps_exact_counts` | 1531-1571 | REAL | — | Never-policy: 39 superseded + Present(39); same bound. Breaks structural-drop accounting under Never. |
| `preroll_holds_are_not_judged_while_the_clock_lags_the_generation` | 1580-1629 | REAL | — | M10 core: 16 aggressive ticks at frozen old position — no present, no late/superseded/stale, `next_due None`, pending 4 (incl. blocked old-gen 502). Breaks any pre-roll exemption. |
| `preroll_holds_present_in_pts_order_once_the_generation_publishes` | 1632-1676 | REAL | — | Pre-terminal: due-adjacent landing stays held; post-publish: [100,120,140] in PTS order, 0 drops. Breaks the release path. |
| `held_preroll_frames_are_discarded_when_a_later_generation_publishes` | 1679-1710 | REAL | — | Failed seek: 3 held frames die with their generation (stale 3, presented 0). Breaks the wholesale-discard rule. |

† boundary-straddling test, audited here for its body; flagged to A02 in §9.

## 5. Test honesty — mutation analysis (reasoned, not executed)
| Mutation of production code (parts 1–2) | Caught by | Verdict |
|---|---|---|
| `is_late`: `<` → `<=` (on-bound late) | `tests/scheduling.rs::ac_boundary_pts_exactly_on_the_late_threshold_presents` (:272-277, pts 990 @ now 1000, threshold 10 → must Present) — ran ok, baseline :5976. In-file suite alone would NOT catch it. | CAUGHT (cross-file) |
| Selection takes first-due instead of largest-due | `the_largest_due_frame_wins_and_supersedes` (1211: expects 3) | CAUGHT |
| `insert_sorted` appends instead of sorted insert | `out_of_order_arrivals_present_in_pts_order` (1239: expects [10,12,13,15]) | CAUGHT |
| Repeat budget `<=` → `<` (one extra repeat) | `repeat_budget_then_background` (1269-1271: 4th action must be Background) | CAUGHT |
| Step-1 purge forgets `self.last = None` | `generation_change_purges...` (1297: expects Background, not RepeatLast of old gen) | CAUGHT |
| Inference uses window-front instead of tail/last | `missing_pts_is_inferred...` (1320: expects 20 = last-presented 10 + duration 10, window empty) | CAUGHT |
| WindowFull drops the frame instead of handing back | `window_bound_is_enforced...` (1381: asserts handed-back pts == 2000) | CAUGHT |
| DropTrail grows past cap under storm | `late_storm...`/`superseded_storm...` (1510/1556-1558: len == MAX_DROP_TRAIL) | CAUGHT |
| Pre-roll frames judged by frozen position | `preroll_holds_are_not_judged...` (1607-1611: late must be empty — a backward landing at pts 100 vs now 500 would purge without the exemption) | CAUGHT |
| Held pre-roll presents before terminal | `preroll_holds_present...` (1647-1655: 8 ticks at the due-adjacent slot, all Background) | CAUGHT |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every acceptance test runs on injected virtual time with zero sleeps (doc 03 §8)" | lib.rs module docs | REAL | All 18 tests drive `VirtualVideoClock::set_ns`; no sleep/timer anywhere in range (rg over chunk). |
| "Zero heap traffic (D-1/D-8) … measured by `tests/zero_alloc_scheduler.rs`" | scheduler.rs tick docs 693-694 (part2's range, claim verified by this chunk's context) | REAL | Baseline :6007-6012 — `dropping_ticks_allocate_nothing_after_warm_up` ok. |
| "skips are observable events, never silent" (doc 01 §1.3 rule 9) | lib.rs docs | REAL | `TickDrops` trails asserted in 6 of the 18 tests (late/superseded/stale counts + retained PTS). |
| M10 pre-roll hold module docs (52-71) | scheduler.rs module docs (part1's range) | REAL | The preroll trilogy (1580-1710) executes exactly the documented three outcomes; ran ok in baseline :5955-5957. |

## 7. Dead / duplicated / unreachable code
None in range. `mod test_support` is `#[cfg(test)]`-gated (lib.rs:181-182) — test fixtures never ship.

## 8. Dependencies & flags
Tests import `testkit::Must` (dev-dep, per crate Cargo.toml) and `crate::test_support`; no feature gates. The suite compiled and ran in the baseline workspace run (log :5915).

## 9. Open questions for the next agent
- **For W03-A02 (part1) / W03-A01 (part2):** production-code observations from my mutation reading that belong to their ranges: (a) step-1 `retain(|p| p.frame.generation == current)` (730) would also drop *future*-generation pre-roll frames if a host ever published generation N+1 while N+2 frames are resident — unreachable under the engine's mint/publish discipline, but worth their confirmation; (b) the `early_frames_are_held_until_their_slot` test (1133-1157) is majority part3 but its header/first-asserts sit in part2 — A02 should not re-audit its body, only its first 8 lines.
- The failing baseline integration test `journey.rs::journey_seek` ("decode accounting keeps the pre-seek deliveries — observed 90, expected 154", log :2515-2531, footer item 4) exercises the seek flow this scheduler's pre-roll hold participates in; the in-file preroll tests all pass, so the defect is more likely in the engine-side seek accounting (session.rs part4 / pipeline) — flagged to that owner via the log.
