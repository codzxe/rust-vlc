# AUDIT: crates/streaming/src/reconnect.rs#part2 (lines 516-1040)
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-21 |
| Lines / functions | 516-1040 (chunk boundary declared: part1 = 1-515, module docs + policy/jitter/backoff/generation + states + health + StreamEventHub through `emit` [A08]; part2 = 516-1040, hub `dropped`/`forward_signal` + ReconnectStats + RetryCore + the whole `RetryFetch` executor + `HttpFetch` impl + test harness + first two tests [A09]; part3 = 1041-1560, remaining tests [A03]) / 31 items (19 production, 12 test-harness/tests) |
| File verdict | UNREACHABLE (real, careful executor; zero shipped callers — the only production constructor path has no non-test callers) |
| Reachable from shipped entry point? | NO — `RetryFetch` is constructed only by `SchemeRegistry::with_retrying_fetch` (schemes.rs:264-270), whose only callers are schemes.rs's own tests (:958, :1015) and tests/net_interrupted.rs. The shipped CLI open lane builds `SchemeRegistry::new()` → bare `UreqClient` (apps/cli/src/demux_host.rs:491). Engine side is equally unwired: `SourceRetryGuard`/`is_retryable_network` (playback-engine recovery.rs) have no callers outside their own module/lib re-export. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `rg "with_retrying_fetch|RetryFetch::new|health_state\(|forward_signal\(" **/*.rs` (all hits in reconnect.rs itself, schemes.rs tests, net_interrupted.rs); `rg "RetryFetch|reconnect|StreamEventHub" crates/streaming/src/hls/*` + remote_playlist.rs (no matches — no adopter); read of reconnect.rs:1-1050 (part2 complete + part1/first part3 context); cross-reads: clock.rs:74-155 (SystemClock/FakeClock sleep contracts), buffer.rs:147-180 (BufferSignal variants — `forward_signal` exhaustiveness); `sed -n` quote verifies at 68-72, 778-800, 745-762; baseline: streaming unit tests NEVER RAN (both baseline test runs abort before the streaming test binary) |

## 1. One-paragraph verdict
The executor half of the reconnect doctrine is genuinely well-built: a tight fetch loop that holds no lock across I/O or sleeps, generation stamps re-checked after every inner completion and every backoff wait, exhaustion surfacing the last real error instead of a synthesized one, and a test harness (scripted transport + fake clock + recorded requests) that can prove resume-from-offset and cancellation behavior. But it is all dressing on a door nobody opens: no shipped entry point constructs `RetryFetch`, so the shipped `lumen` HTTP lane has **zero** transport-level retry — one read timeout kills the open — and the module doc's "v0 ships both layers on" is contradicted by the wiring on both layers (the engine's `SourceRetryGuard` is equally unwired). Within the code itself, the shared singleton state machine misrepresents concurrent fetches (the trait contract explicitly invites cross-thread sharing), and the stats count a planned-but-cancelled retry as issued.

