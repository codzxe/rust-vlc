# AUDIT: crates/packet-queue/src/limits.rs (whole file, 513 lines)

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 513 / 15 production units + 10 tests |
| File verdict | REAL (executed in the shipped open lane; the strongest small file audited this wave) |
| Reachable from shipped entry point? | YES, EXECUTED. `lumen open` → playback-engine `Pipeline::wire`→`build_limb` (pipeline.rs:1114 `BoundedQueue::new(limits, thresholds)`) with `QueuePlan` defaults built from `QueueLimits::audio_default(1 MiB)` / `video_default(4 MiB)` (pipeline.rs:437-438) → `WatermarkThresholds::from_limits` (queue.rs:262) → `binding_axis` at every `try_push` (queue.rs:401/443/462). The baseline open transcripts' Buffering→Playing transitions are these watermarks' flags driving priming. Suite executed: 10/10 `limits::tests` + the property/backpressure oracle suites pass in `orchrun-test-workspace-nofailfast.log.txt:3534-3543`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read tool over the whole file; `rg -n "binding_axis|admits one|6.FIX" crates/packet-queue/src/queue.rs crates/packet-queue/tests/*.rs` (call sites: try_push/blocking push ×3; the admission oracle property at tests/property.rs:290-293, 389-390; the 6.FIX-pcm regression at tests/backpressure.rs:190); `rg -n "audio_default|video_default|from_limits" crates/playback-engine/src/pipeline.rs crates/packet-queue/src/queue.rs` (reachability chain); `grep -n "limits::tests::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (10/10 ok) |

## 1. One-paragraph verdict
This is the standard the rest of the engine should be held to: boundedness is enforced at the type level (NonZeroUsize caps, typed refusal of Unknown/zero durations — an unbounded queue is unconstructible), the one impossible const arm is a compile-time `loop {}` with a scoped, justified lint allow and a comment explaining why it beats the `unreachable!` it replaced, the livelock invariant (an empty queue always admits one item, whatever its weight — the measured 6.FIX-pcm wav failure) is documented with its measured numbers and verified by an independent property oracle, the huge-weight check is genuinely overflow-proof on two of three axes, and the 10 unit tests plus the property suite assert exact boundaries, not shapes. Two S4s only: the packets axis does its addition in `usize` *before* the `u128` cast its own doc advertises (bytes/duration do it right), and the quarter-floored watermark defaults make `high` and `low` simultaneously true for a degenerate cap-of-1 configuration. Neither is reachable through shipped defaults.

## 2. What actually works (verified)
— Typed unboundedness refusal — verified: `QueueLimits::new` rejects `Duration::Unknown` and zero with distinct typed errors (154-162), tests pin both messages; packet/byte caps cannot be zero at all (`NonZeroUsize`).
— The const `cap` trick — verified: `None => loop {}` in const position is a build failure (never yields), all three call sites are `const` items (40/43/47), the fn is private, and the `#[allow(clippy::empty_loop)]` is scoped with the false-positive rationale — the documented improvement over the `unreachable!` it replaced.
— Single-item admission (6.FIX-pcm) — verified: `binding_axis` short-circuits `QueueUsage::EMPTY` (237-239); the invariant is independently oracle-tested (tests/property.rs:290-293 "a queue with nothing resident admits one item unconditionally", 389-390) and regression-tested (tests/backpressure.rs:190, "the old absolute refusal stalled real lanes"); the pipeline-side `producer_parked` escape (pipeline.rs:1992, my part3 file) composes with it coherently.
— u128 discipline on bytes/duration — verified: `resident.bytes as u128 + add_bytes as u128` (241) and the duration pair (242) cannot wrap; test 443-454 drives `usize::MAX`/`u64::MAX` weights through it.
— Watermark predicate — verified: `high` = any-axis at/above, `low` = all-axes at/below (324-329), exactly the documented eager/conservative pair; test 469-506 pins all four quadrants including the neither-flag middle case.
— Defaults match doc 03 §4.4(b) — verified: audio 1 s/200, video 2 s/300, subtitle 10 s/128, all test-pinned (377-399).

## 3. Findings

