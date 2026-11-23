# AUDIT: crates/streaming/src/reconnect.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 1560 / 30 prod items + 20 tests + 6 test helpers |
| File verdict | UNREACHABLE (real, tested code — no shipped entry point composes it) |
| Reachable from shipped entry point? | NO — see F-001 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | `cargo test -p streaming --lib reconnect::` → 20 passed/0 failed (0.00s); `rg` caller hunts (exit 0/1 as noted) |

## 1. One-paragraph verdict
The reconnect engine itself is genuine and hostile-grade: correct bounded exponential ladder with saturating `2^n` (L189-196), true full jitter via a seeded xorshift64\*/Lemire `below()` (L231-253), an honest give-up condition (`attempt >= max_retries`, last underlying error surfaced, not synthesized — L735-748, L797-799), generation-stamped cancellation checked after every inner completion AND after every backoff sleep (L785-811), a lock never held across I/O or sleep (L779-802), and a bounded drop-and-count event hub (L507-515). I ran all 20 unit tests myself: pass. But the module is dead code in the shipped product: the only production HTTP open lane (`apps/cli/src/demux_host.rs:491`) constructs `SchemeRegistry::new()` — the raw ureq client with NO retry wrapper — and `with_retrying_fetch`/`RetryFetch` are referenced only by tests. `lumen open http://…` ships with zero reconnect. The module doc's adoption claim ("the fetch/ranged/HLS paths adopt retry … zero call-site changes") is false for every shipped path.

## 2. What actually works (verified)
— `cargo test -p streaming --lib reconnect::` → `test result: ok. 20 passed; 0 failed; 0 ignored; 194 filtered out; finished in 0.00s`. Tests assert sleeps against a mirrored seeded schedule, event sequences, stats counters, and byte-identical `Range` re-issue — they would fail if the ladder, jitter, give-up, or resume logic broke. Baseline agrees (audit/runs/baseline/cargo-test-workspace.txt:5568-5587).

## 3. Findings
F-001 [S2]
Where: crates/streaming/src/reconnect.rs:13-16 vs apps/cli/src/demux_host.rs:489-495
Code:
```
//! 2. **Executor** ([`RetryFetch`]) — an [`HttpFetch`] wrapper any
//!    transport composes with (the fetch/ranged/HLS paths adopt retry
//!    by *wrapping their transport handle*, zero call-site changes):
```
and the actual shipped call site:
```
    let resource = match fetch {
        Some(fetch) => streaming::ranged::RangedResource::open(fetch, url, config),
        None => streaming::schemes::SchemeRegistry::new().open_http(
```
What is wrong: `rg 'RetryFetch|with_retrying_fetch'` over the whole repo shows the ONLY non-test, non-self references are `schemes.rs:270` (inside `with_retrying_fetch`, itself only called at schemes.rs:958/1015 — tests) and `tests/net_interrupted.rs:37`. The shipped CLI lane builds `SchemeRegistry::new()` → bare `UreqClient` (schemes.rs:236-240). Why it matters: the entire 6.D2 card (bounded retry, resume-from-offset, generation cancellation, `StreamEventHub`, `health_state`) never executes in `lumen-cli` or `desktop`; a user on flaky Wi-Fi gets zero retries. The doc's "paths adopt retry" claim describes a mechanism no shipped caller uses. Feature matrix is honest here (`network-reconnect-buffering` = not_started, docs/feature-matrix.toml:1462) — the in-code doc is not. What correct looks like: `demux_host.rs` open lane uses `SchemeRegistry::with_retrying_fetch(...)` (or RetryFetch wrapping the injected fetch), and the module doc states adoption status truthfully. Confidence: HIGH — Verification: ran (rg hunts + read of both files).
Note: severity held at S2 rather than S0 only because the feature-matrix does not claim shipped reconnect; if any product doc does, this escalates.

F-002 [S3]
Where: crates/streaming/src/reconnect.rs:549-564
Code:
```
    /// Fetches cancelled by a generation bump (late responses
    /// discarded).
    pub superseded: u64,
    /// Events the hub dropped (bounded-memory accounting).
    pub events_dropped: u64,
}
```
What is wrong: `events_dropped` is never written. `rg 'events_dropped'` → only the declaration (line 563). The hub counts drops in its own `Arc<AtomicU64>` (`dropped()`, L519-521) but nothing ever copies that into `stats`. Why it matters: `stats().events_dropped` is a public telemetry surface that reports 0 forever, even while events are being dropped — a lying counter. What correct looks like: `stats()` (or `emit`) folds `hub.dropped()` in. Confidence: HIGH — Verification: ran rg.

