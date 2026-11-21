# AUDIT: crates/streaming/src/buffer.rs#part2
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-21 |
| Lines / functions | 357 (472–828 of 828) / 12 items in table (2 helpers + 10 tests) |
| Chunk boundary | part2 = 472–828 = the entire `#[cfg(test)] mod tests` (equal split is 414; the production tail 415–471 — `predict_stall_ns` end, `snapshot`, `obs_fraction` — moves to A01's part1 so the tests module is whole). Flagged in log. |
| File verdict | REAL (library; UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO. `BufferModel::new` appears only in this file's tests (479, 620, 688, 744 — rg). Production imports of the module's types exist (`reconnect.rs:89`, `hls/master.rs:83`, `dash/abr.rs:34`, `dash/sched.rs:337`) but every one of those consumers is itself unwired: `forward_signal` (reconnect.rs:532) has no production caller; reconnect's `RetryFetch::new_checked` is called only from tests (reconnect.rs:974, 1292) and integration tests; `with_retrying_fetch` only from schemes.rs tests (958, 1015); HLS/DASH have zero references in apps/cli (rg). The shipped `lumen` binary never constructs this model. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "BufferModel" apps tests crates --glob '*.rs'` (construction sites = tests only); `rg -n "forward_signal\|with_retrying_fetch\|RetryFetch::new" …` (test-only callers); `rg -n "streaming::hls|hls::" apps/cli/src` (zero); matrix rows read: `docs/feature-matrix.toml:1457-1463` (`network-reconnect-buffering` = not_started all platforms, empty evidence); baseline: orchrun-test-workspace-nofailfast.log.txt streaming run — all 10 `buffer::tests` ok. |

## 1. One-paragraph verdict
The test half of the adaptive-buffer model is the strongest test suite in my assignment: exact signal sequences, exact episode durations (initial fill 2.9 s, longest rebuffer 2.8 s), a pinned prediction-vs-drain ordering (predict at step 13, drain at step 27), and a synthetic 3G trace that exercises fill → outage → recovery end-to-end. All 10 tests pass in baseline and each would fail under a targeted mutation. The problem is what wraps them: nothing outside this file ever constructs a `BufferModel`, the signal consumers in reconnect/HLS/DASH are themselves unreachable from any shipped entry point, and the feature matrix honestly admits it (`network-reconnect-buffering`: not_started, evidence empty). Card 6.D1's policy core is real, verified, and wired to nothing — a completed engine part for a car with no chassis.

## 2. What actually works (verified)
— verified by baseline `orchrun-test-workspace-nofailfast.log.txt` (streaming 214-test run at 4941) → all 10 `buffer::tests` pass.
— verified by read (unit math): `eta_ns = (runway_media_ns / deficit) * NS_PER_MS` (439) — deficit is media-ns per wall-ms, runway/deficit = wall-ms, ×1e6 = ns: dimensionally correct; the EWMA `prev - (prev>>2) + (new>>2)` (339) truncation is negligible at real rates (~1e6 media-ns/ms).
— verified by read: hysteresis invariants (fill needs BOTH axes 382-383, drain trips on EITHER 404-405) match the module doc's contract exactly.

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/buffer.rs:252 (the model) vs the whole repo (no constructor outside tests)
Code:
```
pub struct BufferModel {
    watermarks: BufferWatermarks,
```
What is wrong: the entire 6.D1 adaptive buffer — watermarks, EWMA stall prediction, rebuffer metrics — has zero production wiring: no non-test code constructs a `BufferModel`, calls `observe`, or forwards the resulting signals (consumer trace in header). The module doc's doctrine ("the host that drives a network source observes the buffer after every fetch/serve step") describes a host that does not exist in the shipped product.
Why it matters: 828 lines of tested policy core sit dead behind `pub mod buffer`; the reconnect/HLS/DASH consumers that import its types are themselves unwired, so the dead zone is transitive. The matrix row is honest (not_started), so this is not fraud — it is an unbuilt feature whose lowest layer quietly finished first.
What correct looks like: either the playback engine's network-source path constructs the model and forwards signals (the 6.D1→executor card), or the crate documents the module as a parked policy core pending that card.
Confidence: HIGH — Verification: rg over apps/, tests/, crates/ for every construction and call site.
Severity note: S3, not S0 — the feature-matrix row does not claim this works (status not_started, evidence ""), so no doc is contradicted.

### F-002 [S4]
Where: crates/streaming/src/buffer.rs:312-321 (production range, cross-ref to A01; behavior pinned by MY range's `reset_rates_forgets_history`)
Code:
```
    pub fn reset_rates(&mut self) {
        self.consumption_rate = None;
        self.arrival_rate = None;
        self.prev = None;
        if self.stall_predicted {
            self.stall_predicted = false;
        }
    }
```
What is wrong: `reset_rates` clears an outstanding stall prediction WITHOUT emitting `BufferSignal::StallCleared` (it returns `()`). Signal-driven consumers (the exact pattern `dash/abr.rs:202-204` implements: `StallCleared → self.stalled = false`) never learn the prediction died: after a seek, an ABR controller keeps `stalled = true` until some later observe happens to produce a fresh clear — and after a reset the first re-prediction needs two observations, so the stale flag can drive down-switches on stale state. The test `reset_rates_forgets_history` (710-721) only checks the snapshot, not the signal — the gap is invisible to it.
Why it matters: API asymmetry — every other state change surfaces as a signal; a seek is precisely when a consumer's stall state must be invalidated loudly.
What correct looks like: return the `StallCleared` signal when a prediction was outstanding (or document that hosts must treat reset as a clear).
Confidence: HIGH — Verification: read of both sites; no test asserts the signal behavior.
Chunk note: the code lives in A01's part1; recorded here because the failing-consumer pattern and the test gap are in my range. Cross-ref in §9.

### F-003 [S4]
Where: crates/streaming/src/buffer.rs:325-357 (production range, cross-ref) — untested edges visible from MY range's coverage
Code:
```
        if let Some((prev_ns, prev_buffered)) = self.prev {
            let elapsed_ms = obs.now_ns.saturating_sub(prev_ns) / 1_000_000;
            if elapsed_ms > 0 {
```
What is wrong: two observation-path edges have no test anywhere in the module: (a) `elapsed_ms == 0` (two observations in the same millisecond) silently drops the `consumed_delta_ns` and rebases `prev`; (b) a non-monotonic host clock (`now_ns` going backward) also yields `elapsed_ms == 0` via `saturating_sub` and rebases silently. Neither can panic (saturating arithmetic — good), but both are behavior the tests never pin, and (b) contradicts the "caller's monotonic clock" doctrine only softly (the model survives a violation by lying about the baseline).
Why it matters: the module's selling point is determinism under a hostile host; the zero/backward-time cases are exactly where a fake or wrapped clock lands.
What correct looks like: two tests: same-ms observations preserve rates and rebase prev; backward time is ignored (prev unchanged) or documented.
Confidence: HIGH — Verification: read; `rg "elapsed_ms" crates/streaming/src/buffer.rs` shows the single use; no test constructs either edge (all tests use strictly increasing ms-spaced timestamps).
Chunk note: code in A01's part1; gap evidenced by my range's test census. Cross-ref in §9.

## 4. Per-function table
| Test | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `model()` helper | 479-490 | REAL | — | Round watermarks (10k/2k ns, 1k/200 B), 5 ms horizon. |
| `obs()` helper | 492-500 | REAL | — | consuming=true fixed; the paused case is built inline where needed (663-664). |
| `watermark_validation_rejects_degenerate_bands` | 503-522 | REAL | — | Zero, equal, and inverted bands all refused. Would fail if validate clamped instead of refusing. |
| `watermarks_fill_on_both_drain_on_either` | 526-554 | REAL | — | Full hysteresis walk: fill, band-steady, byte-drain, duration-drain, refill. Exact signal equality. |
| `fill_requires_both_axes` | 558-577 | REAL | — | Byte-rich/duration-poor and the reverse both stay Low. |
| `rebuffer_metrics_account_episodes` | 581-611 | REAL | — | Exact durations: initial 1 ms, rebuffer 7 ms, second 3 ms; longest stays 7 ms; count 2. Any ledger mutation fails it. |
| `stall_prediction_deficit_vs_runway` | 618-667 | REAL | — | Balanced rates never predict; outage fires at t=8 (EWMA decay pinned); pause clears. ETA asserted by bound (0, 5 s]. |
| `no_prediction_when_filling` | 670-682 | REAL | — | Arrival 2x consumption: Steady. |
| `no_prediction_when_runway_outlives_horizon` | 686-707 | REAL | — | 98 s runway vs 2 s horizon: no prediction — the miss-avoidance contract. |
| `reset_rates_forgets_history` | 710-721 | REAL | S4 (F-002) | Pins snapshot-only; misses the missing StallCleared signal. |
| `fill_fraction_is_bottleneck_axis` | 724-735 | REAL | — | min-of-axes + clamp, 1e-6 tolerance. |
| `synthetic_3g_trace_reacts_truthfully` | 742-826 | REAL | — | The AC trace: exact initial_fill 2.9 s, drain at step 27, predict at step 13 (ordering pinned — the point of prediction), recovery closes the episode at 2.8 s. Strongest test in the module. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 10 | YES | Exact signal/duration/ordering equality throughout; mutation of any watermark comparison, EWMA constant, or ledger line flips an assert. Concrete example: change `>> 2` to `>> 3` in the arrival EWMA (352) and `stall_prediction_deficit_vs_runway`'s pinned `Some(8)` moves — the assert fails. |
| (gap) reset signal | NO test covers it | F-002 — `StallCleared`-on-reset is neither emitted nor tested. |
| (gap) degenerate time | NO test covers it | F-003 — same-ms and backward-clock observations untested. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Reconnect + adaptive buffering … stall prediction" (feature) | feature-matrix.toml:1457-1463 | UNREACHABLE (row says not_started — honest) | Code exists + 10 tests pass, but no shipped wiring (F-001 trace); the row's empty evidence matches reality. |
| "the observation path is allocation-free" | buffer.rs:34-36 | REAL | `observe` builds no collections (read); signals are Copy. |
| "one division per observation" | buffer.rs:44-46 | REAL | Two divisions per rate-bearing observation (336, 350) — one per rate (consumption + arrival); the doc's "one division" undercounts by one. Cosmetic drift, noted here, not counted. |
| "initial fill separated from rebuffers" | buffer.rs:28-29 | REAL | `rebuffer_metrics_account_episodes` pins both ledgers exactly. |
| "the predictor fires ~1.4 s before the drain" | buffer.rs:777-780 | REAL | Steps 13 vs 27 at 100 ms = 1.4 s, asserted (798-802). |

## 7. Dead / duplicated / unreachable code
The whole module is unreachable from shipped entry points (F-001). Within the tests: no dead helpers; both helpers are used.

## 8. Dependencies & flags
`crate::clock::WallClock` (type reference only in `now_ns`), `testkit::Must` (tests). No features, no unsafe, no panics. 39/39 lint inheritance (baseline hygiene).

## 9. Open questions for the next agent
- A01 (part1, 1-471): owns the F-002/F-003 code sites — `reset_rates` signal return and the `elapsed_ms == 0` / backward-clock edges. Confirm and severity-call from the production side.
- A09/A10 (reconnect parts): the `forward_signal` hub (reconnect.rs:532) has no production caller — the reconnect file's audit should record the same unwired verdict.
- Matrix owner: `network-reconnect-buffering` not_started is honest TODAY; when the row flips, this module's tests alone will not suffice — an executor-side e2e (model → signal → consumer) must land with it.
