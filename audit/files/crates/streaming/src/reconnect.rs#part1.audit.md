# AUDIT: crates/streaming/src/reconnect.rs#part1 (lines 1-520, extended to 521 to close `StreamEventHub::dropped`)
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-21 |
| Chunk boundary | part1 = lines 1-521, per wave-00 assignments.tsv LOC split (part1=520 → this audit: module docs, policy/jitter/schedule, generation counter, state/health/event seam through `StreamEventHub::dropped`'s body; extended 520→521 to close the function — `forward_signal` at 523+ belongs to part2=A09, part3=A03 has the executor's `impl HttpFetch` tail + tests from 1041). The `#[cfg(test)] mod tests` starts at 819 — in part2/part3 territory; my functions' tests cited from baseline, their bodies audited by A09/A03. |
| Lines / functions | 521 / 26 (6 consts + 1 struct + 5 RetryPolicy items + 4 Jitter items + 3 BackoffSchedule items + 4 GenerationCounter items + 2 enums + health_state + StreamEvent enum + 4 StreamEventHub items) |
| File verdict | UNREACHABLE (real, tested library code; no shipped composition instantiates any of it) |
| Reachable from shipped entry point? | NO. Chain of proof: `lumen` CLI's only http lane is `open_http_source` (apps/cli/src/demux_host.rs:488-495) → `SchemeRegistry::new()` (schemes.rs:236-240: `fetch: Arc::new(UreqClient::new())` — bare client, NO RetryFetch) → `RangedResource::open`. `rg "with_retrying_fetch\|RetryFetch::new"` → callers are schemes.rs:270 (the `with_retrying_fetch` ctor) and tests only; `SchemeRegistry::with_retrying_fetch` itself is called only in schemes.rs tests (958, 1015). HLS/DASH construct no RetryFetch. The seam half (`health_state`, `StreamEventHub`, `ReconnectState`, `GenerationCounter`, `forward_signal`) has ZERO consumers outside reconnect.rs (rg across crates/ apps/ — one doc mention in transcode/limits.rs:58). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "with_retrying_fetch\|RetryFetch" ` (production never wraps) · `rg -n "health_state\|StreamEventHub\|ReconnectState\|GenerationCounter\|BackoffSchedule\|forward_signal"` (seam consumed nowhere) · `rg -n "DEFAULT_EVENT_CAPACITY"` (dead in this crate; the desktop one at apps/desktop/src/shell.rs:57 is an unrelated `NonZeroUsize`) · `sed -n '233,240p' crates/streaming/src/schemes.rs` (production ctor bare) · baseline `orchrun-test-workspace-nofailfast.log.txt:5110-5129` (all 20 reconnect::tests ok) · feature-matrix row `network-reconnect-buffering` (docs/feature-matrix.toml:1457-1467: status not_started ×4, evidence "") |

## 1. One-paragraph verdict
The policy math is genuinely correct — I verified the exponential ladder's saturation guard, the xorshift64\* triple (12/25/27 + star multiply, zero-seed fixed point handled), the Lemire multiply-high bound, the full-jitter draw, and the health-state mapping against its documented table, and all 20 module tests ran green in the baseline run. But the entire 1,560-line card-6.D2 machinery — bounded retry, backoff, resume-from-offset, generation-safe cancellation, the event hub, the "engine-facing seam" — is dead code in the shipped product: the CLI's only HTTP path constructs a bare `UreqClient` through `SchemeRegistry::new()` and never wraps it, `with_retrying_fetch` has no production caller, and not one of the seam types (`health_state`, `StreamEventHub`, `ReconnectState`) is read by any code outside this file. The module doc's "v0 ships both layers on" is false of the shipped binary; to its credit, the feature matrix honestly marks `network-reconnect-buffering` `not_started` with empty evidence — so this is drift, not matrix fraud.

## 2. What actually works (verified)
— All 20 `reconnect::tests` green in baseline `orchrun-test-workspace-nofailfast.log.txt:5110-5129` (release profile), including the tests that pin my chunk's functions: `ceiling_ladder_is_exponential_and_capped`, `jitter_is_deterministic_and_bounded`, `backoff_full_jitter_within_ceiling`, `generation_counter_advances`, `health_state_maps_both_models`, `hub_is_bounded_and_counts_drops`, `policy_bounds_are_validated`, `per_scheme_policy_is_bounded`, `default_policy_is_bounded`.
— Hand-verified `ceiling_ms` (189-196): `attempt >= 64` short-circuits to `max_ms` before the pow; `2u64.saturating_pow(attempt)` cannot overflow for attempt < 64; `.min(self.max_ms)` enforces the ceiling — the ladder is monotone and bounded exactly as documented.
— Hand-verified the xorshift64\* state update (231-238): the (12, 25, 27) shift triple with the `0x2545_F491_4F6C_DD1D` star multiplier is the canonical Marsaglia sequence; each step is a bijection on nonzero states, so a nonzero seed never degenerates to the zero fixed point; `seeded(0)` remaps to the documented constant (216, 222-226).
— Hand-verified `below` (244-253): the 128-bit multiply-high `(raw as u128 * bound) >> 64` maps a uniform 2^64 draw to `0..bound` with bias ≤ 1 interval — the comment's "uniform" is exact enough for backoff jitter.
— `health_state` (398-418) matches its recorded engine-mapping table row-for-row, including the terminal-`Failed` override and the initial-fill/`Rebuffering` split.

