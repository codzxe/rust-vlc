# AUDIT: crates/streaming/src/clock.rs
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-15 |
| Lines / functions | 215 / 8 non-test + 4 tests |
| File verdict | REAL implementation; production half (SystemClock) UNREACHABLE — zero production call sites |
| Reachable from shipped entry point? | NO — SystemClock is constructed only in tests/doc comments; the CLI http lane never builds a clock (no retry layer at all) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `cargo test -p streaming --lib clock::` → 4 passed/0 failed (exit 0); rg SystemClock/with_retrying_fetch call sites; sed quote checks |

## 1. One-paragraph verdict
The port itself is correct and small: a two-method `WallClock` trait (monotonic
`now_ms`, bounded `sleep_ms`), an `Instant`-based `SystemClock` (saturating u128→u64
millis clamp, real thread sleep), and a mutex-backed `FakeClock` whose sleep
advances virtual time and records the ledger — the mechanism the
net_interrupted/hls e2e suites rely on, and their virtual-time claims are real (I
ran those suites too). The hostile finding is reachability: NO production code
constructs a `SystemClock`. The two doc comments that say "production passes
`SystemClock::shared`" (schemes.rs:252-253, hls/live.rs:140-141) describe a
production that does not exist — the CLI's shipped http lane
(`SchemeRegistry::new()` → bare `UreqClient`) never wraps in `RetryFetch`, so
neither the clock nor the bounded-retry machinery it paces is reachable from a
shipped entry point; `with_retrying_fetch` has zero non-test callers.

## 2. What actually works (verified)
— verified by `cargo test -p streaming --lib clock::` → 4 passed; 0 failed.
`clocks_are_send_sync` actually moves the shared clock across a thread boundary.
Virtual-time behavior cross-verified by my runs of net_interrupted (3/3) and
hls_master_e2e (3/3), which assert `clock.sleeps()` ledgers.

## 3. Findings
F-001 [S2]
Where: crates/streaming/src/clock.rs:56-72 (SystemClock) + crates/streaming/src/schemes.rs:236-240,252-253
Code:
```
    /// A shared handle (one lazily-created `Arc` per process — the
    /// resource constructors that default to wall time share it).
    pub fn shared() -> Arc<dyn WallClock> {
```
What is wrong: `SystemClock::shared()` has zero production callers (rg: only
schemes.rs/reconnect.rs/hls-live.rs doc comments and test code reference
SystemClock). The shipped http open lane is
`apps/cli/src/demux_host.rs:491 -> SchemeRegistry::new().open_http(...)`, and
`SchemeRegistry::new()` (schemes.rs:236-240) builds `Arc::new(UreqClient::new())`
— a bare transport, NO `RetryFetch` wrapper. `with_retrying_fetch`
(schemes.rs:264) — the only constructor that takes a clock — is called solely
from schemes.rs's own tests (L958, L1015). Consequences: (a) the shipped lane
has no bounded retry/backoff at all (one transient reset = open failure), while
net_interrupted proves RetryFetch resilience nobody ships; (b) schemes.rs's doc
"production passes `SystemClock::shared`" is false — production passes nothing.
Why it matters: users see "network playback" fail on first blip; the tested
resilience story is dead code. What correct looks like: the CLI constructs
`SchemeRegistry::with_retrying_fetch(UreqClient::new(), policy,
SystemClock::shared())`. Confidence: HIGH — Verification: read (full call-chain
rg); not executed (no networked lane in sandbox).
F-002 [S4]
Where: crates/streaming/src/clock.rs:51-53
Code:
```
    /// Sleep **exactly** `ms` monotonic milliseconds (a bounded wait
    /// the caller budgeted; the fake advances virtual time instead).
    fn sleep_ms(&self, ms: u64);
```
What is wrong: `std::thread::sleep` guarantees AT LEAST the duration (OS
scheduling may extend it); "exactly" overstates the contract. FakeClock is
exact; SystemClock is not. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/streaming/src/clock.rs:32-37
Code:
```
//! Every sleep issued through this port is bounded by a caller's
//! explicit budget (a backoff ceiling, a reload interval): the port
//! itself imposes no additional cap, but no unbounded wait may be
//! expressed with it — `sleep_ms(u64::MAX)` is a caller bug.
```
What is wrong: The port documents "no unbounded wait may be expressed" but
enforces nothing — `sleep_ms(u64::MAX)` compiles and (SystemClock) parks the
thread ~584My. The doc admits it is a caller bug, so this is a documentation-of-
limitation, not a defect; a `debug_assert!` or a typed budget would make the
contract executable. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| WallClock (trait) | 46-54 | REAL | S4 | "exactly" overstated (F-002) |
| SystemClock::shared | 61-72 | REAL | S2 | correct OnceLock<Mutex<Option>> lazy; ZERO production callers (F-001) |
| SystemClock::now_ms | 75-82 | REAL | — | u128→u64 saturating clamp correct |
| SystemClock::sleep_ms | 84-88 | REAL | — | zero-guarded real sleep |
| FakeClock::new/set_ms/advance_ms/sleeps | 111-134 | REAL | — | saturating advance; ledger clone |
| FakeClock::lock | 136-140 | REAL | — | poison recovery |
| FakeClock WallClock impl | 143-155 | REAL | — | sleep advances + records; zero no-op both surfaces |
| tests ×4 | 162-214 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fake_clock_sleep_advances_and_records | YES | exact ms arithmetic + ledger |
| fake_clock_clones_share_time | YES | Arc-shared state both directions |
| system_clock_is_monotonic | WEAK | second >= first across an immediate re-read — near-tautological; no elapsed-time or sleep-duration check |
| clocks_are_send_sync | PARTIAL | thread-boundary move proves Send+Sync at runtime; `assert!(reading < u64::MAX)` is nearly vacuous |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "production passes `SystemClock::shared`" | schemes.rs:252-253, live.rs:140-141 | FALSE | rg: no production constructor of SystemClock; CLI lane builds no retry layer (F-001) |
| "the fake advances virtual time instantly" | clock.rs:21-24 | REAL | net_interrupted runs in 0.02s with sleeps ledger [900,400] |
| "Monotonic… never SystemTime" | clock.rs:26-30 | REAL | Instant-based (L76-81) |
| "Sleep exactly `ms`" | clock.rs:51-52 | FALSE for SystemClock | thread::sleep is at-least semantics (F-002) |

## 7. Dead / duplicated / unreachable code
- `SystemClock` + `SystemClock::shared`: production-dead (F-001). Everything
  else is reachable from streaming's (library-only) sessions and tests.

## 8. Dependencies & flags
std only. No features. Used by reconnect.rs, hls/live.rs, buffer.rs,
schemes.rs — all outside the shipped lane except schemes' non-retry path.

## 9. Open questions for the next agent
- Was dropping the retry wrapper from `SchemeRegistry::new()` a deliberate
  latency decision or an unfinished 6.D2 adoption? Check worklog/records for
  "6.D2 adoption point".
- buffer.rs also imports WallClock — is the buffer model likewise unreachably
  paced?
