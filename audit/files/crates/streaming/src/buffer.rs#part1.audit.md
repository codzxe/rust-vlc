# AUDIT: crates/streaming/src/buffer.rs#part1 (lines 1-471)
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-21 |
| Chunk boundaries | part1 = 1-471 (all production code through `obs_fraction`); `#[cfg(test)] mod tests` at 472 belongs to part2/W10-A05. Equal-split anchor 1-414 adjusted to the natural production/tests seam. |
| Lines / functions | 471 / 16 (consts, types, 8 methods/functions) |
| File verdict | UNREACHABLE (real, baseline-tested policy core; no production constructor) |
| Reachable from shipped entry point? | NO. `rg "BufferModel::new"` across the repo: only buffer.rs's own tests (480, 620). No production code constructs a `BufferModel`; `reconnect.rs:89` imports only the `BufferLevel`/`BufferSignal` types for its `forward_signal` mapping (532-535), and no production signal source exists (the model is the only producer of those signals). The shipped `lumen` open lane (file/http → probe → container) has no buffer-model stage. Executed in baseline as library code: orchrun-test-workspace-nofailfast.log.txt:4955-4959 (`buffer::tests::* … ok`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "^(pub |)fn…" buffer.rs` (outline/split); `rg "BufferModel"` repo-wide (tests-only constructors; dash/hls hits are doc comments); `rg "BufferModel::new"` (only 480/620 in-file); read of `SystemClock::now_ms` (clock.rs:74-84 — process-monotonic `Instant`, not zero-at-construction); baseline log grep; `sed -n` on all quoted ranges (verbatim). |

## 1. One-paragraph verdict
A clean, genuinely pure policy core — allocation-free observation path, typed watermark validation, integer-only EWMA math with correct dimensional analysis, and a test suite that pins episodes and predictions against virtual time. Two real defects: the initial-fill episode is anchored at clock-zero rather than construction-time, so with the crate's own production `SystemClock` (process-monotonic since first use) the headline startup-latency metric books the entire process uptime before the first fill; and the watermark state machine silently overwrites stall-edge signals, so an observation that fills and newly predicts a stall reports only `Filled` and the rising edge is lost. Neither is reachable today — no production code constructs the model — which makes this a carefully tested island waiting for the 6.D wiring that the matrix already lists as not_started.

## 2. What actually works (verified)
— verified by baseline run `orchrun-test-workspace-nofailfast.log.txt:4955-4959`: `fill_fraction_is_bottleneck_axis`, `fill_requires_both_axes`, `no_prediction_when_filling`, `no_prediction_when_runway_outlives_horizon`, `reset_rates_forgets_history` all pass — the hysteresis and prediction semantics execute.
— verified by read: `predict_stall_ns` (425-441) has no divide-by-zero (deficit==0 early return), compares the ETA against the horizon as u128 BEFORE the `as u64` cast (no truncation false-positive), and refuses to predict while paused; the EWMA update (337-354) is shift-only fixed point with no allocation; all episode arithmetic uses `saturating_sub`.

## 3. Findings

### F-001 [S2]
Where: crates/streaming/src/buffer.rs:297-300 (contract text) with clock.rs:74-84 (the production clock)
Code:
```
// The initial episode: the stream starts buffering the
// moment it is constructed (epoch 0 on the caller's
// clock — the model never reads a clock itself).
episode_started_ns: Some(0),
```
What is wrong: the initial episode is anchored at `0` on the caller's clock, asserting the clock reads zero at construction. The constructor takes no `now_ns`, and the crate's own production clock (`SystemClock::now_ms`, clock.rs:74-84) is process-monotonic since first use — never zero at model construction. The first `Filled` computes `duration = obs.now_ns - 0` (389), booking the ENTIRE process uptime preceding the stream (minutes or hours in a long-lived session) as `initial_fill_ns` — the startup-latency metric is systematically inflated by construction.
Why it matters: initial fill is the headline streaming QoE metric this module exists to produce (module doc: "the initial fill's wall time (startup latency)"); with the only shipped clock the number is garbage. The in-tree tests mask it by feeding virtual clocks that start at ~0 (buffer.rs:590-592: first observation at t=1ms, `initial_fill_ns == 1_000_000` — the assumption is baked into the test).
What correct looks like: `BufferModel::new(watermarks, stall_window_ns, now_ns)` taking the construction-time reading (matching the clock-injection doctrine the module itself preaches), or an explicit first-observation anchor (episode opens at the first `observe`, not at construction).
Confidence: HIGH — Verification: read of both files + the masking test; no execution.

### F-002 [S3]
Where: crates/streaming/src/buffer.rs:360-377 vs 400/409
Code:
```
(false, Some(eta_ns)) => {
    self.stall_predicted = true;
    self.last_eta_ns = eta_ns;
    signal = BufferSignal::StallPredicted { eta_ns };
}
...
signal = BufferSignal::Filled;
```
What is wrong: `observe` computes the stall edge first, then runs the watermark machine, whose `Filled`/`Drained` assignments (400, 409) overwrite any `StallPredicted`/`StallCleared` set earlier. An observation that crosses the fill watermarks AND newly predicts a stall (deficit > 0 with ≥4 s runway — entirely possible) returns only `Filled`; the internal `stall_predicted` flag is set, so subsequent observations take the `(true, Some(eta_ns))` arm which emits NO signal — the rising edge is never delivered to the host until the prediction clears.
Why it matters: the host that resumes playback on `Filled` is told nothing about a predicted underrun inside the 2 s horizon — the exact event the predictor exists to surface, swallowed by an undocumented signal priority. Single-value `BufferSignal` return makes the loss silent.
What correct looks like: either a priority rule that lets stall edges win over watermark transitions (stalls are the safety-critical class), or a `Vec<BufferSignal>`/bitfield return, or at minimum a documented priority and a `stall_predicted`-carrying `Filled` variant.
Confidence: HIGH — Verification: read; no test covers the fill+stall coincidence (test list: `no_prediction_when_filling` covers the opposite corner).