## 3. Findings

### F-001 [S2]
Where: crates/streaming/src/reconnect.rs:68-72 (module doc) vs apps/cli/src/demux_host.rs:491 and crates/streaming/src/schemes.rs:236-240
Code:
```
//! side (single-owner mode) if measurement shows the product budget
//! double-spends; v0 ships both layers on, each honest about its own
//! budget.]
```
```
    pub fn new() -> SchemeRegistry {
        SchemeRegistry {
            fetch: Arc::new(UreqClient::new()),
        }
    }
```
What is wrong: the module doc asserts the fetch-level retry layer and the engine-level recovery window both ship ON ("v0 ships both layers on"). In the shipped binary the fetch-level layer is OFF: the only production HTTP composition is `SchemeRegistry::new()` wrapping a bare `UreqClient` (demux_host.rs:491), and `with_retrying_fetch`/`RetryFetch::new_checked` have zero production callers (rg: schemes.rs tests only). A user playing `lumen open http://…` through a flaky connection gets NO transport-level retry, despite 1,560 lines of bounded-reconnect machinery compiled into the library.
Why it matters: the doc misstates shipped behavior; the natural reading ("both layers on") is what a maintainer would rely on when triaging a dropouts bug. The feature matrix is honest (`network-reconnect-buffering` = not_started ×4, evidence ""), so this is internal doc drift, not external fraud — but the user-visible consequence (no reconnect anywhere in the product) is real.
What correct looks like: either the CLI's `open_http_source` composes `SchemeRegistry::with_retrying_fetch(Arc::new(UreqClient::new()), RetryPolicy::per_scheme(scheme), SystemClock::shared())`, or the module doc states plainly that no shipped composition enables the layer and the matrix's not_started is the operative truth.
Confidence: HIGH — Verification: read + rg (call graph traced end to end; no cargo run needed to prove absence of callers).

### F-002 [S3]
Where: crates/streaming/src/reconnect.rs:175-181
Code:
```
    pub fn per_scheme(scheme: &str) -> RetryPolicy {
        match scheme {
            "http" | "https" => RetryPolicy::default(),
            _ => RetryPolicy::default(),
        }
    }
```
What is wrong: the "per-scheme table" is a no-op — every arm returns the same `RetryPolicy::default()`, and the parameter is never consulted. The doc (167-174) admits v0 ships one posture, but the function name and signature promise per-scheme behavior that does not exist; clippy cannot flag it because the match is exhaustive and the arms differ syntactically.
Why it matters: a caller reading the signature reasonably believes scheme divergence is implemented; the `per_scheme_policy_is_bounded` test (baseline 5122) passes trivially against the no-op, pinning nothing about per-scheme behavior.
What correct looks like: v0 should return `RetryPolicy::default()` unconditionally with the seam documented, or the match arms should carry the (currently identical) policy constants explicitly so divergence has a landing place that type-checks differently.
Confidence: HIGH — Verification: read (both arms traced; no other definition).

