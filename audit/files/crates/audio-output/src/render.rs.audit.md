# AUDIT: crates/audio-output/src/render.rs
| Field | Value |
|---|---|
| Auditor | W08-A01 |
| Date | 2026-11-23 |
| Lines / functions | 311 / 6 (+4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/host.rs:614,726,824,1274 → NullDevice::render (null.rs:301-306) → render_into (render.rs:89); same in apps/web/src/host.rs:1250 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `cargo test -p audio-output` -> 46 passed / 0 failed (33 unit + 12 integration + 1 doctest); rg `RingRenderer` over repo; sed quote verification |

## 1. One-paragraph verdict
`render_into` is the genuine real-time callback core and it is reachable: the shipped CLI's pump calls it through `NullDevice::render` every render quantum during playback. The logic is correct (aligned-prefix ring read, zero-fill on underrun, counter advance by frames consumed, one underrun event per short callback), the no-blocking contract holds by construction (borrows only, no owned value constructed), and the tests are honest — I ran the whole crate suite: 46/46 pass. The catch for the hostile question "is render.rs the real callback driver?": it drives the **null device only**. No OS audio callback (WASAPI/CoreAudio/ALSA) exists anywhere in the repo to call it — the "device thread" is the CLI's own pump loop, and the rendered samples are discarded. The exported "production" `RingRenderer`/`RenderCallback` shape is dead code: nothing constructs it outside its own unit test.

## 2. What actually works (verified)
— verified by `cargo test -p audio-output` -> "test result: ok. 33 passed" (unit, includes all 4 render.rs tests), "4 passed" (callback_budget), "1 passed" (zero_alloc_null_sink), "4 passed", "3 passed", "1 passed" (doc); exit 0.
— verified by `rg -n "RingRenderer" <repo>` -> only audio-output/src/render.rs (definition+test) and lib.rs:141 (re-export); zero constructors anywhere else.

## 3. Findings
F-001 [S3]
Where: crates/audio-output/src/render.rs:122-154
Code:
```
/// The production [`RenderCallback`]: a shared ring + counters view.
///
/// Constructed at open; moved into (or handed to) whatever drives the
/// device.
```
What is wrong: `RingRenderer` is documented as "The production RenderCallback", but it is constructed nowhere in the workspace except its own unit test (render.rs:297) — `rg` finds zero uses outside audio-output. The `RenderCallback` trait itself has no production caller either: `NullDevice::render` (the shipped driver) calls `render_into` directly (null.rs:301), not the trait. The "production" callback shape is dead code with a lying name.
Why it matters: Checklist 6.5 (names that lie / dead code). A future device-backend author wiring `RingRenderer` per the doc would be building on a shape that has never run once in production.
What correct looks like: Either NullDevice composes RingRenderer, or the doc says "reference shape, unused in v0".
Confidence: HIGH — Verification: read (rg) + ran (tests).

F-002 [S3]
Where: crates/audio-output/src/render.rs:84-88 and 148-153
Code:
```
/// `out.len()` must be a multiple of `channels` — a device buffer is
/// always whole frames; an unaligned tail is left **untouched** (the
/// driver-side wrapper validates and reports; the RT core stays
/// branch-cheap).
```
What is wrong: The claimed "driver-side wrapper validates and reports" exists in exactly one place: `NullDevice::render` (null.rs:298-300, `audio.render_buffer_unaligned`). The pub, doc-named-"production" wrapper `RingRenderer::render` (render.rs:149-153) performs **no** validation — it forwards to `render_into`, which truncates `frames_requested = out.len() / channels` and leaves the unaligned tail untouched: stale garbage in the device buffer, no error, no report. Anyone using the exported production shape gets silent corruption the doc promises cannot happen.
Why it matters: Checklist 6.7 (doc asserts a guard the code does not provide on the path it names "production").
What correct looks like: `RingRenderer::render` validates alignment itself, or the trait method returns a Result.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/audio-output/src/render.rs:26-33
Code:
```
/// counter advance, and (driver-side, outside the callback) zero
/// storage growth across any number of renders. What this v0 gate
/// cannot do is *trap* an allocation the way doc 03 §4.9 c's
```
What is wrong: The no-allocation guarantee of `render_into` is structural argument + op-count ledger only; there is no allocator trap (honestly admitted, `[PROPOSAL: tooling]` owed to testkit since wave 2.C3, still unlanded). Meanwhile the shipped driver allocates a fresh `vec![0.0_f32; RENDER_QUANTUM * channels]` per render round inside its loop (apps/cli/src/host.rs:613) — legal under the contract (outside the callback) but the repo's own "zero heap traffic on the real paths" pitch (null.rs write_bytes) is not matched on the render side of the pump.
Confidence: HIGH — Verification: read (host.rs:613 quote verified by sed).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RenderCallback (trait) | 51-57 | UNREACHABLE | S3 | No production caller; NullDevice bypasses it (F-001) |
| RenderStats (struct) | 60-72 | REAL | — | Plain data ledger |
| RenderStats::fully_served | 76-79 | REAL | — | Trivial predicate |
| render_into | 89-120 | REAL | — | Correct: aligned read, silence fill, counter advance, underrun count; borrows only |
| RingRenderer::channels | 138-140 | UNREACHABLE | — | Dead struct (F-001) |
| RingRenderer::ring | 143-146 | UNREACHABLE | — | Dead struct |
| RingRenderer::render (impl) | 148-153 | UNREACHABLE | S3 | No alignment validation despite doc claim (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| full_read_copies_everything_and_advances_once | YES | Asserts exact copied values, counter=2, underruns=0; NAN-poisoned out catches a missed copy |
| underrun_fills_silence_and_counts_one_event | YES | NAN-poisoned tail must become 0.0; counter/underrun counts asserted exactly |
| ring_segments_stay_within_the_power_of_two_wrap_budget | YES | Sweeps 9 request sizes × arbitrary cursors, asserts segments ≤ 2 and frames_read+silence==requested; a torn or multi-wrap read trips it |
| renderer_trait_callback_produces_the_same_output | YES (weakly) | Only test of RingRenderer; would catch a broken forward, not the missing validation (F-002) |
| tests/callback_budget.rs (integration) | YES | Asserts per-call op budgets + counter deltas across channels 1/2/6 and capacities; ran green |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "takes only borrows … it *cannot* allocate" | render.rs:6-12 | REAL | Signature is `&SpscRing, &SampleCounter, &AtomicU64, &mut [f32]`; returns Copy struct; call graph = ring.read/counter.advance/fetch_add/fill |
| "the driver-side wrapper validates and reports" | render.rs:86-88 | FALSE for RingRenderer, REAL for NullDevice | RingRenderer::render (149-153) has no validation; NullDevice::render (null.rs:298-300) does |
| "One counter advance: every rendered frame (data or silence) was consumed" | render.rs:106-108 | REAL | counter.advance(frames_requested) at 108; tests assert count() growth |
| "silence, never repeat old audio" (doc 03 §4.8 d) | render.rs:37-40 | REAL | out[..].fill(0.0) at 104; underrun test asserts zeros not stale data |

## 7. Dead / duplicated / unreachable code
`RingRenderer` + `RenderCallback` trait: exported (lib.rs:141), never used in production (F-001). `render_into` is the only live symbol — through `NullDevice::render`.

## 8. Dependencies & flags
No feature gates; std-only atomics. Depends on `crate::counter::SampleCounter` and `crate::ring::SpscRing` (both audited reachable). audio-output is a dependency of apps/cli (Cargo.toml:49) and apps/web (Cargo.toml:68) — release-lane included.

## 9. Open questions for the next agent
- apps/cli/src/host.rs:613: per-round `vec![0.0_f32; ...]` allocation in the shipped render driver — a D-1/D-8 style audit of the CLI pump's render loop is owed (belongs to the apps/cli auditor).
- The web lane (apps/web/src/audio/sink.rs) claims to be a "transcription" of this file's semantics in JS (`apps/web/audio-worklet.js`) — verify the JS twin actually matches (silence fill, counter bump) in the web lane's audit.