### F-003 [S4]
Where: crates/streaming/src/buffer.rs:127
Code:
```
pub fn validate(&self) -> Result<(), &'static str> {
```
What is wrong: typed-validation doctrine (MP §5 "never a clamp") implemented with a stringly-typed error — four distinct invariant violations collapse into `&'static str` with no machine-readable code, unlike every other module in this crate (`NetworkError::code()`, `MpdError::code()`, cue's `ParseError`).
Why it matters: callers cannot match on the failure class; inconsistent with the crate's own error vocabulary.
What correct looks like: a small `WatermarkError` enum with stable codes, folded into the host's config errors.
Confidence: HIGH — Verification: read; documented compromise (123-126) noted but the inconsistency stands.

### F-004 [S4]
Where: crates/streaming/src/buffer.rs:68-70
Code:
```
pub fn now_ns(clock: &dyn WallClock) -> u64 {
    clock.now_ms() * NS_PER_MS
}
```
What is wrong: unchecked multiply — `now_ms` beyond ~18.4e12 ms (~584 years) overflows u64 (debug panic / release wrap). `SystemClock` itself clamps `as_millis` into u64 first, so the helper can receive values that overflow only at absurd uptimes.
Why it matters: theoretical; but `saturating_mul` costs nothing and the module's own arithmetic is otherwise saturating-everywhere.
What correct looks like: `clock.now_ms().saturating_mul(NS_PER_MS)`.
Confidence: HIGH — Verification: read; arithmetic by hand.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| NS_PER_MS / DEFAULT_* consts | 63-86 | REAL | — | sane defaults (4 s/500 ms, 1 MiB/64 KiB, 2 s horizon) |
| now_ns | 68-70 | REAL (defect) | S4 | F-004 unchecked multiply |
| BufferWatermarks::Default | 108-117 | REAL | — | mirrors consts |
| BufferWatermarks::validate | 127-142 | REAL (defect) | S4 | F-003 stringly error; invariants themselves correct (strict drain<fill both axes, positive) |
| BufferLevel | 147-155 | REAL | — | two-state hysteresis, default Low |
| BufferSignal | 160-180 | REAL | — | Copy; five variants; see F-002 for the delivery gap |
| RebufferMetrics | 186-203 | REAL | — | VLC-shaped accounting (initial fill separated) |
| BufferSnapshot | 207-224 | REAL | — | Copy; f32 fraction documented as min-of-axes |
| BufferObservation | 229-245 | REAL | — | deltas-not-absolutes contract; negatives via reset_rates |
| BufferModel::new | 284-305 | REAL (defect) | S2 | F-001 zero-anchored initial episode; validates watermarks |
| watermarks | 308-310 | REAL | — | trivial getter |
| reset_rates | 314-321 | REAL | — | also clears an outstanding prediction — correct (rates are its basis) |
| observe | 325-421 | REAL (defect) | S3 | F-002 signal priority; rate math, episode ledger, hysteresis otherwise correct; allocation-free as claimed |
| predict_stall_ns | 425-441 | REAL | — | no div-by-zero, u128 compare before cast, paused-aware; honest ETA |
| snapshot | 444-462 | REAL | — | min-of-axes fraction clamped; division-free query path as documented |
| obs_fraction | 465-470 | REAL | — | zero-total guarded |

## 5. Test honesty
Tests (472-827) belong to part2/W10-A05. Read for evidence: `rebuffer_metrics_account_episodes` (582) pins the full episode ledger against virtual time — and its first observation at t=1ms encodes the F-001 zero-anchor assumption (`initial_fill_ns == 1_000_000` would be false with a realistic clock offset). `no_prediction_when_filling` (671) covers the opposite corner of F-002. The suite is behavior-pinning, not tautological — but both defects sit precisely in the corners it does not feed (non-zero clock origin; fill+stall coincidence).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the initial fill's wall time (startup latency)" is measured | buffer.rs:200-202, 279-280 | FALSE with the production clock | F-001: anchored at 0; SystemClock never reads 0 at construction |
| "nothing here fetches, sleeps, or allocates … observation path is allocation-free" | buffer.rs:33-36 | REAL | read of observe/snapshot — no allocation, Copy types |
| `network-reconnect-buffering` row: "not_started" all platforms | feature-matrix.toml:1466-1475 | Accurate for wiring, understated for code | BufferModel (6.D1 policy half) exists and is tested; no constructor in production — the row should say "library landed, unwired" like the code reality |
| "Rates are kept per-millisecond internally … one division per observation" | buffer.rs:43-46 | REAL | 333-354: exactly two divisions per observation (one per rate) — doc says "one division per *observation*" per rate; minor wording, math verified |

## 7. Dead / duplicated / unreachable code
Whole model unreachable from shipped entry points (see header). Within the file, nothing dead. `reconnect.rs`'s `forward_signal` consumes the signal types but has no production signal producer — the types are referenced, the engine is not.

## 8. Dependencies & flags
std-only; depends on `crate::clock::WallClock` (the port, not a concrete clock — construction would inject `SystemClock::shared()` in production, which is what makes F-001 live). No feature gates. No unsafe.

## 9. Open questions for the next agent
- Part2 owner (W10-A05): do any tests feed a non-zero clock origin (F-001) or a fill+stall coincidence (F-002)? My read of 479-618 says no.
- Wave owner: when 6.D wiring lands, F-001 becomes a user-visible metric bug on day one — the constructor signature must change first. Record as a wiring precondition.
