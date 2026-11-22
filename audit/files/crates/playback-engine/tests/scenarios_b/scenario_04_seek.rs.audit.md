# AUDIT: crates/playback-engine/tests/scenarios_b/scenario_04_seek.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 153 / 1 test + 3 helpers |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | N/A — scenario test (doc 10 §6.4 row 4) over the command layer (shared ScenarioHost). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | `head -40` + inventory (exit 0). No cargo; run UNVERIFIED (W00: 1751/2/25). |

## 1. One-paragraph verdict

Three seeks through all three wire forms (absolute into the second half, relative hop from the landing, percent-of-duration), each verified on four axes: class-A accept receipt, exactly one `timing.seek-completed` per seek, completion position within the parent fixture's recorded 100 ms tolerance, and the post-completion frame ring forward-only from the landing (a stale frame = a stamp behind the landing). Media still ends honestly after all three. The tolerance-based position check is weaker than the engine-level seek suite's exact-anchor asserts (my seek_roundtrip audit) but matches the scenario row's contract ("within tolerance", the parent's expectation TOML). Honest layering.

## 2. What actually works (verified)

- `seek_and_assert` (29-77): receipt-shape match, terminal counted via `count_events` deltas, tolerance compare, ring-forward-only scan — read-verified.
- The stale-frame oracle is structural (stamps behind the landing point) rather than generation-based — an independent second angle on staleness vs the property suite's generation checks.

## 3. Findings

### F-001 [S4] Tolerance (100 ms) instead of exact landing
Where: scenario_04_seek.rs:24 — `const TOLERANCE_US: i64 = 100_000;` documented as "the parent's recorded seek tolerance (its expectation TOML)". Correct for the scenario row's wording; weaker than the truth the engine actually delivers (exact anchor — seek_roundtrip). A regression that lands within 99 ms of the target would pass here and fail there. Recorded as the intended layering, not a defect.

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `seek_and_assert` | 29-77 | REAL | four-axis verify per seek |
| `seek_completed_position`/`name_target_us` | 79-97 | REAL | helpers |
| `seek_lands_three_targets_within_tolerance_without_stale_frames` | 99-153 | REAL | three forms, ends honestly |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| seek_lands_three_targets… | YES (reasoned) | wrong landing >100 ms, missing/duplicate terminal, or a behind-landing stamp each fail with the seek's name in the message |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| doc 10 §6.4 row 4: `Seeked{actual}` within tolerance, no stale frames after completion | header | REAL-UNPROVEN | asserted as specified; run UNVERIFIED |

## 7-9. Dead code / Dependencies / Open questions

None beyond the shared-host dependency noted in scenario_03's audit.
