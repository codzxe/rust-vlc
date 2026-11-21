# AUDIT: crates/streaming/src/clock.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-21 |
| Lines / functions | 215 / 15 (1 trait + 2 impls + 8 fns/methods + 4 tests) |
| File verdict | UNREACHABLE from shipped entry (the port is real and e2e-exercised via FakeClock — but the only production composition that would use `SystemClock` is not adopted by the shipped CLI path) |
| Reachable from shipped entry point? | **NO.** Consumers: `buffer.rs` (`now_ns` helper over the port), `reconnect.rs` (`RetryFetch`, clock at :600/:802), `schemes.rs::with_retrying_fetch` (:264-271). The shipped CLI HTTP path is `apps/cli/src/demux_host.rs:491` → `SchemeRegistry::new()` → **bare `UreqClient`, no retry wrapper, no clock** (schemes.rs:236-240). `with_retrying_fetch` — the sole composition that injects a clock into production — has no app caller; `SystemClock::shared()` is referenced only by reconnect.rs's tests (:1554) and a doc comment. FakeClock is exercised by the e2e fault suites (net_interrupted 3/3, netfault_proxy_e2e 7/7 — baseline green). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "sleep_ms|WallClock|SystemClock|FakeClock"` over streaming src → consumer map above. `sed -n '780,815p' reconnect.rs` → sleep-then-check-generation pattern. `sed -n '215,275p' schemes.rs` → `new()` vs `with_retrying_fetch`. Baseline: `orchrun-test-workspace-nofailfast.log.txt:4986-4989` (clock::tests 4/4 ok); `:5228-5234` (net_interrupted 3/3); netfault_proxy_e2e 7/7. |

## 1. One-paragraph verdict
A clean 215-line injectable clock port with the right doctrine (decisions take `now` from callers; only waiting touches a clock; `Instant` never `SystemTime`; poison-tolerant locks; saturating virtual time). It works, its unit tests pass, and its fake half carries two real fault-injection e2e suites to green in the baseline. Two hygiene defects: `SystemClock::shared()` locks a `Mutex<Option<…>>` on every call to read a value that never changes after init, and the port's "sleep **exactly** ms" contract oversells an at-least, uninterruptible `thread::sleep` — the reconnect consumer verifiably blocks out the full backoff budget before noticing a generation bump. The damning context is reachability: the retry composition this port exists for is not wired into the shipped binary's HTTP path — `lumen` opens URLs through a bare `UreqClient` with no reconnect and no clock.

## 2. What actually works (verified)
— Unit suite 4/4 ok: `orchrun-test-workspace-nofailfast.log.txt:4986-4989` (`clocks_are_send_sync` actually crosses a thread boundary; `fake_clock_sleep_advances_and_records` pins the schedule-recording surface).
— FakeClock drives the real fault e2es: `net_interrupted.rs` 3/3 (`:5228-5234` — mid-segment drop resume, manifest reset backoff, live-window wire delay) and `netfault_proxy_e2e.rs` 7/7 — both use `Arc<FakeClock>` as the production-shaped clock (verified imports at net_interrupted.rs:32, netfault_proxy_e2e.rs:26).
— Monotonicity design verified: `now_ms` is `Instant::elapsed()` off a process-wide `OnceLock<Instant>` epoch (76-81) — no `SystemTime`, no NTP skew; the u128→u64 clamp at 81 is unreachable in practice (584M years of ms).
— Poison handling: both mutexes recover via `unwrap_or_else(|poisoned| poisoned.into_inner())` (69, 139) — a panicked test thread cannot wedge the clock.

## 3. Findings

### F-001 [S4] — `SystemClock::shared()` locks a mutex on every call to read an immutable value
Where: clock.rs:64-71
Code:
```rust
    pub fn shared() -> Arc<dyn WallClock> {
        static SHARED: OnceLock<Mutex<Option<Arc<dyn WallClock>>>> = OnceLock::new();
        let mut guard = SHARED
            .get_or_init(|| Mutex::new(None))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.get_or_insert_with(|| Arc::new(SystemClock)).clone()
    }
```
What is wrong: the shared handle is initialized once and never replaced, yet every call takes a process-wide mutex to read it. `static SHARED: OnceLock<Arc<dyn WallClock>>` with `SHARED.get_or_init(|| Arc::new(SystemClock)).clone()` is lock-free after init and strictly simpler. The `Option` inside the `Mutex` adds a third layer that serves no one — nothing ever sets it back to `None` or swaps the inner value.
Why it matters: pure hygiene, but this is the constructor the module docs advertise as the production clock handle; on a hot path (backoff pacing calls it per retry) it serializes threads for no reason and gives reviewers a double-take.
What correct looks like: `static SHARED: OnceLock<Arc<SystemClock>>` (or `OnceLock<Arc<dyn WallClock>>` if variance matters) + `get_or_init(...).clone()`.
Confidence: HIGH — Verification: read; no other writer of the inner `Option` exists (rg).