## 2. What actually works (verified)
— `HttpFetch for RetryFetch::fetch` (773-817) traced by hand for max_retries ∈ {0, 3, 4}: attempt 0 is the first try; `plan_retry(attempt)` exhausts when `attempt >= max_retries` with `attempts = attempt + 1` — default 4 → 5 attempts total, matching the doc (:94-95); max_retries=0 → first failure exhausts with `Failed { attempts: 1 }` (fail-fast, pinned legal by `policy_bounds_are_validated` :1022-1025).
— Lock discipline verified by read: the core lock is taken and dropped inside `record_success`/`record_superseded`/`plan_retry` only — never across `inner.fetch` (:782) or `clock.sleep_ms` (:802); `record_*` drop the guard before `emit` (:711, :726, :745, :760).
— Generation rules: stamp at entry (:775), discard-on-completion (:785-790, success included), discard-mid-backoff (:806-811) — both cancellation paths answer `Cancelled { generation: observed }` and never deliver stale data; `record_superseded` voids the state (:721-728).
— `plan_retry` (735-763): non-retryable → `None` untouched-state passthrough (matches `NetworkError::is_retryable`, error.rs:225-250); retryable → jittered delay under the ladder, `Reconnecting` state + event with the same numbers.
— `StreamEventHub::forward_signal` (532-541): exhaustive over `BufferSignal` (verified against buffer.rs:160-180 — Steady/Filled/Drained/StallPredicted/StallCleared), Steady forwards nothing.
— `dropped`/`emit` (:507-521): `try_send` with Full/Disconnected → counted drop, never a block — bounded-memory contract honored.
— Bounded everything: attempts ≤ 16 (ceiling), single sleep ≤ 60 s (ceiling), hub capacity bounded with drop-counting; `RetryPolicy::validate` refuses rather than clamps (part1, cross-checked).

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/reconnect.rs:597-817 (the executor) + :71 (module-doc claim)
Code (module doc, part1 range, quoted for the cross-part claim):
```
//! double-spends; v0 ships both layers on, each honest about its own
//! budget.]
```
and the wiring reality (schemes.rs:236-239, what the CLI builds):
```
    pub fn new() -> SchemeRegistry {
        SchemeRegistry {
            fetch: Arc::new(UreqClient::new()),
```
What is wrong: `RetryFetch` — and with it the entire reconnect module — is unreachable from the shipped binary. The doc's "v0 ships both layers on" is false on both layers: the fetch layer is never wrapped (CLI → bare `UreqClient`), and the engine layer's `SourceRetryGuard` has no callers (rg: recovery.rs + lib.rs re-export only). The matrix row `network-reconnect-buffering` honestly says `not_started` (docs/feature-matrix.toml:1457-1463, evidence=""), so the code doc overstates what ships.
Why it matters: the user-visible consequence is that `lumen open http://…` has no retry at any layer — a single transient read timeout or connection reset aborts the open — while a reader of reconnect.rs believes "both layers on". This is the wave's central question ("what really opens a URL?") answered with: a fragile, retry-less lane, with 1,560 lines of tested resilience machinery parked beside it.
What correct looks like: either compose `RetryFetch` in `SchemeRegistry::new()` (the composition point exists and is tested at schemes.rs:958-1027) and update the matrix row with evidence, or fix the module doc to say "both layers implemented, wiring lands with the engine's network lane".
Confidence: HIGH — Verification: read + rg (full caller sweep in header; demux_host.rs:474-500 read).

### F-002 [S3]
Where: crates/streaming/src/reconnect.rs:570-577 (`RetryCore`) + 773-817 (`fetch`)
Code:
```
struct RetryCore {
    schedule: BackoffSchedule,
    state: ReconnectState,
    stats: ReconnectStats,
}
```
What is wrong: `RetryFetch` implements `HttpFetch`, whose contract is "an [`HttpFetch`] handle is shared across worker threads" (fetch.rs:497-499) — yet `RetryCore.state` is ONE state machine for every in-flight fetch. Two concurrent `fetch` calls interleave transitions: fetch A fails and parks in `Reconnecting` (inside `plan_retry`, :752-755); fetch B succeeds first-try and overwrites the state to `Steady` (:714); A's retry is still pending. `state()`/`health_state()` — the engine seam this module exists to serve — then reports `Steady` while a reconnect is in flight. The jitter schedule is likewise shared (documented as intentional for draws, but the state model is not qualified anywhere).
Why it matters: the trait invites concurrent use and the engine seam is designed for live queries; under concurrency the health verdict is arbitrary (last writer wins). No test in the module exercises two overlapping fetches (all tests are sequential — verified against the test list :1007-1552).
What correct looks like: either document the single-in-flight-fetch limitation on the type and assert it (debug_assert / a try-lock permit), or move the per-fetch state machine onto the fetch call (a `FetchState` local to `fetch`, aggregating into the shared `ReconnectState` only at boundaries).
Confidence: MEDIUM — Verification: read (570-577, 702-770, 773-817; fetch.rs:495-502); no concurrent test exists (test list read); behavior under interleaving reasoned, not executed (tests never ran — see §5).
Confidence note: the interleaving is mechanical from the code read; MEDIUM only because the engine lane that would consume the seam is itself unwired, so today's blast radius is latent.