### F-001 [S4]
Where: crates/packet-queue/src/limits.rs:240-242
Code:
```rust
        let packets = (resident.packets + add_packets) as u128;
        let bytes = resident.bytes as u128 + add_bytes as u128;
        let duration = resident.duration.0 as u128 + add_duration.0 as u128;
```
What is wrong: The packets line adds in `usize` and casts the SUM; the bytes and duration lines cast the operands and add in `u128`. The method's own doc (224-226) says "The byte and duration sums are computed in `u128` so the *check* can never overflow even for adversarial weights" — the packets axis is silently outside that guarantee: `resident.packets + add_packets` overflows usize before the cast if `resident.packets` is near `usize::MAX` (debug panic, release wrap).
Why it matters: Inconsistent discipline inside three adjacent lines that exist to make the same promise; a future caller pushing a batch (`add_packets > 1`) against a pathologically large configured cap could trip it. Practically unreachable today (all three call sites pass `add_packets = 1` — queue.rs:401/443/462 — and `resident.packets` is bounded by a configured cap no real host sets near 2^64), hence S4.
What correct looks like: `let packets = resident.packets as u128 + add_packets as u128;` — the same shape as the line below it; add the packets case to the `usize::MAX` test.
Confidence: HIGH — Verification: read (all three call sites enumerated; overflow semantics of the cast-after-add shape).

### F-002 [S4]
Where: crates/packet-queue/src/limits.rs:289-303 (from_limits), 323-331 (evaluate)
Code:
```rust
        let quarter = |cap_value: u64| (cap_value / 4).max(1);
```
and the consumer (evaluate, 324-329):
```rust
        let high = usage.packets >= marks.high.packets
            || usage.bytes >= marks.high.bytes
            || usage.duration.0 >= marks.high.duration.0;
        let low = usage.packets <= marks.low.packets
            && usage.bytes <= marks.low.bytes
            && usage.duration.0 <= marks.low.duration.0;
```
What is wrong: For a cap of 1 on any axis (e.g. `max_packets = 1`), `quarter(1) = max(0,1) = 1` — the low threshold equals the high threshold equals the cap. A queue with its single item resident then evaluates `high == true` AND `low == true` simultaneously: the queue is "full enough to prime" and "running dry" in the same observation. Downstream, pipeline priming wants `high` while the starvation debounce counts `low` streaks — contradictory signals for a degenerate-but-constructible config (the typed constructor happily accepts `max_packets = 1`).
Why it matters: The floor-at-one rule (correctly) keeps the low signal usable for tiny caps, but nobody considered that at cap ≤ 1 the two predicates overlap; shipped defaults (200/300 packets, 1 s/2 s durations) never hit it, and no test constructs a cap-1 limits set. Latent config edge only.
What correct looks like: `low = min(quarter, cap - 1)` per axis (a low threshold strictly below the high threshold), or document that caps < 4 axes degenerate and let `evaluate`'s flags be consumed with that caveat; add a cap-1 test asserting `!(high && low)` per usage.
Confidence: HIGH — Verification: read (arithmetic traced for cap values 1/2/3/4; predicate truth table enumerated; shipped defaults confirmed non-overlapping).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `cap` (const fn) | 30-36 | REAL | — | Const-position build-failure trick; private; all call sites const; scoped allow justified. |
| `AUDIO/VIDEO/SUBTITLE_DEFAULT_PACKETS` | 40/43/47 | REAL | — | 200/300/128, doc-cited, const-evaluated. |
| `LimitAxis` + `Display` | 50-68 | REAL | — | Three-variant enum; consumed by pipeline's starvation diagnostics (pipeline.rs:2206-2218, my part3 context). |
| `QueueUsage` + `EMPTY` | 76-93 | REAL | — | Copy struct; sums saturate upstream (queue.rs keeps them ≤ caps). |
| `QueueLimits` (struct) | 102-110 | REAL | — | All three caps present; Copy. |
| `QueueConfigError` + `Display` + `Error` | 114-138 | REAL | — | Two typed refusals; messages test-pinned. |
| `QueueLimits::new` | 149-163 | REAL | — | The typed boundary; tests 343-374. |
| `QueueLimits::audio_default` | 168-175 | REAL | — | 1 s/200 + caller bytes; EXECUTED (pipeline.rs:437). |
| `QueueLimits::video_default` | 180-187 | REAL | — | 2 s/300; EXECUTED (pipeline.rs:438). |
| `QueueLimits::subtitle_default` | 194-201 | REAL | — | 10 s/128; wired by switch paths (engine-API-only today). |
| `QueueLimits::binding_axis` | 227-252 | REAL | S4 (F-001) | Single-item admission + first-violated-axis; u128 discipline missing on the packets line; EXECUTED at every push. |
| `WatermarkThresholds` + `new` | 268-281 | REAL | — | Plain carrier. |
| `WatermarkThresholds::from_limits` | 289-303 | REAL | S4 (F-002) | Quarter-floored defaults; overlap at cap 1. |
| `Watermark` + `evaluate` | 310-331 | REAL | — | The doc's eager/conservative pair, exactly; four-quadrant test. |
| `mod tests` (10 tests) | 334-513 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `limits_accept_known_positive_duration` / `limits_reject_unknown_duration` / `limits_reject_zero_duration` (343-374) | YES | Exact typed-error and message assertions. |
| `audio_default_matches_doc03` / `video_default_matches_doc03` / `subtitle_default_is_small_and_duration_led` (377-399) | YES | Literal equality with the doc figures. |
| `binding_axis_reports_the_first_violated_cap` (402-440) | YES | Each axis isolated; the fits case pins the boundary (exactly-at-cap admits). |
| `binding_axis_checks_in_u128_so_huge_weights_cannot_wrap` (443-454) | PARTIALLY | Drives `usize::MAX` bytes and `u64::MAX` duration — but not a `usize::MAX` packets resident, so F-001's shape is untested (the packets line would wrap before the cast). |
| `watermark_default_high_is_capacity_low_is_a_quarter` (457-466) / `watermark_flags_follow_the_predicate` (469-506) | YES | Exact thresholds and all four quadrants incl. the neither-flag middle. |
| `config_error_is_a_std_error` (509-512) | YES (trivially) | Trait coherence. |
| Property/backpressure suites (other files) | YES | The admission oracle (property.rs:290-293/389-390) is an independent check of this file's invariant; 6.FIX-pcm regression at backpressure.rs:190. All passing in the no-fail-fast baseline. |
| Coverage gaps | — | No cap-1 watermark test (F-002); no `usize::MAX`-packets binding_axis case (F-001); the single-item admission's own unit test lives in property.rs, not here (fine, but a reader of this file alone would not know it's oracle-checked — the doc comment does say so). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "An unbounded queue is unconstructible" | limits.rs:5-9, 98 | REAL | NonZeroUsize caps + typed duration refusals; tests 343-374 |
| "whichever hits first refuses the next push (invariant Q1)" | limits.rs:3-5 | REAL | binding_axis first-violated ordering + try_push consumers |
| "a queue with nothing resident always admits one item, whatever its weight" (6.FIX-pcm) | limits.rs:206-222 | REAL | 237-239 short-circuit + property oracle + backpressure regression |
| "The byte and duration sums are computed in `u128` so the check can never overflow" | limits.rs:224-226 | SHALLOW | True for bytes/duration (241-242); the packets line (240) adds in usize first (F-001) — the doc's guarantee silently excludes the third axis |
| "`high` … at or above … on any axis; `low` … at or below … on all axes" | limits.rs:261-267 | REAL | evaluate 324-329; four-quadrant test |
| "the quarter is this crate's default; doc 03 assigns the real policy to product.toml" | limits.rs:286-287 | REAL | `[PROPOSAL]`-tagged; from_product_limits (pipeline.rs:325) is the product-facing seam |
| "usage ≤ cap + one item's weight" (admission boundedness) | limits.rs:215-216 | REAL | The lone item is admitted once; subsequent pushes see non-empty resident and are refused on the violated axis (traced) |

