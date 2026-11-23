# AUDIT: crates/streaming/src/buffer.rs
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-23 |
| Lines / functions | 827 / 10 production fns/methods + 10 tests |
| File verdict | UNREACHABLE (real, well-tested pure policy core — zero production callers) |
| Reachable from shipped entry point? | NO — `BufferModel::new`/`observe` are called only by buffer.rs's own tests; repo-wide grep shows every other mention is a doc comment (hls/master.rs, dash/sched.rs, dash/abr.rs, reconnect.rs) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | `cargo test -p streaming --lib buffer::` -> 10 passed / 0 failed (exit 0); repo-wide rg for `BufferModel|BufferObservation|BufferWatermarks`; `sed -n` quote verification |

## 1. One-paragraph verdict
The 6.D1 buffer model is a genuinely implemented, genuinely tested pure policy core: two-axis watermark hysteresis (fill on both bytes AND duration, drain on either), EWMA consumption/arrival rates with shift-only merging, honest stall prediction with ETA, and VLC-shaped rebuffer accounting — all 10 tests pass under my run, including a synthetic 3G trace with exact episode timings (predict at 1.3 s, drain at 2.7 s, 2.8 s episode). It is also completely dead: no host anywhere in the repo constructs a `BufferModel` or calls `observe`; the module doc's "the host that drives a network source observes the buffer after every fetch/serve step" describes a host that does not exist. Only the `BufferSignal` enum escapes this file, feeding ABR seams (hls/master.rs, dash) that are themselves unwired. Plus one real signal-loss bug: a watermark transition in the same observation silently overwrites a stall edge.

## 2. What actually works (verified)
— verified by `cargo test -p streaming --lib buffer::` -> `test result: ok. 10 passed; 0 failed` (validation, both-axes fill/either-drain, episode accounting with exact ns values, deficit-vs-runway prediction with bound-checked ETA, pause-clears-prediction, filling-never-predicts, runway-outlives-horizon, reset, fill fraction, 3G trace).
— verified by repo-wide rg: `BufferModel` appears outside this file only in doc comments (hls/master.rs:29,154; dash/sched.rs:14; dash/abr.rs:15; reconnect.rs:395) — no construction site, no `observe` call site.

