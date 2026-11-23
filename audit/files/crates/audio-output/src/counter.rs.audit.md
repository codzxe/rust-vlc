# AUDIT: crates/audio-output/src/counter.rs
| Field | Value |
|---|---|
| Auditor | W08-A01 |
| Date | 2026-11-23 |
| Lines / functions | 133 / 7 (+3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `SampleCounter::new` at null.rs:86 (Shared::open) ← `NullBackend::open_stream` ← apps/cli/src/pipeline_host.rs:1599; `advance` ← `render_into` (render.rs:108) ← `NullDevice::render` ← apps/cli/src/host.rs:614 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p audio-output` -> 46 passed / 0 failed (the 3 counter tests included, exit 0); rg callers of `with_base`/`new`/`rate_hz`; sed quote verification |

## 1. One-paragraph verdict
A correct, minimal, genuinely reachable atomic sample counter: one `AtomicU64` of consumed instants, a typed refusal for a zero rate, and the `media_clock::SampleCounterSource` impl the shipped clock actually reads (media-clock/src/clock.rs:678 consumes `sample_rate_hz`). The pause-freeze claim ("frozen by construction") is true — pause simply stops render calls, and only render advances the counter (null.rs:296-306). Tests are honest and pass in my run. Two hygiene defects only: the `advance` doc says "Saturating" while `fetch_add` wraps (harmless at u64 scale, but the stated invariant is not what the code does), and the `with_base` device-switch offset carry has zero production callers — no device switch exists anywhere in the repo.

## 2. What actually works (verified)
— verified by `cargo test -p audio-output` -> 46/46 pass, including zero_rate_is_typed, counter_is_monotonic_and_rate_carrying, offset_carry_never_rewinds_across_devices.
— verified by rg: `SampleCounter::new` ← null.rs:86; `advance` ← render.rs:108 (live render path); `sample_rate_hz` ← media-clock/src/clock.rs:678.

## 3. Findings
F-001 [S4]
Where: crates/audio-output/src/counter.rs:65-70
Code:
```
    /// Advance by the instants the device just consumed (the render
    /// callback's one counter operation). Saturating — u64 instants
    /// at any rate outlive every playback session ever started.
    pub fn advance(&self, instants: u64) {
        let _ = self.count.fetch_add(instants, Ordering::Relaxed);
    }
```
What is wrong: The doc says "Saturating"; `fetch_add` **wraps** on overflow, it does not saturate. At 48 kHz a u64 counter needs ~12 million years to wrap, so the practical claim (outlives every session) is true, but the stated mechanism is false — the invariant held is luck of the width, not saturation.
Why it matters: Checklist 6.7 — a false mechanism in the doc of the clock's single truth source; a future narrowing to u32 (e.g., a wire counter) would inherit a lie.
What correct looks like: "Wrapping (u64 overflow is unreachable at audio rates)" or an actual `fetch_update` saturating loop.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/audio-output/src/counter.rs:52-63
Code:
```
    pub fn with_base(base: u64, rate_hz: u32) -> Result<SampleCounter, EngineError> {
        let counter = SampleCounter::new(rate_hz)?;
        counter.count.store(base, Ordering::Relaxed);
        Ok(counter)
    }
```
What is wrong: `with_base` — the documented device-switch offset carry (doc 03 §4.9 e, "structural and tested") — has no production caller: rg finds only counter.rs's own test and pump.rs:301 (inside `#[cfg(test)]`, pump.rs:243) using it. No code path in the repo ever switches audio devices (no real backends exist), so the never-rewind-across-switches guarantee is a tested-but-never-executed feature.
Why it matters: Checklist 6.5 (dead code awaiting an integration that does not exist); the module doc sells it as a live invariant.
Confidence: HIGH — Verification: read (rg call-chain).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SampleCounter::new | 42-50 | REAL | — | Typed zero-rate refusal (`audio.sample_rate_invalid`) |
| SampleCounter::with_base | 59-63 | REAL / production-dead | S4 | Only test callers (F-002) |
| SampleCounter::advance | 68-70 | REAL | S4 | Wrapping, not "Saturating" (F-001); single RT-path counter op |
| SampleCounter::count | 74-76 | REAL | — | Relaxed load; monotonic by construction |
| SampleCounter::rate_hz | 79-81 | REAL | — | Immutable field |
| SampleCounterSource::sample_count | 85-87 | REAL | — | What media-clock reads |
| SampleCounterSource::sample_rate_hz | 89-91 | REAL | — | Consumed at clock.rs:678 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| zero_rate_is_typed | YES | `.err().must()` fails on Ok; code asserted by string |
| counter_is_monotonic_and_rate_carrying | YES | Exact counts after two advances; trait view asserted |
| offset_carry_never_rewinds_across_devices | YES (narrowly) | Catches with_base storing < base or advance rewinding; cannot catch a wrong base > old.count (no such failure mode exists) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Pause freezes the counter … frozen *by construction*, not by a special case" | counter.rs:13-16 | REAL | No pause branch here; null.rs:296-299 pauses by refusing to render → no advance |
| "Saturating — u64 instants … outlive every playback session" | counter.rs:66-67 | FALSE mechanism / TRUE conclusion | fetch_add wraps (F-001); 2^64/48000s ≈ 12 Myr |
| "the offset-carry invariant, structural and tested" | counter.rs:17-19 | SHALLOW | Tested yes; structural no — zero production callers (F-002) |
| "the device sample counter is the only trusted progress source" (doc 03 §4.7) | counter.rs:2-4 | REAL | CLI host.rs:545/1217 read NullDevice::sample_count as the clock truth |

## 7. Dead / duplicated / unreachable code
`with_base` is production-dead (F-002). No duplication. The rest of the file is live on the shipped null path.

## 8. Dependencies & flags
`media_clock::SampleCounterSource` (the trait the clock consumes), `media_types::EngineError`, `crate::error`. No features, no unsafe, std-only atomics.

## 9. Open questions for the next agent
- When a real backend lands (ADR-005), who calls `with_base` at device switch? Nothing in apps/cli or apps/web plans for it today — the media-clock auditor should confirm the clock re-anchor story (host.rs seek paths) does not silently assume counter continuity that only `with_base` provides.
