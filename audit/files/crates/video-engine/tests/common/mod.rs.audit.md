# AUDIT: crates/video-engine/tests/common/mod.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 273 / 16 items |
| File verdict | REAL — the integration-suite fixture module: virtual monotonic and audio-master clocks over media-clock's injected-truth traits, synthetic frames, recording presenter with an in-order assertion helper |
| Reachable from shipped entry point? | N/A (test-support module for crates/video-engine/tests/*; consumed by sync_scenarios.rs — audited next — and siblings) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

The integration twin of the crate's cfg(test) `test_support.rs`, with
the additions the integration suites need: `AudioMasterClock` — the
production configuration (audio master over an injected 48 kHz sample
counter, doc 01 §1.3 rule 8) so the scheduler sees exactly the samples
the test advances (:100-151) — plus `pause`/`resume` on the monotonic
twin (:85-93), a `PresentRecord` struct, and `assert_in_order`
(:248-257) that encodes doc 03 §4.10 f's in-order presentation
property as a reusable check. Same honest patterns as the unit copy:
`fetch_max` monotonic safety (:62-67), fail-before-log presenter
(:260-268), structured typed presenter failure.

## 2. What actually works (verified)

— Both clock masters are the real `MediaClock` constructions
(`system_monotonic`, `audio_master`) — the fakes are the sources, the
clock math under test is real.
— `AudioMasterClock::advance_samples` is the pump's consumption shape
— the exact mechanism that is missing in the shipped CLI between
commands (see the playback-engine audit F-001): here the TEST plays
the pump's role deterministically.
— `assert_in_order` gives every consumer suite the ordering property
for free with a context-bearing failure message.

## 3. Findings

### F-001 [S4] Near-verbatim duplication of src/test_support.rs with no cross-pin (same as the unit copy's F-001)
Where: tests/common/mod.rs:1-6 vs crates/video-engine/src/test_support.rs

Code:
```
//! Integration tests cannot see the crate's cfg(test)
//! `test_support`, so this module carries the production-shaped
//! fakes
```
What is wrong: forced by Rust's module visibility, documented here —
but the two copies (FakeMonotonic, frame builders, presenter_failure,
RecordingPresenter) are near-verbatim with semantic drift risk and no
cross-reference or shared test that pins their agreement (the unit
copy's `frame_stream_base` is absent here; this copy's
`AudioMasterClock`, `pause`/`resume`, `PresentRecord`,
`assert_in_order` are absent there — deliberate, but undocumented as
a policy).
What correct looks like: a two-line comment in each file naming the
other and the intended delta (integration adds audio-master + pause
+ records; unit adds stream-base frames).
Confidence: HIGH — Verification: read both files.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `FakeMonotonic` | 27-33 | REAL | — | |
| `VirtualClock::{new,clock,set_ns,advance_ns,now_ns,set_generation,pause,resume}` | 39-94 | REAL | — | fetch_max safety |
| `FakeCounter` (SampleCounterSource) | 106-119 | REAL | — | 48 kHz |
| `AudioMasterClock::{new,clock,advance_samples,set_generation}` | 121-151 | REAL | — | the production clock shape |
| `frame`/`frame_missing` | 159-177 | REAL | — | |
| `presenter_failure` | 185-194 | REAL | — | structured typed error |
| `PresentRecord` | 198-204 | REAL | — | |
| `RecordingPresenter::{new,failing_at,slots_seen,gens_seen,records,assert_in_order,present}` | 208-273 | REAL | — | fail-before-log; reusable ordering assert |

## 5. Test honesty

Fixture module (no tests of its own); its consumers' honesty is judged
in their own audits (sync_scenarios.rs next).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no sleeps, ever" | :4-5 | REAL | no sleep/timer in the module (Grep) |
| "the configuration real playback uses (doc 01 §1.3 rule 8)" | :98-99 | REAL | matches pipeline_host.rs:1609-1612 audio_master construction |

## 7. Dead / duplicated / unreachable code

`#![allow(dead_code)]` subset-use; the duplication (F-001).

## 8. Dependencies & flags

`video_engine` (Presenter/PresentRequest pub), `media_clock`,
`media_types`, `testkit`. No feature gates.

## 9. Open questions for the next agent

- Wave 6: whether any integration suite relies on fixture semantics
  that silently differ from the unit copy (F-001's drift risk).