F-003 [S3]
Where: crates/streaming/src/reconnect.rs:735-747 with ReconnectState::Steady doc L339-341
Code:
```
    fn plan_retry(&self, attempt: u32, err: &NetworkError) -> Option<u64> {
        if !err.is_retryable() {
            return None;
        }
```
What is wrong: a non-retryable failure (e.g. 404, TLS failure) records NOTHING — state remains `Steady`, whose doc says "the last fetch succeeded first-try". After a hard 404 the wrapper's `state()` claims delivery is working. The comment "the state stays honest" (L734) is wrong for this arm. Test `non_retryable_failure_passes_through` (L1236-1248) bakes the wrong behavior in (`assert_eq!(wrapper.state(), ReconnectState::Steady)` at L1246). Why it matters: `health_state()` consumers get `Steady`/`Buffering` verdicts after a terminal fetch failure. What correct looks like: a `Failed { attempts: 1 }` (or a dedicated `Terminal` state) for non-retryable errors. Confidence: HIGH — Verification: read + ran test.

F-004 [S4]
Where: crates/streaming/src/reconnect.rs:59-60
Code:
```
//! spends a *small, fast* budget (sub-second jittered backoff, default
//! 4 retries ≤ 8 s ceiling) per fetch; the engine's window is the
```
What is wrong: "sub-second jittered backoff" is false as a bound: with defaults the retry-4 ceiling is `min(8000, 500·2^3)=4000 ms`; worst-case total sleep is 500+1000+2000+4000 = 7.5 s. Only the FIRST retry's ceiling is sub-second. Why it matters: budget math elsewhere (engine composition claims) inherits the wrong scale. Confidence: HIGH — Verification: read (ceiling_ms ladder asserted by test at L1044-1051).