## 7. Dead / duplicated / unreachable code
None. Every item is consumed (caps→defaults→pipeline; LimitAxis→starvation diagnostics; QueueConfigError→QueuePlan::from_product_limits validation path).

## 8. Dependencies & flags
`media_types::time` (Duration/Nanoseconds) and `std::num::NonZeroUsize` only — the crate's L0 charter intact. No cfg, no unsafe, no feature gates. One scoped `#[allow(clippy::empty_loop)]` with justification (the only `#[allow]` in the file).

## 9. Open questions for the next agent
1. Queue owner (queue.rs auditor): F-001's fix belongs with a `usize::MAX`-packets test case; consider asserting `add_packets == 1` at the call sites or widening the signature to u128 if batch pushes ever appear.
2. Product-config auditor: the `[PROPOSAL: config in product.toml]` notes (100-101, 286-287) — has any wave verified whether product.toml actually carries queue limits and `from_product_limits` reads them, or do all shipped hosts ride the hard-coded defaults (pipeline.rs:437-438 suggests the latter)?
3. Whoever owns watermark consumption: confirm no shipped path can construct a cap-of-1 axis (F-002's premise) — the byte budget comes from the host (`nonzero(one_mib)`), so a host passing `NonZeroUsize::new(1)` byte budget with a >1-byte packet is the nearest miss (the admission invariant would then keep exactly one resident packet at all times, permanently `high`+`low`).
