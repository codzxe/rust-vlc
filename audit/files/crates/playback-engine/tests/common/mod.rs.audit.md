# AUDIT: crates/playback-engine/tests/common/mod.rs
| Field | Value |
|---|---|
| Auditor | W03-A10 |
| Date | 2026-11-23 |
| Lines / functions | 458 / 24 (fakes, builders, drivers; no tests of its own — a `tests/common` helper module) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test-support module). Included by 5 test binaries (`mod common;` in session_flow.rs, model_walk.rs, hotplug.rs:46, recovery_env.rs:112, reopen_alloc.rs:19), all green in baseline (cargo-test-workspace.txt:4251-4445). Ran one consumer live: `./target/debug/deps/session_flow-a00fa5b050666f17` -> 34 passed, 0 failed (exit 0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | direct prebuilt run of session_flow consumer -> 34 passed (exit 0); `rg` consumer/wiring verifications |

## 1. One-paragraph verdict
A clean shared test-support module: fakes for the demux/decode seams with the honest shapes (scripted events, mid-run stall release via a shared flag, one-shot armed failures, the codec-api stale-packet guard honored), builders wiring the real `Engine` (never a mock of it), a frozen virtual clock and frozen event throttle for determinism, and a bounded hang-proof `poll_until`. The `#![allow(dead_code)]` is the standard tests/common discipline and is documented. The only defect is a stale header that names 2 of the 5 consuming binaries.

## 2. What actually works (verified)
— verified by running a consumer live: `./target/debug/deps/session_flow-a00fa5b050666f17` -> `test result: ok. 34 passed; 0 failed` (exit 0) — the harness drives the real engine end-to-end (open→play→pause→seek-shapes→end, per that binary's test names).
— verified wiring: `mod common;` present in session_flow.rs, model_walk.rs, hotplug.rs:46, recovery_env.rs:112, reopen_alloc.rs:19; all five binaries in the baseline workspace run (lines 4251, 4276, 4320, 4360, 4442) with ok results.

## 3. Findings
F-001 [S4]
Where: crates/playback-engine/tests/common/mod.rs:1-5
Code:
```
//! The shared engine-test harness (card 3.C1, hoisted verbatim from
//! `tests/session_flow.rs` — the design's step 1: the fake stage
//! objects, builders, and poll drivers the session suite and the
//! model-walk suite both drive the real [`Engine`] with. No behavior
//! change; `session_flow.rs` now consumes it as `mod common`).
```
What is wrong: The header names two consumers (session suite, model-walk suite); the module is actually included by five test binaries (session_flow, model_walk, hotplug, recovery_env, reopen_alloc — rg-verified). A reader auditing the blast radius of a harness change here would miss three consumers.
Why it matters: Shared-harness changes ripple into all five; the doc understates the surface.
What correct looks like: List all five (or none — "the engine-test binaries").
Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SharedNow / impl MonotonicSource | 55-61 | REAL | — | Virtual clock source; frozen at 0 in every builder (see §7). |
| Collector / impl FrameConsumer | 65-78 | REAL | — | Retaining sink — correct here (these suites assert on delivered frames; contrast replacement_alloc's non-retaining sink for memory measurement). |
| FakeDemuxer (start/next_event/seek) | 86-129 | REAL | — | Script-driven; exhausted script → stall flag → Discontinuity (documented honest non-blocking shape) else EndOfStream; unseekable base fake with typed refusal; seekable variants live in seek_flow.rs per the comment. |
| FakeDecoder | 133-162 | REAL | — | One-shot armed failure, stale-generation guard, one frame per packet. |
| FakeProvider | 166-184 | REAL | — | Armed selection failure (the registry-lied case) via shallow_clone. |
| shallow_clone | 188-196 | REAL | — | Drops the boxed source chain — documented reason (the engine never clones errors; the fake needs per-call copies). |
| base / at_micros / track / packet / audio_frame | 202-253 | REAL | — | 1 ms-stride packets, µs-base timestamps; static payload bytes. |
| inaccessible / decoder_error | 255-273 | REAL | — | Typed error constructors for the failure scenarios. |
| tiny_queues / tiny_budget | 278-293 | REAL | — | 16-packet queues, 64/4/8 budget; the comment's watermark math (high 12, low 4) is the consumers' premise. |
| virtual_clock / frozen_throttle | 295-315 | REAL | — | Two SEPARATE frozen SharedNow instances (clock and throttle) — sound because nothing ever advances either; see §7. |
| config_with | 317-341 | REAL | — | Full-control builder (used by model_walk's custom shapes). |
| Fixture / open_with | 343-389 | REAL | — | Wires the real Engine::open; script/stall shared out so tests can release stalls mid-run. |
| open_audio / open_stalled_audio | 391-411 | REAL | — | The underrun shape: preload then stall, stream not ended. |
| poll_until | 416-433 | REAL | — | Bounded, loud failure naming state and limit; predicate checked pre-poll (zero-poll fast path). |
| take_events / frames_of / receipt_for | 435-458 | REAL | — | Accessors; `receipt_for` .must() panics on a missing receipt (loud). |

## 5. Test honesty
No tests in this file — it is the harness its consumers' honesty rests on. The load-bearing properties, verified by reading:
- The fakes implement the plugin-api/codec-api contracts with the failure shapes the engine must survive (start failure, select failure, decode failure, stall/discontinuity, stale generation) — consumers can force every error lane.
- `poll_until` cannot hang silently (bounded loop + assert with state text).
- The throttle freeze is documented as *permitting exactly one in-episode buffering emission* — the determinism premise behind consumers' exact-sequence asserts.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "hoisted verbatim from tests/session_flow.rs … No behavior change" | lines 1-5 | REAL-UNPROVEN (the verbatim/no-change part) | Hoist history is git territory (forbidden to me); current shape is consistent with a hoist, and all consumers pass. |
| "the session suite and the model-walk suite both drive the real Engine" | lines 3-4 | REAL but INCOMPLETE | True for those two + three more (F-001). |
| "an armed stall answers discontinuity markers — the honest non-blocking shape" | lines 82-85 | REAL | Code at 110-116; stall recovery/counting covered by recovery_env.rs (21 tests, baseline ok). |
| "a never-advancing source permits exactly one in-episode buffering emission" | lines 306-309 | REAL | The events_contract.rs happy-path test (audited earlier today) independently pins exactly that emission pattern (0.0, climb, 1.0) under a frozen clock. |

## 7. Dead / duplicated / unreachable code
- `#![allow(dead_code)]` — documented tests/common discipline (each consumer uses a subset). Bounded and standard; noted, not counted.
- Duplication with events_contract.rs's own fakes (SharedNow / FakeDecoder shapes) — deliberate per-binary independence (the contract suite built its own tighter harness); cosmetic.
- The clock and throttle use two distinct frozen `SharedNow` instances — neither Arc is exposed by `Fixture`, so no consumer can advance either; sound today, but a one-line "both frozen forever" comment would prevent a future footgun.

## 8. Dependencies & flags
- codec_api, media_clock, media_types, packet_queue, playback_engine, plugin_api, testkit, commands (RequestId) — all shipped crates; dev-only usage; no feature gates.

## 9. Open questions for the next agent
- The consumers (session_flow 34 tests, model_walk 4, hotplug 8, recovery_env 21, reopen_alloc 1) are other agents' files; this harness's quality sets their ceiling — their audits should confirm they actually exercise the armed-failure builders (fail_start/fail_select/fail_decode) rather than only the happy path.