F-005 [S4]
Where: crates/streaming/src/reconnect.rs:176-181
Code:
```
    pub fn per_scheme(scheme: &str) -> RetryPolicy {
        match scheme {
            "http" | "https" => RetryPolicy::default(),
            _ => RetryPolicy::default(),
        }
    }
```
What is wrong: both arms return the identical value; the "per-scheme table" is a constant in disguise. Documented as intentional (one honest posture, L167-174), so not fraud — but the match is dead differentiation and `per_scheme` is equivalent to `RetryPolicy::default()`. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: crates/streaming/src/reconnect.rs:114
Code:
```
pub const DEFAULT_EVENT_CAPACITY: usize = 256;
```
What is wrong: never referenced anywhere (`rg DEFAULT_EVENT_CAPACITY` → only this line; the desktop crate's same-named constant is unrelated). Dead constant. Related: doc 07 §config declares `network.reconnect-attempts` (int 0–10, 3) — no Rust code reads that key, and its documented default (3) contradicts `DEFAULT_MAX_RETRIES = 4` (L96) and ceiling 16 (L100). Config declared but never read. Confidence: HIGH — Verification: ran rg (exit 1 for config key).

## 4. Per-function table (production items; tests in §5)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts (6) | 94-114 | REAL | S4 | DEFAULT_EVENT_CAPACITY dead (F-006) |
| RetryPolicy::Default | 135-143 | REAL | — | matches documented posture |
| RetryPolicy::validate | 151-165 | REAL | — | typed refusals, no clamp; tested |
| RetryPolicy::per_scheme | 176-181 | REAL | S4 | constant in disguise (F-005) |
| RetryPolicy::ceiling_ms | 189-196 | REAL | — | saturating pow, cap; correct; tested |
| Jitter::seeded | 222-226 | REAL | — | zero-seed escape from fixed point |
| Jitter::next_u64 | 231-238 | REAL | — | correct xorshift64\* constants |
| Jitter::below | 244-253 | REAL | — | Lemire multiply-high, bound 0/1 handled |
| BackoffSchedule::new/policy/delay_ms | 273-291 | REAL | — | full jitter `uniform(0, ceiling)`; tested |
| GenerationCounter::new/current/bump | 314-328 | REAL | — | Acquire/AcqRel ordering correct |
| ReconnectState (enum) | 337-363 | REAL | S3 | Steady-after-404 dishonesty (F-003) |
| HealthState (enum) + health_state | 375-418 | REAL | — | pure total mapping; full truth table tested |
| StreamEvent (enum) | 432-474 | REAL | — | carries attempts/delay/generation |
| StreamEventHub::with_capacity | 492-501 | REAL | — | sync_channel as documented |
| StreamEventHub::emit | 507-515 | REAL | — | never blocks; drops+counts; tested |
| StreamEventHub::dropped | 519-521 | REAL | — | ok |
| StreamEventHub::forward_signal | 532-541 | REAL | — | Steady forwards nothing; tested |
| ReconnectStats (struct) | 549-564 | REAL | S3 | events_dropped never written (F-002) |
| RetryFetch::new_checked/new | 616-649 | REAL | — | seed = clock.now_ms(); validated policy |
| RetryFetch::with_event_hub/with_generation/generation/policy | 654-680 | REAL | — | builder seams |
| RetryFetch::state/stats | 684-692 | REAL | — | short lock, Copy data |
| RetryFetch::lock_core | 696-700 | REAL | — | poison recovery |
| RetryFetch::record_success | 705-716 | REAL | — | Recovering→Steady laddering tested |
| RetryFetch::record_superseded | 721-728 | REAL | — | voids state, emits Superseded |
| RetryFetch::plan_retry | 735-763 | REAL | S3 | give-up condition correct; non-retryable arm records nothing (F-003) |
| RetryFetch::emit | 766-770 | REAL | — | no hub = no-op |
| HttpFetch::fetch (RetryFetch) | 774-816 | REAL | — | correct loop; checks generation after inner fetch AND after sleep; lock not held across I/O; narrow check-then-return race window (bump between L785 check and L794 return delivers a stale Ok) — inherent to non-atomic check, blast radius one response, S4-level |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| policy_bounds_are_validated | YES | asserts each refusal arm + legal zero |
| per_scheme_policy_is_bounded | weakly | only asserts validity+equality; cannot catch wrong posture (there is none) |
| ceiling_ladder_is_exponential_and_capped | YES | exact ladder values + saturation points |
| jitter_is_deterministic_and_bounded | YES | determinism, divergence, zero-seed escape, bound edges |
| backoff_full_jitter_within_ceiling | YES | strict `<` ceiling per attempt + replay determinism |
| generation_counter_advances | YES | monotonic + shared-clone semantics |
| retry_recovers_after_transient_failures | YES | mirrors seeded schedule; exact event vec; stats; 3 requests |
| steady_after_first_try_success_follows_recovery | YES | state machine edge |
| fresh_fetch_after_failure_resets_state | YES | Failed → new episode |
| retry_exhaustion_surfaces_last_error | YES | attempts=4, retries=3, 4 requests, 3 sleeps, last event ReconnectFailed{4} |
| non_retryable_failure_passes_through | YES (bakes in F-003) | asserts state stays Steady — passes because the defect is the spec |
| resume_from_offset_reissues_identical_range | YES | byte-exact Range echo on retry |
| late_response_discarded | YES | Cancelled{0}, superseded=1, no body |
| cancellation_during_backoff | YES | bump mid-sleep → no second request |
| superseded_failure_discarded | YES | stale error never surfaces |
| hub_is_bounded_and_counts_drops | YES | capacity 2, drop count 2 |
| hub_forwards_buffer_transitions | YES | exact forwarded vec; Steady→false |
| health_state_maps_both_models | YES | full truth table |
| wrapper_works_without_hub | YES | stats/state still queryable |
| default_policy_is_bounded | weakly | constants + one happy fetch on SystemClock |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the fetch/ranged/HLS paths adopt retry by wrapping their transport handle" | reconnect.rs:14-15 | FALSE for shipped paths | F-001: only tests compose RetryFetch |
| "sub-second jittered backoff, default 4 retries ≤ 8 s ceiling" | reconnect.rs:59-60 | FALSE (sub-second part) | ceiling_ms ladder: 500/1000/2000/4000; total ≤7.5 s (F-004) |
| "Exhaustion here surfaces the last underlying error" | reconnect.rs:63-64 | REAL | test retry_exhaustion_surfaces_last_error (ran) |
| `network.reconnect-attempts` (int 0–10, 3) config | docs/07-config-driven-system.md:608 | PAPER-ONLY | rg: key read nowhere; default 3 ≠ code 4 (F-006) |
| matrix row network-reconnect-buffering not_started | docs/feature-matrix.toml:1462 | REAL (honest) | matches F-001 unwired state |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from shipped entry points (F-001). Dead: `DEFAULT_EVENT_CAPACITY` (L114), `ReconnectStats::events_dropped` (L563). No duplication found.

## 8. Dependencies & flags
No feature gates on this module (`crates/streaming/src/lib.rs` exposes it unconditionally — verified by `rg 'reconnect::'` hits in tests without features). Depends only on crate-internal `buffer`, `clock`, `error`, `fetch` — no new external deps.

## 9. Open questions for the next agent
- Does any desktop/tauri lane build a `SchemeRegistry` with retry (grep found none — confirm in desktop audit)?
- When the engine's network lane lands, does it call `health_state`? Currently zero non-test callers.