### F-003 [S4]
Where: crates/streaming/src/reconnect.rs:398-418, 432-474, 476-521
Code:
```
pub fn health_state(
    level: BufferLevel,
    initial_filled: bool,
    reconnect: ReconnectState,
) -> HealthState {
```
What is wrong: the entire "engine-facing seam" — `health_state`, `HealthState`, `StreamEvent`, `StreamEventHub` — is consumed by no code outside this module (rg across crates/ apps/: zero call sites; the engine's recovery.rs in playback-engine references none of it). The doc's "the bridge that lands with the engine's network lane" (425) never landed on the engine side.
Why it matters: dead public API whose doc claims an engine integration that does not exist; the HealthState table (369-374) maps to engine surfaces that will never receive these verdicts as long as nothing queries them.
What correct looks like: either the engine's network lane consumes the seam, or the docs mark the seam as unconsumed (the matrix already says not_started — the module doc should match).
Confidence: HIGH — Verification: rg (zero external consumers).

### F-004 [S4]
Where: crates/streaming/src/reconnect.rs:112-114
Code:
```
/// Default bounded event-hub capacity: one drain interval's worth of
/// transitions (mirrors the engine's outbox sizing posture).
pub const DEFAULT_EVENT_CAPACITY: usize = 256;
```
What is wrong: dead constant — `rg DEFAULT_EVENT_CAPACITY` shows no user in this crate (the identically-named desktop constant at apps/desktop/src/shell.rs:57 is a different, unrelated `NonZeroUsize`). `StreamEventHub::with_capacity` takes its capacity as a parameter and no caller passes this default.
Why it matters: dead public API; the doc's "mirrors the engine's outbox sizing posture" is unverifiable and the value wires nothing.
What correct looks like: use it as the default in the `with_event_hub` constructor (part2) or delete it.
Confidence: HIGH — Verification: rg (no users).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module constants (6) | 94-114 | REAL | S4 (F-004 for EVENT_CAPACITY) | retry/backoff constants used by Default/validate; EVENT_CAPACITY dead. |
| `RetryPolicy` (struct) | 125-133 | REAL | — | Copy data. |
| `RetryPolicy::default` | 135-143 | REAL | — | 4 retries, 500 ms base, 8 s cap — matches doc §"sub-second jittered backoff, default 4 retries ≤ 8 s". |
| `RetryPolicy::validate` | 151-165 | REAL | — | all four invariants; typed refusal; green in baseline (`policy_bounds_are_validated`). |
| `RetryPolicy::per_scheme` | 176-181 | REAL (no-op) | S3 (F-002) | ignores its argument. |
| `RetryPolicy::ceiling_ms` | 189-196 | REAL | — | saturation-guarded ladder; hand-verified. |
| `Jitter` + `JITTER_ZERO_SEED` | 208-216 | REAL | — | canonical xorshift64\*. |
| `Jitter::seeded` | 222-226 | REAL | — | zero-seed remap. |
| `Jitter::next_u64` | 231-238 | REAL | — | correct triple + star multiply. |
| `Jitter::below` | 244-253 | REAL | — | Lemire multiply-high; bound ≤ 1 → 0. |
| `BackoffSchedule::new` / `policy` / `delay_ms` | 273-291 | REAL | — | full-jitter draw under the ladder; green in baseline (`backoff_full_jitter_within_ceiling`). |
| `GenerationCounter::new` / `current` / `bump` | 314-328 | REAL | — | Acquire/AcqRel ordering sound; monotonic; green in baseline (`generation_counter_advances`). |
| `ReconnectState` (enum) | 337-363 | REAL | — | Copy state machine vocabulary. |
| `HealthState` (enum) | 375-391 | REAL | — | unconsumed seam (F-003). |
| `health_state` | 398-418 | REAL | — | matches doc table; green in baseline (`health_state_maps_both_models`). |
| `StreamEvent` (enum) | 432-474 | REAL | — | 7 event kinds; `Buffering.progress` f32 0..=1. |
| `StreamEventHub::with_capacity` | 492-501 | REAL | — | sync_channel; capacity 0 = rendezvous (documented). |
| `StreamEventHub::emit` | 507-515 | REAL | — | never blocks; Full/Disconnected counted; green in baseline (`hub_is_bounded_and_counts_drops`). |
| `StreamEventHub::dropped` | 518-521 | REAL | — | Relaxed load of a monotone counter. |

## 5. Test honesty
Tests live in part2/part3 (mod tests at 819) — owned by A09/A03. Baseline citation for my functions' coverage: all 20 `reconnect::tests` green at `:5110-5129`. The one honesty caveat in range: `per_scheme_policy_is_bounded` cannot fail against a no-op table (F-002) — it pins boundedness, not per-scheme behavior.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "v0 ships both layers on, each honest about its own budget" | reconnect.rs:68-72 | FALSE (of the shipped binary) | production composition is bare `UreqClient` (schemes.rs:236-240, demux_host.rs:491); no RetryFetch caller — F-001. |
| "the fetch/ranged/HLS paths adopt retry by wrapping their transport handle, zero call-site changes" | reconnect.rs:13-15 | SHALLOW | the wrapper exists and works (tests), but no shipped path wraps — F-001. |
| "the bridge that lands with the engine's network lane" (event seam) | reconnect.rs:424-425 | PAPER-ONLY | zero engine-side consumers (rg) — F-003. |
| `network-reconnect-buffering` status | docs/feature-matrix.toml:1457-1467 | REAL (honest) | not_started ×4, evidence "" — matches shipped reality; the code exists beyond the matrix's claim but nothing enables it. |
| Exponential/full-jitter/backoff math (module doc §1) | reconnect.rs:7-12 | REAL | hand-verified + baseline tests green. |

## 7. Dead / duplicated / unreachable code
Everything in this chunk is production-unreachable (see header chain). Within the module: `DEFAULT_EVENT_CAPACITY` dead (F-004); `per_scheme`'s parameter dead (F-002); the seam types externally unconsumed (F-003). No duplication found.

## 8. Dependencies & flags
Depends on `crate::buffer` (BufferLevel/BufferSignal — itself audited this wave by A01/A05), `crate::clock::WallClock`, `crate::error::NetworkError`, `crate::fetch` port. No feature gates; no unsafe; std-only (mpsc sync_channel). All 20 module tests green in baseline.

## 9. Open questions for the next agent
- A09 (part2: `RetryFetch` core + `forward_signal`): confirm the lock is truly never held across a sleep/inner fetch (module doc 79-80 promises it) and whether `with_event_hub` defaults to `DEFAULT_EVENT_CAPACITY` (F-004's fix site).
- A03 (part3: `impl HttpFetch for RetryFetch`): verify the generation re-check happens after EVERY inner completion AND after every sleep, and that exhaustion surfaces the last underlying error (doc 63-65).
- Cross-wave: if the orchestrator wants reconnect live, the one-line composition site is demux_host.rs:491 — everything else is already built and tested.