### F-003 [S4]
Where: crates/streaming/src/reconnect.rs:749-751 (`plan_retry`) + 806-811 (mid-backoff cancel)
Code:
```
        let delay_ms = core.schedule.delay_ms(attempt);
        let retry_number = attempt + 1;
        core.stats.retries += 1;
```
What is wrong: `retries` is incremented when the retry is *planned*; a generation bump during the subsequent sleep cancels the fetch (:806-811) before the retry is ever issued, but the counter is not rolled back and `ReconnectStats::retries` is documented as "Retry fetches **issued**" (:553-554). Same for `Reconnecting` state (set at :752, voided to `Steady` by `record_superseded` — state self-corrects, the counter does not).
Why it matters: telemetry drift — a cancellation-heavy session reports retries that never hit the wire; the doc/code pair disagrees about what the number means.
What correct looks like: increment `retries` after the sleep survives the generation re-check (or rename the field "retries_planned").
Confidence: HIGH — Verification: read (735-763, 796-813, 550-564).

### F-004 [S4]
Where: crates/streaming/src/reconnect.rs:782 (request clone on every attempt)
Code:
```
            let result = self.inner.fetch(request.clone());
```
What is wrong: the request is cloned inside the loop, so the FIRST attempt — the overwhelmingly common case — also pays a clone (URL String + headers Vec + all). The module doc claims "the only allocations are the request clone **per retry**" (:81-82).
Why it matters: one avoidable allocation per successful fetch on the open path's hot seam, and a doc/code mismatch about even that.
What correct looks like: `let mut request = request;` outside the loop, clone only when a retry is scheduled.
Confidence: HIGH — Verification: read (778-782, module doc :80-83).