### F-002 [S4] — "Sleep **exactly** `ms`" is not what `thread::sleep` provides, and the port has no cancellation
Where: clock.rs:51-53 (trait doc) and 84-88 (impl)
Code:
```rust
    /// Sleep **exactly** `ms` monotonic milliseconds (a bounded wait
    /// the caller budgeted; the fake advances virtual time instead).
    fn sleep_ms(&self, ms: u64);
```
(and)
```rust
    fn sleep_ms(&self, ms: u64) {
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms));
        }
    }
```
What is wrong: `std::thread::sleep` guarantees **at least** the duration (scheduler jitter, signals), not exactly; and the port offers no way to interrupt a wait — no condvar, no generation check-in, no max-slice. The one in-tree consumer pattern proves the consequence: `reconnect.rs:802-805` sleeps the full `delay_ms`, and only then checks whether the generation moved — a bump (seek, reopen, shutdown) during the wait is invisible until the whole budget has been burned.
Why it matters: bounded-but-uninterruptible waits turn "cancel" into "cancel after up-to-N-seconds"; the doc's "exactly" tells API users a precision the platform does not offer. Blast radius today is nil (the retry layer is not shipped — see header), which keeps this S4 rather than S3.
What correct looks like: doc "at least `ms`"; and/or slice the sleep (e.g. 250 ms max per `sleep_ms` call in a loop that re-checks the generation), or add `sleep_until_ms(deadline, cancelled: &AtomicBool)`.
Confidence: HIGH — Verification: read + reconnect.rs:796-812 consumer trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `WallClock` (trait) | 46-54 | REAL | S4 (F-002) | Two-method port; `Send + Sync` bounds pinned by test. |
| `SystemClock` (type) | 58-59 | REAL | — | Zero-sized; `Copy`. |
| `SystemClock::shared` | 64-71 | REAL | S4 (F-001) | Over-locked singleton (F-001). Returns the only production clock — with zero production callers (see header). |
| `SystemClock::now_ms` | 74-82 | REAL | — | Process-wide `OnceLock<Instant>` epoch inside the method — shared across all instances (unit type, so correct); clamp unreachable. |
| `SystemClock::sleep_ms` | 84-88 | REAL | S4 (F-002) | Zero-guard present; at-least semantics, uninterruptible. |
| `FakeClock` + state | 100-109 | REAL | — | `Arc<Mutex<state>>`; clone shares time — pinned by test. |
| `FakeClock::new` | 111-115 | REAL | — | Frozen at 0. |
| `FakeClock::set_ms` | 117-122 | REAL | — | Test-surface; used only by this file's own test — pub API with a single in-crate test caller (near-dead, acceptable as documented test surface). |
| `FakeClock::advance_ms` | 124-128 | REAL | — | Saturating add. |
| `FakeClock::sleeps` | 130-134 | REAL | — | The assertion surface the e2e suites' backoff checks consume (reconnect tests read schedules through it). |
| `FakeClock::lock` | 136-140 | REAL | — | Poison-tolerant. |
| `WallClock for FakeClock` | 143-155 | REAL | — | sleep advances + records; zero-sleep no-op pinned by test. |
| tests ×4 | 157-215 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fake_clock_sleep_advances_and_records` (163) | YES | Exact arithmetic (500+250+125=875) + schedule vec + zero-sleep no-op. |
| `fake_clock_clones_share_time` (180) | YES | Cross-clone visibility both directions incl. `set_ms`. |
| `system_clock_is_monotonic` (192) | WEAK | Two readings + `>=` — a millisecond-resolution regression inside one call can't be caught, but the property (no panic, no backwards) is pinned. Acceptable for a wall clock. |
| `clocks_are_send_sync` (203) | YES | Real thread boundary crossing with `Arc<dyn WallClock>`; asserts a sane reading. |
| e2e consumers (other files) | YES | net_interrupted + netfault_proxy suites drive reconnect/backoff schedules through FakeClock and assert resume/backoff behavior (baseline green) — the port's contract is load-bearing in those proofs. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "production passes `SystemClock::shared`" (for the retry composition) | schemes.rs:253 | FALSE in practice | No production caller: apps/cli uses `SchemeRegistry::new()` (bare UreqClient, schemes.rs:236-240); `SystemClock::shared()` appears only in reconnect.rs tests + doc comments. |
| "engine code never reads ambient time to make a *decision*" | clock.rs:5-7 | REAL (in-crate) | reconnect takes delays from `plan_retry` (policy), sleeps via the port, re-checks generation after (reconnect.rs:796-812). |
| "Sleep **exactly** `ms` monotonic milliseconds" | clock.rs:51-52 | PARTIAL | F-002: at-least semantics; uninterruptible. |
| "the port itself imposes no additional cap … `sleep_ms(u64::MAX)` is a caller bug" | clock.rs:34-37 | REAL | No cap in the impl (read); callers pass policy-bounded delays. |

## 7. Dead / duplicated / unreachable code
- `SystemClock` as a whole: production-shaped but production-unused (the retry composition it serves is not adopted by any shipped host) — the honest label is "test/e2e-only today".
- `FakeClock::set_ms`: single in-file test caller; near-dead pub test surface.
- No duplication.

## 8. Dependencies & flags
- std only (`OnceLock`, `Mutex`, `Instant`, `thread::sleep`). No unsafe, no feature gates. This file is the dependency-free core the C5 discipline advertises — verified.

## 9. Open questions for the next agent
- For A04/A05 (schemes.rs): the "production passes `SystemClock::shared`" claim (:253) vs the CLI's actual `SchemeRegistry::new()` — is the 6.D2 retry adoption simply not wired into apps/cli yet (making every shipped HTTP fetch retry-less), and does the matrix claim reconnect as more than it ships?
- For A09 (reconnect.rs): with F-002's uninterruptible sleep, what is the worst-case shutdown latency of a `RetryFetch` worker — the backoff ceiling value would quantify it.
- For the buffer.rs owner (A01): `now_ns(clock)` = `now_ms × 1e6` — confirm no consumer needs sub-ms resolution from the wall port (the media clock is separate).
