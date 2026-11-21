# AUDIT: crates/streaming/src/reconnect.rs#part3 (lines 1041-1560)
| Field | Value |
|---|---|
| Auditor | W10-A03 |
| Date | 2026-11-21 |
| Lines / functions | 520 (1041-1560) / 19 (18 `#[test]` fns + 1 local clock double `BumpingClock`). Part boundaries: part1 (W10-A10) = 1-520 (module docs, consts, `RetryPolicy`, `Jitter`); part2 (W10-A09) = 521-1040 (`BackoffSchedule`, `GenerationCounter`, `ReconnectState`/`HealthState`/`health_state`, `StreamEvent`/`StreamEventHub`, `ReconnectStats`, `RetryCore`, `RetryFetch` + `HttpFetch` impl, test harness head); **part3 (MINE) = 1041-1560 — the entire behavioral test suite** (my range begins exactly at the `#[test]` of `ceiling_ladder_is_exponential_and_capped`; the shared fixtures `ScriptedFetch`/`setup`/`drain`/`sleeps`/`get` live at 820-1003 in part2's range). |
| File verdict | UNREACHABLE (the tests are REAL and executed green; the production lane they cover is not in any shipped call chain) |
| Reachable from shipped entry point? | NO. `RetryFetch`'s only adoption point is `SchemeRegistry::with_retrying_fetch` (schemes.rs:264-272), whose callers are schemes.rs's own `#[cfg(test)]` tests (schemes.rs:958, 1015). The shipped CLI's HTTP open path is `apps/cli/src/demux_host.rs:491` → `SchemeRegistry::new().open_http(...)` — the registry WITHOUT the retry wrapper. `health_state`/`StreamEventHub`/`forward_signal` have zero callers outside reconnect.rs itself (rg across crates + apps). The streaming crate is linked into `lumen` (CLI dep), but this module's runtime lane is never composed in. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "RetryFetch\|with_retrying_fetch\|health_state\|StreamEventHub" crates apps` (scoped) → census above; `sed -n '240,290p' crates/streaming/src/schemes.rs` → with_retrying_fetch body; `sed -n '474,495p' apps/cli/src/demux_host.rs` → CLI uses SchemeRegistry::new(); `rg -n "events_dropped" crates/streaming/src` → declaration only (reconnect.rs:563), never written; `sed -n` on all quoted ranges; `rg -n "test reconnect::tests::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` → 20/20 ok at 5110-5129 |

## 1. One-paragraph verdict
This is the best test suite I have audited in wave 10: scripted-transport replay with request recording, a mirrored-seed schedule proof that pins the exact jittered delays the wrapper slept, byte-exact `Range` header re-issue verification, two distinct generation-invalidation interleave points (a response landing after a bump, and a bump landing mid-backoff via a clock double that bumps inside `sleep_ms`), bounded-hub drop accounting, and a full truth-table test for the composite health verdict — every assertion names a typed code or an exact event sequence, and all 20 tests ran green in the no-fail-fast baseline. Mutation reasoning says the suite would catch essentially every plausible regression in the retry core: ladder shape, jitter bounds/determinism, budget accounting, state transitions, resume identity, cancellation both sides of the sleep. The two problems are structural, not local: none of this machinery is reachable from the shipped binary (the CLI opens HTTP sources through the bare `SchemeRegistry::new()`, no retry wrapper), and one telemetry field the suite would naturally have covered — `ReconnectStats::events_dropped` — is dead on arrival: declared, never written, never asserted, permanently zero.

## 2. What actually works (verified)
— All 20 reconnect tests executed green: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5110-5129` (includes all 18 tests in my range plus the 2 part2-range policy tests).
— Schedule-proof mechanism verified by reading both sides: `RetryFetch::new_checked` seeds `BackoffSchedule` from `clock.now_ms()` (reconnect.rs:622, 630); `setup()` uses a fresh `FakeClock` reading 0 (963-982), so the test's mirror `BackoffSchedule::new(fast_policy(), 0)` (1130) replays the exact draws.
— Generation interleave verified: `fetch` stamps at entry (775), re-checks after inner completion (785-790) and after every backoff sleep (806-811); `cancellation_during_backoff`'s `BumpingClock` exercises the second point through the real port (1329-1337).

## 3. Findings

### F-001 [S3] A 20-test suite guards a lane the shipped binary never composes — CLI HTTP opens have no retry
Where: crates/streaming/src/reconnect.rs:1041-1560 (the suite) ↔ apps/cli/src/demux_host.rs:491
Code:
```rust
    /// The exponential ladder: doubling to the ceiling, then flat;
    /// shift-saturation at high attempt counts stays at the ceiling.
    #[test]
    fn ceiling_ladder_is_exponential_and_capped() {
```
What is wrong: every test in my range exercises `RetryFetch`/hub/health machinery whose only production adoption point (`SchemeRegistry::with_retrying_fetch`, schemes.rs:264-272) is itself called only from schemes.rs's cfg(test) block (958, 1015). The shipped `lumen` CLI's HTTP lane constructs `SchemeRegistry::new()` (demux_host.rs:491) — no retry layer, no generation handle, no event hub.
Why it matters: the matrix row `network-reconnect-buffering` says not_started/empty-evidence (honest), but the crate's own docs (schemes.rs:253-260 "the 6.D2 adoption point") describe adoption that no shipped host performs; a user whose stream hiccups gets no bounded retry, while the repo carries a fully-proven implementation. Classic wave-10 pattern: real code, tested, dead in product.
What correct looks like: the CLI's `open_http_source` should construct the registry via `with_retrying_fetch` (that is precisely why the constructor exists), or the schemes doc must stop describing it as the adoption point.
Confidence: HIGH — Verification: rg census + read both call sites.

### F-002 [S3] `ReconnectStats::events_dropped` is declared and never written — a telemetry field that always reports zero
Where: crates/streaming/src/reconnect.rs:563 (declaration, part2's range — finding surfaced by my part3 coverage analysis)
Code:
```rust
    /// Fetches cancelled by a generation bump (late responses
    /// discarded).
    pub superseded: u64,
    /// Events the hub dropped (bounded-memory accounting).
    pub events_dropped: u64,
```
What is wrong: `rg -n "events_dropped" crates/streaming/src` returns exactly one hit — this declaration. `RetryFetch::emit` (766-770) calls `hub.emit(event)` and discards the boolean; nothing ever folds the hub's `dropped()` count into stats. No test asserts the field (the suite asserts `fetches`/`retries`/`recovered`/`exhausted`/`superseded` — all five covered in my range at 1156-1161, 1218-1222 — but `events_dropped` is absent from every assertion).
Why it matters: a host reading `stats().events_dropped` to detect an undrained hub (the bounded-memory contract the field documents) always sees 0 — a lying sensor is worse than no sensor; the suite's silence is exactly how it went unnoticed.
What correct looks like: `emit()` (or a stats refresh) records `hub.dropped()` delta into `core.stats.events_dropped`, plus one test where the hub saturates and the stat increments.
Confidence: HIGH — Verification: rg single-hit + read emit/plan_retry/record_* — no write site exists.

### F-003 [S4] The hub's `Disconnected` branch is never tested — a mutation there passes the whole suite
Where: crates/streaming/src/reconnect.rs:1410-1426
Code:
```rust
        let (hub, receiver) = StreamEventHub::with_capacity(2);
        assert!(hub.emit(StreamEvent::StallCleared), "first fits");
        assert!(hub.emit(StreamEvent::StallCleared), "second fills it");
        assert!(
            !hub.emit(StreamEvent::StallCleared),
            "the third drops, never blocks"
        );
```
What is wrong: `hub_is_bounded_and_counts_drops` saturates the channel (`TrySendError::Full`) but never drops the `Receiver` to exercise `TrySendError::Disconnected` (reconnect.rs:510). A mutation returning `true` (delivered) on Disconnected, or skipping the drop counter for it, survives the entire suite.
Why it matters: receiver-drop is the common runtime shape (host tears down while a retry loop still emits); the branch's counting/return-value contract is unproven.
What correct looks like: extend the test — `drop(receiver); assert!(!hub.emit(...)); assert_eq!(hub.dropped(), 3);`.
Confidence: HIGH — Verification: read the emit match (507-515) and every hub test in the file; no drop-receiver case exists.

## 4. Per-function table (part3 only — all items are tests)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| test `ceiling_ladder_is_exponential_and_capped` | 1041-1052 | REAL | — | Exact ladder 100/200/400/800, cap at ≥4, shift-saturation at 64 and 1000; kills min/saturating_mut/pow mutations. |
| test `jitter_is_deterministic_and_bounded` | 1056-1076 | REAL | — | Same-seed replay, cross-seed divergence, strict bound, zero-seed escape (JITTER_ZERO_SEED pinned), bound 1/0 edges. |
| test `backoff_full_jitter_within_ceiling` | 1081-1102 | REAL | — | Strictly-inside-ceiling over attempts 0..6 × 24 draws; distinctness >1; the replay block (1096-1101) is redundant with jitter determinism (harmless — noted, not a defect). |
| test `generation_counter_advances` | 1105-1114 | REAL | — | Start 0, monotonic bump, clone-shares semantics. |
| test `retry_recovers_after_transient_failures` | 1120-1168 | REAL | — | The suite's centerpiece: mirrored-seed delay proof (1130-1140), exact event triple (1142-1155), all stats fields but events_dropped, state Recovering, request count 3. Note: the `if d1 > 0` filter mirrors FakeClock's zero-sleep no-op — a production change to skip sleep_ms(0) would still pass (behavior-equivalent). |
| test `steady_after_first_try_success_follows_recovery` | 1172-1183 | REAL | — | Recovering → Steady promotion pinned. |
| test `fresh_fetch_after_failure_resets_state` | 1187-1203 | REAL | — | 4×503 exhausts (max_retries 3), code string pinned ("streaming.net.http-status"), next episode resets to Steady. Script construction sound (5 outcomes + repeat-last). |
| test `retry_exhaustion_surfaces_last_error` | 1209-1231 | REAL | — | Last weather surfaces (not a synthesized "exhausted"), attempts 4, retries 3, 3 sleeps, 4 events. |
| test `non_retryable_failure_passes_through` | 1235-1248 | REAL | — | 404: 1 request, 0 retries, no sleep, state Steady, no events — pins is_retryable() through the seam. |
| test `resume_from_offset_reissues_identical_range` | 1253-1278 | REAL | — | Range header byte-identical across retry (clone() re-issue); URL identity too. |
| test `late_response_discarded` | 1283-1315 | REAL | — | Success-after-bump → `Cancelled{generation:0}`, superseded=1, Steady, Superseded event. |
| test `cancellation_during_backoff` (+ `BumpingClock`) | 1321-1378 | REAL | — | Local clock double bumps inside `sleep_ms` — the honest interleave point; no second request, events [Reconnecting, Superseded] with mirrored d1. |
| test `superseded_failure_discarded` | 1383-1405 | REAL | — | Failure-after-bump → Cancelled, no retry, no sleep. |
| test `hub_is_bounded_and_counts_drops` | 1410-1426 | REAL | S4 | Full-branch covered; Disconnected branch NOT (F-003). Drained-hub re-accept + sticky dropped-count asserted. |
| test `hub_forwards_buffer_transitions` | 1430-1447 | REAL | — | All four forwarding edges + Steady suppresses. |
| test `health_state_maps_both_models` | 1452-1513 | REAL | — | 8-row truth table incl. Failed dominance and Recovering watermark promotion. |
| test `wrapper_works_without_hub` | 1518-1539 | REAL | — | Hub-optional contract; generation handle clonability. |
| test `default_policy_is_bounded` | 1544-1559 | REAL | — | Default constants pinned; real `SystemClock::shared()` path exercised once (first-try success — no real sleeping). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 18 in range except noted | YES | Each pins exact typed values: ladder numbers, event sequences, stats counts, request counts, Range bytes, state enums — any behavioral drift fails an equality. Mutation examples reasoned: remove the post-sleep generation check → cancellation_during_backoff fails; make 404 retryable → non_retryable fails; off-by-one budget (`>=` → `>`) → exhaustion counts fail (retries 3→4). |
| hub_is_bounded_and_counts_drops | PARTIALLY | Full branch yes; Disconnected branch no (F-003). |
| backoff_full_jitter replay block | NO (redundant) | Two fresh same-seed schedules — proves Jitter determinism already pinned at 1058-1062; kept as harmless redundancy. |
| (suite-wide) | NO for events_dropped | Nothing feeds the field, so nothing can test it (F-002). |
All green: orchrun-test-workspace-nofailfast.log.txt:5110-5129.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Reconnect + adaptive buffering … Bounded retry with backoff, resume-from-offset, generation-safe cancellation, stall prediction" status not_started, evidence "" | docs/feature-matrix.toml `network-reconnect-buffering` (~1456) | REAL (honest matrix) / UNREACHABLE (code) | Implementation + this suite exist and pass (log 5110-5129); no shipped host composes it (F-001 census). |
| "every `open_http` fetch — the probe, the window fetches … retries" (adoption claim) | schemes.rs:253-260 (doc) | FALSE-in-product | The shipped CLI uses `SchemeRegistry::new()` (demux_host.rs:491); `with_retrying_fetch` is test-only (F-001). |
| "a full hub drops the event and counts it — the host that does not drain loses events before it loses memory" | reconnect.rs:478-480 | REAL | hub_is_bounded_and_counts_drops green. |
| "the fetch path treats it as telemetry, never a failure" (drop policy) | reconnect.rs:505-506 | REAL | emit discards the bool; retry tests never fail on drops. |

## 7. Dead / duplicated / unreachable code
- The tested lane is unreachable from shipped hosts (F-001). Inside the module: `ReconnectStats::events_dropped` is dead (F-002). No duplication in my range; the local `BumpingClock` double is justified (the interleave point is the port method itself).

## 8. Dependencies & flags
Tests use in-crate `clock::{FakeClock, SystemClock}`, `fetch::{ByteRange, HttpRequest}`, `testkit::Must`, std sync primitives — all dev-consistent with the crate's std-only posture. No feature gates. No `#[ignore]`. The `lumen-allow-literal` annotations on unit.test URLs are the lint-exemption convention (test vectors, verified not shipped endpoints).

## 9. Open questions for the next agent
- part2 (W10-A09): confirm no write site for `events_dropped` exists in your range either (my rg says the whole crate has only the declaration) — F-002 ownership.
- schemes.rs owner: whether the `with_retrying_fetch` doc block's adoption claim should be rewritten until a host calls it (F-001).
- buffer.rs / hls/live.rs owners: `BufferSignal` producers exist in the buffer model — do they ever reach `forward_signal` in production, or is that bridge also orphaned? (rg says orphaned; live.rs is my next file — will check the live-edge side.)