### F-005 [S4]
Where: crates/streaming/src/reconnect.rs:819-1040 (test harness) vs audit/runs/baseline/
What is wrong: the entire test module — like every streaming unit test — never executed in the committed baseline (both runs abort before the streaming test binary; test-workspace.log.txt ends at apps/desktop, test-workspace-allfeatures.log.txt at first-vertical). Also `tests/net_interrupted.rs` (the socket-level retry e2e) is in no wave-10 assignment.
Why it matters: REAL requires execution evidence; the executor's correctness story currently rests on read-only reasoning.
What correct looks like: `cargo test -p streaming --no-fail-fast` captured (REQUEST-ORCH-RUN); assign net_interrupted.rs.
Confidence: HIGH — Verification: baseline log section inventories read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `StreamEventHub::dropped` | 517-521 | REAL | — | Relaxed load, fine. |
| `StreamEventHub::forward_signal` | 523-541 | REAL | — | Exhaustive over BufferSignal (verified vs buffer.rs:160-180); Steady → no emit. |
| `ReconnectStats` | 548-564 | REAL | S4 | F-003 (`retries` semantics drift). |
| `RetryCore` | 570-577 | REAL | S3 | F-002 (singleton state vs concurrent fetches). |
| `RetryFetch` (struct) | 597-604 | REAL | — | Fields as documented. |
| `RetryFetch::new_checked` | 616-635 | REAL | — | Validates policy; jitter seeded from clock reading (ms-resolution — two same-ms wrappers share a sequence; trivial). |
| `RetryFetch::new` | 644-649 | REAL | — | Delegates; Result for honesty. |
| `with_event_hub` | 651-657 | REAL | — | Builder. |
| `with_generation` | 659-667 | REAL | — | Builder; the session-owner seam. |
| `generation` | 669-674 | REAL | — | Clone handle. |
| `policy` | 676-680 | REAL | — | Accessor. |
| `state` | 682-686 | REAL | S3 | Query surface for F-002's shared state. |
| `stats` | 688-692 | REAL | — | Accessor. |
| `lock_core` | 694-700 | REAL | — | Poisoning recovery, house pattern. |
| `record_success` | 702-716 | REAL | — | Correct transitions (retries>0 → Recovering+event; else Steady); lock dropped before emit. |
| `record_superseded` | 718-728 | REAL | — | Voids state to Steady, emits Superseded; documented semantics. |
| `plan_retry` | 730-763 | REAL | S4 | F-003; exhaustion math verified for 0/3/4 retries. |
| `emit` | 765-770 | REAL | — | No-op without hub. |
| `HttpFetch::fetch` | 773-817 | REAL | S3/S4 | F-001 (unreachable), F-002 (concurrency), F-004 (first-attempt clone); cancellation rules verified by trace. |
| test: `Weather`/`Outcome` | 827-863 | REAL | — | Script vocabulary incl. bump-then-answer shapes. |
| test: `ScriptedFetch` | 865-950 | REAL | — | Legit transport double; records requests (resume proof surface); repeat-last semantics. |
| test: `fast_policy`/`setup`/`drain`/`sleeps`/`get` | 952-1003 | REAL | — | Harness; `sleeps`' zero-filter is redundant (FakeClock already drops zero sleeps — clock.rs:148-154). |
| test: `policy_bounds_are_validated` | 1005-1026 | REAL | — | All four refusals + zero-retries-legal; would catch validate regressions. |
| test: `per_scheme_policy_is_bounded` | 1028-1037 | REAL | — | Pins the one-posture table; weak but honest. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `policy_bounds_are_validated` (:1007) | YES | Each invalid invariant asserted directly; zero-retry posture pinned legal. |
| `per_scheme_policy_is_bounded` (:1031) | YES | Divergent per-scheme policy breaks the equality asserts. |
| (part3 tests reference my harness) `retry_recovers_after_transient_failures`, `resume_from_offset_reissues_identical_range`, `late_response_discarded`, `cancellation_during_backoff` (:1121-1383, A03) | YES (from read) | Scripted outcomes + recorded requests + fake-clock sleeps — the harness in my range can prove them. |
| **Baseline execution** | **NEVER RAN** | F-005. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "v0 ships both layers on, each honest about its own budget" | module doc :71 | FALSE | F-001: neither RetryFetch nor SourceRetryGuard is wired into any shipped path. |
| "the engine's recovery guard … retries Network{Timeout\|ConnectionLost\|Dns} reads within its own window" | module doc :55-57 | REAL-UNPROVEN | recovery.rs implements it (:293-300) but has no callers — the guard is real code, not a shipped behavior. |
| "Exhaustion here surfaces the last underlying error … rather than a synthesized 'exhausted' error" | module doc :63-65 | REAL | :797-799 returns `err` verbatim. |
| network-reconnect-buffering: not_started, evidence "" | docs/feature-matrix.toml:1457-1463 | REAL | Matches the wiring reality (honest row). |

## 7. Dead / duplicated / unreachable code
The executor, hub, stats, and health seam are all unreachable from shipped (F-001). `RetryPolicy::per_scheme` is a typed seam whose every arm returns the default (documented as intentional). No duplication found within my range.

## 8. Dependencies & flags
`crate::buffer` (BufferSignal/BufferLevel), `crate::clock` (WallClock port — SystemClock/FakeClock both real), `crate::error`, `crate::fetch`. No feature gates. `tests/net_interrupted.rs` unassigned in wave 10.

## 9. Open questions for the next agent
- A03 (part3): do the tests pin the F-003 drift (a superseded-mid-backoff fetch's `retries` count)? `cancellation_during_backoff` (:1322) is the candidate — check whether it asserts `stats.retries`.
- A08 (part1): `RetryPolicy::per_scheme` returning the default for every scheme (including "rtsp", which the transport refuses earlier) — the doc calls it "the typed seam where divergence lands"; consider whether an unreachable scheme should even consult policy.
- Orchestrator: assign tests/net_interrupted.rs; REQUEST-ORCH-RUN `cargo test -p streaming --no-fail-fast --release --locked`.