## 3. Findings
F-001 [S2]
Where: crates/streaming/src/buffer.rs:6-10 (doc) vs repo reality
Code:
```
//! The model is **data in, signals out** — the recovery.rs doctrine
//! from the playback engine ("policy is data, the executor owns
//! mutation"): the host that drives a network source observes the
//! buffer after every fetch/serve step
//! ([`BufferModel::observe`](BufferModel::observe)) and the model
```
What is wrong: the observing host does not exist. Nothing in lumen-cli, desktop, input, playback-engine, or even the streaming crate's own sessions constructs a `BufferModel`; the only `new()`/`observe()` call sites are this file's tests. Every downstream consumer (HLS ABR `on_buffer_signal`, DASH sched) waits for signals no code will ever produce. The whole card 6.D1 — stall prediction, rebuffer metrics — is library-shelf code with a doc that describes an integration that was never written. What correct looks like: either a host lane in the CLI open path (observe after each ranged/HLS fetch/serve) or an honest "unwired, library-only" status line like the record files carry.
Confidence: HIGH — Verification: ran (repo-wide rg, callers enumerated).
F-002 [S3]
Where: crates/streaming/src/buffer.rs:359-377 and 379-412
Code:
```
            (false, Some(eta_ns)) => {
                self.stall_predicted = true;
                self.last_eta_ns = eta_ns;
                signal = BufferSignal::StallPredicted { eta_ns };
            }
```
then later in the same call:
```
                if filled {
                    self.level = BufferLevel::High;
```
…ending `signal = BufferSignal::Filled;` (line 400) / `signal = BufferSignal::Drained;` (line 409).
What is wrong: `signal` is assigned by the stall-edge block first and then unconditionally overwritten by the watermark block when both transition in one observation — watermark wins, silently. A `StallPredicted` rising edge can be lost exactly when it matters: mid-recovery, the buffer crosses the fill watermark (→`Filled`) while the arrival EWMA is still depressed enough that the deficit predicts an underrun inside the horizon (reachable with fill watermark ≪ horizon; e.g. fill_media 1 s, horizon 5 s, deficit 0.5x → ETA 2 s). The host never sees the warning; internal state says `stall_predicted = true` so only a later poll of `snapshot()` recovers the fact — the edge event is gone. Same for `StallCleared` overwritten by `Drained`. What correct looks like: return both (a small signal set), or document and assert the priority.
Confidence: MEDIUM — Verification: read (control flow traced); the co-occurrence requires non-default watermark/horizon ratios, not executed.
F-003 [S4]
Where: crates/streaming/src/buffer.rs:314-321
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
What is wrong: the `if` is dead weight (identical to `self.stall_predicted = false;`), and `last_eta_ns` is not reset (masked only because `snapshot` gates on `stall_predicted`) — stale state kept around for no reason. Cosmetic.
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/streaming/src/buffer.rs:296-301
Code:
```
            // The initial episode: the stream starts buffering the
            // moment it is constructed (epoch 0 on the caller's
            // clock — the model never reads a clock itself).
            episode_started_ns: Some(0),
```
What is wrong: the initial-fill episode's start is hardcoded to epoch 0 of the caller's clock. A host that constructs the model at process start and first observes at t=10 s books a 10 s `initial_fill_ns` (startup-latency metric inflated by construction-to-first-observe delay). Documented in the comment, but it is a metrics footgun with no `begin(now_ns)` API to avoid it.
Confidence: HIGH — Verification: read (arithmetic: `duration = obs.now_ns.saturating_sub(started)`, line 389).
F-005 [S4]
Where: crates/streaming/src/buffer.rs:127-142
Code:
```
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.fill_media_ns == 0
```
What is wrong: stringly-typed `&'static str` errors on a public API, against the crate's own typed-error house style (`NetworkError`, `SchemeError` dotted codes). Same pattern as schemes.rs's `with_retrying_fetch`.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| now_ns (free fn) | 68-70 | REAL | — | clock.now_ms()×1e6; overflow needs 584 y of ms |
| BufferWatermarks::Default::default | 108-117 | REAL | — | 4 s/500 ms + 1 MiB/64 KiB pairs |
| BufferWatermarks::validate | 127-142 | REAL | S4 | Correct invariants (drain<fill both axes, positive); stringly errors |
| BufferModel::new | 284-305 | REAL | S4 | Delegates to validate; epoch-0 episode (F-004) |
| BufferModel::watermarks | 308-310 | REAL | — | Getter |
| BufferModel::reset_rates | 314-321 | REAL | S4 | F-003 |
| BufferModel::observe | 325-421 | REAL (buggy edge) | S3 | F-002 overwrite; rates one-division-per-observation as documented; saturating arithmetic throughout |
| BufferModel::predict_stall_ns | 425-441 | REAL | — | Units correct (media-ns ÷ (media-ns/wall-ms) = wall-ms ×1e6); u128 math, no overflow at u64 inputs; paused → None |
| BufferModel::snapshot | 444-462 | REAL | — | Bottleneck-axis fill fraction, clamped; allocation-free as documented |
| obs_fraction | 465-470 | REAL | — | Zero-guard + clamp |
| tests (10) | 472-827 | REAL | — | See §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| watermark_validation_rejects_degenerate_bands | YES | 3 degenerate shapes refused, default accepted |
| watermarks_fill_on_both_drain_on_either | YES | Exact signal sequence across band, both axes probed independently |
| fill_requires_both_axes | YES | Byte-rich/duration-poor and reverse both held Low |
| rebuffer_metrics_account_episodes | YES | Exact ns durations: initial 1 ms, rebuffer 7 ms, longest retention across 2 episodes |
| stall_prediction_deficit_vs_runway | YES | Prediction fires exactly at t=8 on the decay trace, ETA bounded (0, 5 s]; pause → StallCleared |
| no_prediction_when_filling | YES | Arrival>consumption → no signal |
| no_prediction_when_runway_outlives_horizon | YES | 98 s ETA vs 2 s horizon → steady |
| reset_rates_forgets_history | YES | All three histories + forecast cleared |
| fill_fraction_is_bottleneck_axis | YES | 0.5 exact both orderings, clamp to 1.0 |
| synthetic_3g_trace_reacts_truthfully | YES | Exact step indices (predict 13, drain 27) and exact episode ns — I re-derived the trace arithmetic; matches |
Caveat: every test drives the model directly; no test covers the lost-edge co-occurrence of F-002 (the tests never combine a stall edge with a watermark transition in one observation).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the host that drives a network source observes the buffer after every fetch/serve step" | buffer.rs:8-10 | FALSE | No host exists; observe() has zero non-test callers (F-001) |
| "nothing here fetches, sleeps, or allocates… observation path is allocation-free" | buffer.rs:33-36 | REAL | observe/snapshot take &mut self/obs by value, no heap (read) |
| "one division per observation" | buffer.rs:44-46 | REAL | Two divisions (consumption + arrival) at 336/350 — per-observation constant, claim substantively true |
| "fill on both axes, drain on either" | buffer.rs:13-17 | REAL | Code 380-411 + tests |
| "rates are kept per-millisecond… as u128 fixed-point" | buffer.rs:43-44 | REAL | Fields 259-260 |

## 7. Dead / duplicated / unreachable code
- The entire module is unreachable from shipped entry points (F-001). `BufferSignal` is the only externally consumed item (hls/master.rs:83, dash/abr.rs:34) — as parameter types of seams that are themselves never fed.
- `RebufferMetrics::buffered_*` duplicates the observation inputs back out (host already had them) — harmless telemetry echo.

## 8. Dependencies & flags
- No feature gates; std-only; sole dep is `crate::clock::WallClock` (used only by the free `now_ns` helper). No I/O, threads, or alloc — matches the module's claims.

## 9. Open questions for the next agent
- If a host lane ever lands, re-test F-002's co-occurrence with production watermark/horizon ratios (default 4 s fill vs 2 s horizon makes Filled+StallPredicted co-occurrence unlikely; the bug needs fill ≪ horizon).
- dash/sched.rs `on_buffer_signal` consumes these signals — check whether DASH sessions are wired to any host (my expectation: same unwired status as HLS).
