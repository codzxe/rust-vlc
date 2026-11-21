# AUDIT: crates/playback-engine/src/recovery.rs#part1 (lines 1–567)
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 567 (chunk L1–567 of 1136) / 20 rows |
| File verdict | REAL / UNREACHABLE (per-guard split — see F-002) |
| Reachable from shipped entry point? | NO for `SourceRetryGuard`/`SurfaceGuard` (only callers: `tests/recovery_env.rs`, both test files; the shipped http lane uses streaming's own `RetryFetch`, not this guard). `AudioDeviceGuard` is code-reachable via `Engine`'s hotplug state (`session.rs:961 hotplug: crate::hotplug::HotplugState` → `hotplug.rs:85,135,147`) but the driving methods (`Engine::observe_audio_device_lost`, `Engine::poll_hotplug`) have NO shipped caller (tests/hotplug.rs only) and the shipped audio backend is the null device (baseline `lumen engine-info` → `audio.null-device`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 0 |
| Commands run | `rg -n "SourceRetryGuard|AudioDeviceGuard|SurfaceGuard|..."` over crates/ + apps/ (complete caller map); `rg -n "hotplug::|HotplugStep"` (hotplug wiring: session field exists, drivers test-only); `rg -n "retry|Retry|reconnect"` in crates/streaming/src (found the wired twin `RetryFetch`, reconnect.rs:597); `rg -n "recovery_env|hotplug"` in nofailfast baseline (21/21 + 8/8 green, :3897–3995); `rg -n "recovery_matrix"` (trunc4 FAILURE :2566–2578); `sed -n` on every quoted range. |

Chunk boundary note: part1 = L1–567: module docs, `RetryWindow`, `RecoveryAction`, `RecoveryStep`, `ReadOutcome`, `is_retryable_network`, `retry_exhausted_error`, `SourcePhase`, and `SourceRetryGuard` through `fail_early` (L564; its body L564–576 plus `exhaust_after_failure`/`exhaust_by_time`/`terminal_answer`/`shallow_error` L578–640 were read for context and are A10's part2 rows — no double-count).

## 1. One-paragraph verdict
The retry-window state machine is genuine, carefully bounded (attempts and time bounds independent, saturating arithmetic, idempotent terminal), and heavily tested — 21/21 `recovery_env` + 8/8 `hotplug` green in the no-fail-fast baseline (:3897–3995), including engine-integrated network-interruption tests. But the entire environment-recovery story is a library without a driver: `SourceRetryGuard` and `SurfaceGuard` have zero production callers, the real http lane ships its own reconnect executor (`streaming::reconnect::RetryFetch`), and the audio path that would drive `AudioDeviceGuard` is the null device with no host ever calling the Engine hotplug hooks. On top of that, one documented config corner is wrong: `max_attempts = 0` still grants exactly one retry because `poll` never checks the attempt budget. The module docs read as if environment recovery is engine behavior; in the shipped binary none of it can fire.

## 2. What actually works (verified)
— Bounded retry state machine (window open/advance/exhaust-by-attempts/exhaust-by-time, non-retryable passthrough, idempotent terminal): executed green in baseline (`source_retry_backoff_schedule_is_bounded_and_deterministic`, `source_retry_window_expires_by_time_with_attempts_left`, `source_tls_failure_is_never_retried`, … nofailfast :3966–3995); saturation arithmetic verified by read (`backoff_ns` L167–171 shift capped at 63, `saturating_mul`, `min`).
— Backoff schedule math: `min(base·2^(attempt−1), max_backoff_ns)` verified by read and pinned by `source_retry_backoff_schedule_is_bounded_and_deterministic` (baseline green).
— `session.buffering` bookend discipline (0.0 at open, 1.0 at close on resume AND on exhaustion AND on early fatal — no stranded open episode): every exhaustion path pushes the 1.0 close when a window was open (L565–568, L584–589, L603–606); engine-integrated tests green (baseline `engine_interrupted_network_rebuffers_then_resumes`, `engine_permanent_network_interruption_is_the_typed_fatal`).
— Allocation claim ("at most one event per step, `Vec::new()` allocates nothing"): verified by read — every `events` vec in L1–640 holds ≤1 event.

## 3. Findings
### F-001 [S3]
Where: crates/playback-engine/src/recovery.rs:431-437
Code:
```
        if now_ns >= next_retry_ns {
            return RecoveryStep {
                action: RecoveryAction::RetryRead {
                    attempt: retries_used + 1,
                },
```
What is wrong: `poll` grants a retry whenever the backoff elapsed and the time-window has room — it never checks the ATTEMPT budget. With `max_attempts: 0`, the first failure opens a `Waiting { retries_used: 0 }` window (L517–535), and `poll` then answers `RetryRead { attempt: 1 }` — one retry is granted. The attempt bound only fires in `observe_failure` AFTER a granted retry fails (L547–548: `retries_used >= max_attempts` with retries_used = 1 ≥ 0). The `RetryWindow` doc explicitly promises otherwise:
```
    /// Maximum **retry** attempts after the first failure (the
    /// first failure itself is not a retry). `0` exhausts on the
    /// first failure.
    pub max_attempts: u32,
```
(L138–142). For every `max_attempts ≥ 1` the invariant "Waiting ⇒ retries_used < max_attempts" holds (exhaustion leaves Waiting immediately), so the corner is exactly `max_attempts = 0` — a plausible product-config value for "never retry".
Why it matters: A policy knob documented as "exhaust on first failure" performs one retry; anyone configuring zero-retry semantics (e.g., a battery-constrained or billing-sensitive deployment) gets silently different behavior. No test covers `max_attempts = 0` (rg over recovery_env.rs: all windows use DEFAULT or custom nonzero values).
What correct looks like: `if retries_used >= self.policy.max_attempts { return self.exhaust_by_time(retries_used); }` before the grant in `poll`, plus a `max_attempts: 0` test.
Confidence: HIGH — Verification: read (L402–560) + hand-traced both phases; baseline tests confirm only the ≥1 paths.
### F-002 [S3]
Where: crates/playback-engine/src/recovery.rs:38-42
Code:
```
/// answers a [`RecoveryStep`] — one [`RecoveryAction`] for the
/// engine side plus zero or more catalog [`commands::EventKind`]s
/// for the wire side. The guards never touch the engine directly;
/// the host (input layer, audio pump, renderer thread, or a test)
/// drives them. This is the same doctrine as the event throttle
```
What is wrong: The "host" that drives these guards does not exist in any shipped entry point. Complete caller map (rg over crates/ + apps/): `SourceRetryGuard` — `tests/recovery_env.rs` only (a test-built `NetworkDemuxer` embeds it); `SurfaceGuard` — `tests/recovery_env.rs` only; `AudioDeviceGuard` — `src/hotplug.rs` only, whose driving methods (`Engine::observe_audio_device_lost`, `Engine::poll_hotplug`, `Engine::hotplug_stats`) are called exclusively by `tests/hotplug.rs`. The CLI (`apps/cli`) constructs no guard and calls no hotplug hook; the shipped audio backend is the null device (baseline engine-info: `audio.null-device`), so device loss is unobservable; the CLI has no windowed surface.
Why it matters: The module documents an environment-recovery capability ("The three environment failure policies") that the shipped binary cannot trigger. Docs/matrix readers will believe network retry, device-loss suspend, and surface-loss degrade paths are live engine behavior; every one is test-only. This is UNREACHABLE per the audit's verdict scale — real code, no shipped entry point reaches it.
What correct looks like: Either wire the guards (http lane consults `SourceRetryGuard`; the audio pump observes loss; the renderer thread reports surface loss) or move the module docs' framing from "the engine's policies" to "library policy awaiting a host (card 7.C1)".
Confidence: HIGH — Verification: rg caller map over crates/ and apps/ (listed above); hotplug wiring traced session.rs:961 → hotplug.rs:147; engine-info feature string cited from baseline INDEX.
### F-003 [S3]
Where: crates/playback-engine/src/recovery.rs:10-16
Code:
```
//! - [`SourceRetryGuard`] — the **interrupted network source**
//!   (doc 03 §4.1(d)): a live/network read that fails stays
//!   non-fatal while a *retry window* — bounded attempts with
//!   bounded backoff inside a bounded virtual-time window — has room
```
What is wrong: The shipped http(s) lane (`apps/cli/src/demux_host.rs:399–427` → the `streaming` crate) does not consult this guard; it ships its own reconnect executor, `streaming::reconnect::RetryFetch` (reconnect.rs:597, generation-stamped, event-hub-integrated). The tree therefore carries two parallel bounded-retry policies with different semantics: `SourceRetryGuard` (engine-level, virtual-time window, `session.buffering` bookends, tested against a fake demuxer) and `RetryFetch` (transport-level, wired). Doc 03 §4.1(d)'s "live/network source → stay in Buffering and keep retrying within policy" is implemented twice and enforced once.
Why it matters: Policy duplication guarantees drift: fixes to one retry policy (backoff caps, exhaustion typing) will not reach the other; the guard's carefully tested bookend discipline has no effect on real network playback because the real lane never opens its windows.
What correct looks like: One retry policy — either `RetryFetch` delegates its window decisions to `SourceRetryGuard`, or the guard is deleted and the docs point at the transport-layer policy.
Confidence: HIGH — Verification: read demux_host.rs http lane + reconnect.rs RetryFetch docs (L582–597); rg confirms no cross-reference between the two.
### Baseline cross-reference (no new severity — context for the wave)
The no-fail-fast run exposed a REAL recovery-domain failure in the shipped binary: `first-vertical recovery_matrix.rs::matrix_mp4_trunc4_opens_then_lands_in_error_typed` FAILED — "the settle lands in Error — observed 'Playing', expected 'Error'" (nofailfast :2566–2578; summary item 5). A truncated MP4 keeps PLAYING instead of settling in Error. That is media-level recovery (pipeline EOF/damage settle — pipeline.rs/session.rs territory, my file 1's neighborhood), not this module's environment guards — but it is the one recovery claim the true baseline contradicts, and none of this file's 29 green tests cover it.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1–109 | REAL (docs) | S3 | F-002/F-003: describes hosts that don't exist; honest `[PROPOSAL]` gaps recorded (L86–99). |
| `RetryWindow` (struct) | 137–151 | REAL | S3 | F-001 doc-vs-code on `max_attempts: 0`. |
| `RetryWindow::DEFAULT` | 156–161 | REAL | — | 6 retries / 10 s / 250 ms→4 s; `[PROPOSAL figures]` flagged. |
| `RetryWindow::backoff_ns` | 167–171 | REAL | — | Shift capped at 63, `saturating_mul`, `min` cap — no overflow path. |
| `impl Default for RetryWindow` | 174–178 | REAL | — | Delegates to DEFAULT. |
| `RecoveryAction` (enum) | 187–230 | REAL | — | 7 variants, `#[non_exhaustive]`; payloads documented. |
| `RecoveryStep` (struct) | 237–244 | REAL | — | action + events; ≤1 event verified. |
| `RecoveryStep::quiet` | 246–254 | REAL | — | Zero-alloc healthy path. |
| `ReadOutcome` (enum) | 262–275 | REAL | — | Data{units}/Interrupted; units count only. |
| `is_retryable_network` | 292–300 | REAL | — | Timeout/ConnectionLost/Dns retry; Tls + ReconnectExhausted don't; pinned by `retryability_classification_pins_the_network_weather_set` (baseline green). |
| `retry_exhausted_error` | 309–331 | REAL | — | Typed `Network{ReconnectExhausted}` + shallow chain root; context `source.retry.exhausted`. |
| `SourcePhase` (enum) | 333–353 | REAL | — | Streaming/Waiting/Exhausted; invariants documented per state. |
| `SourceRetryGuard` (struct) | 372–377 | REAL-UNPROVEN* | S3 | *executed only by tests (F-002). |
| `SourceRetryGuard::new` | 381–388 | REAL | — | Streaming initial phase. |
| `SourceRetryGuard::read` | 402–407 | REAL | — | Dispatch data/failure. |
| `SourceRetryGuard::poll` | 418–440 | REAL | S3 | F-001: grants without attempt-budget check; time-expiry + idempotent terminal correct. |
| `SourceRetryGuard::retrying` | 443–446 | REAL | — | Trivial. |
| `SourceRetryGuard::retries_used` | 452–463 | REAL | — | Streaming→None; Exhausted non-network→Some(0) (honest: no retries were made). |
| `SourceRetryGuard::next_retry_ns` / `window_end_ns` / `windows_opened` | 466–490 | REAL | — | Accessors; per-episode budget (never lifetime) documented and correct. |
| `SourceRetryGuard::observe_data` | 493–502 | REAL | — | Closes window with 1.0 bookend; quiet when none open. |
| `SourceRetryGuard::observe_failure` | 505–560 | REAL | — | Non-retryable→fail_early; Exhausted→terminal; degenerate window (backoff > window) exhausts with 0 attempts (L522–526 — honest corner); retry advance + both exhaustion bounds correct. |
| `SourceRetryGuard::fail_early` (head; body shared with part2) | 564–576 | REAL | — | Passthrough original error; closes open bookend; terminal via `shallow_error`. |

## 5. Test honesty
| Test (recovery_env.rs / hotplug.rs; this chunk's subjects) | Would it fail if the code were broken? | Reason |
|---|---|---|
| retryability_classification_pins_the_network_weather_set | YES | All 6 classifications asserted (:885–893, baseline green). |
| source_retry_backoff_schedule_is_bounded_and_deterministic | YES | Exact schedule asserted; cap and doubling pinned. |
| source_retry_recovers_transient_loss_with_exact_schedule | YES | Bookends and retry counts exact. |
| source_retry_window_expires_by_time_with_attempts_left | YES | The independent time bound pinned. |
| source_retry_exhaustion_answers_the_typed_network_error | YES | Typed error + attempts count asserted. |
| source_tls_failure_is_never_retried | YES | TLS passthrough fatal, no window. |
| source_non_network_failure_passes_through_fatal | YES | Corrupt/device pass through. |
| engine_interrupted_network_rebuffers_then_resumes | YES | Integrated through Session with a guard-gated fake demuxer — subject is real Session + real guard. |
| engine_permanent_network_interruption_is_the_typed_fatal | YES | Integrated fatal path. |
| (gap) max_attempts = 0 | NO TEST | F-001 corner untested. |

All executed green in nofailfast baseline (:3897–3995). The fakes (NetworkDemuxer) are honest: the subject under test is the guard + real Session, the fake is the transport, and the no-fabrication counter (`delivered`) is asserted.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "0 exhausts on the first failure" | recovery.rs L140–141 | FALSE | F-001: poll grants attempt 1 with max_attempts=0. |
| "the host (input layer, audio pump, renderer thread, or a test) drives them" | recovery.rs L39–40 | UNREACHABLE (hosts absent) | F-002 caller map. |
| "live/network read that fails stays non-fatal while a retry window has room" (doc 03 §4.1(d)) | recovery.rs L10–16 | REAL-UNPROVEN in product; REAL in library | Guard works (tests green) but the shipped http lane uses streaming::reconnect::RetryFetch instead (F-003). |
| "no state in which it retries forever" | L362–364 | REAL | Both bounds verified by read + tests. |
| "at most one event on any recovery path" | L106–108 | REAL | Every events vec ≤1 (read). |

## 7. Dead / duplicated / unreachable code
- `SourceRetryGuard`, `SurfaceGuard` (part2), `is_retryable_network`, `retry_exhausted_error` as product paths: unreachable (F-002) — exported (`lib.rs:131–134`), test-driven only.
- Duplicated policy: `SourceRetryGuard` vs `streaming::reconnect::RetryFetch` (F-003).

## 8. Dependencies & flags
Depends on `commands` (EventKind/Stage/ErrorCode), `media_types` (EngineError, NetworkCause) — workspace crates. No feature gates, no unsafe, no I/O (pure policy; virtual time injected). No new deps.

## 9. Open questions for the next agent
- To W03-A10 (recovery.rs#part2): the AudioDeviceGuard/SurfaceGuard rows continue the F-002 reachability story; also check `SurfaceGuard`'s `details: BTreeMap` payloads (L975/L1050) against the catalog contract, and whether `exhaust_after_failure`'s events ever strand a 0.0 when the window opened this call (my read: no — exhaustion before opening emits nothing).
- To the streaming-wave agents: confirm whether `RetryFetch`'s window semantics match doc 03 §4.1(d) as closely as this guard's do (F-003 drift risk).
- To the evidence wave: recovery_matrix trunc4 "observed Playing, expected Error" (nofailfast :2566) — a truncated MP4 that never settles into Error is a user-visible honesty bug in the shipped lane; engine wave should own it (session settle / pipeline EOF policy).
