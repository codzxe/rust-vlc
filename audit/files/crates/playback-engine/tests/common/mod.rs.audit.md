# AUDIT: crates/playback-engine/tests/common/mod.rs

| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-22 |
| Lines / functions | 458 / 22 items |
| File verdict | REAL — the shared engine-test harness: seam fakes (demuxer/decoder/provider/sink), deterministic builders, bounded poll driver; the subject (real `Engine`) is never mocked |
| Reachable from shipped entry point? | N/A (test-support module, consumed by the session suites via `mod common`) |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 (zero — what-was-checked below) |
| Commands run | Read/Grep (quotes verified with unfiltered tools) |

## 1. One-paragraph verdict

Correct test-harness design: the fakes implement the engine's own seam
traits (`Demuxer`, `Decoder`, `DecoderProvider`, `FrameConsumer`) so
the real `Engine`, hub, state machine and pipeline are the code under
test. Determinism is total: the clock and the event throttle share a
frozen-at-0 virtual source (no sleeps anywhere), the demux script is a
shared `VecDeque` a test can mutate mid-run, and the stall flag answers
`Discontinuity` markers — the honest non-blocking "nothing new, stream
not ended" shape the engine counts. The `poll_until` driver fails
loudly instead of hanging. Poisoned mutexes are recovered
(`unwrap_or_else(|p| p.into_inner())`) so a panicked sibling thread
cannot cascade into opaque failures.

## 2. What actually works (verified)

— All four fakes exercise real engine paths: `FakeDemuxer::next_event`
script + stall + EOS (:101-118), `FakeDecoder`'s stale-packet guard
(:152-154) and one-shot failure (:149-151), `FakeProvider`'s armed
selection failure (the registry-lied case, :176-177), `Collector`'s
ordered frame capture (:70-78).
— Builders wire the real `SessionConfig` with tiny deterministic caps
(16 packets → high 12 / low 4, :275-285) and a frozen throttle
(:310-315).
— `open_stalled_audio` (:401-411) is the underrun fixture: preload then
a live stall — the exact shape the engine's starve/debounce logic is
tested against.

## 3. Findings

None. What was checked (per the no-clean-bill rule):
- `#![allow(dead_code)]` (:25): justified and documented — each
  consuming test crate uses a different subset (standard tests/common
  discipline); consumers exist (session_flow.rs, model_walk.rs and
  this wave's suites use `mod common`).
- No sleeps, no wall-clock reads, no unbounded loops: Grep
  `sleep|Instant::now` in this file → zero hits.
- No bare `unwrap()`: every lock is poison-recovering; `must()` is the
  loud testkit assert.
- `shallow_clone` (:188-196) drops the boxed source chain — documented
  as the fake's per-call copy need; the engine itself never clones
  errors, so no production behavior is mirrored wrongly.
- `receipt_for` (:447-458) fails loudly (`must()`) when a receipt is
  missing — no silent pass.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SharedNow` | 55-61 | REAL | — | frozen virtual time |
| `Collector` | 66-78 | REAL | — | poison-recovering |
| `FakeDemuxer::{start,next_event,seek}` | 93-128 | REAL | — | script/stall/EOS; seek honestly unsupported |
| `FakeDecoder::{configure,decode,flush}` | 138-162 | REAL | — | stale guard, one-shot failure, 1 frame/packet |
| `FakeProvider::decoder_for` | 170-184 | REAL | — | armed selection failure |
| `shallow_clone` | 188-196 | REAL | — | documented source-chain drop |
| builders (`base`…`decoder_error`) | 202-273 | REAL | — | typed error factories with stages |
| `tiny_queues`/`tiny_budget` | 278-293 | REAL | — | deterministic caps; high 12 / low 4 |
| `virtual_clock`/`frozen_throttle` | 295-315 | REAL | — | determinism seams |
| `config_with`/`open_with` | 317-389 | REAL | — | real SessionConfig wiring |
| `open_audio`/`open_stalled_audio` | 391-411 | REAL | — | EOF / underrun fixtures |
| `poll_until` | 416-433 | REAL | — | bounded, loud failure |
| `take_events`/`frames_of`/`receipt_for` | 435-458 | REAL | — | accessors; receipt_for must()-fails |

## 5. Test honesty

Support module (no tests of its own — by design). Its consumers'
honesty is judged in their own audits; the harness shape (fakes of
seams, real engine) is the honest pattern.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the model exercised through the real executor, never a mock of it" | :7-8 | REAL | fakes implement seam traits only; Engine is real |
| "an armed stall answers discontinuity markers — the honest non-blocking shape" | :82-85 | REAL | :110-116; engine counts marks (pipeline.rs:2169-2174) |
| "hoisted verbatim from tests/session_flow.rs … No behavior change" | :1-5 | UNVERIFIED | no git in sandbox; cannot diff history |

## 7. Dead / duplicated / unreachable code

`#![allow(dead_code)]` covers subset-use (documented). The fakes are
duplicated in trimmed form in bounded_alloc.rs (that file admits it) —
acceptable test isolation.

## 8. Dependencies & flags

`playback_engine`, `codec_api`, `media_clock`, `media_types`,
`packet_queue`, `plugin_api`, `commands`, `testkit`. No feature gates.

## 9. Open questions for the next agent

- Wave 16: the frozen clock means `timing.time-tick` emissions are
  deterministic; suites asserting tick cadence must advance `SharedNow`
  manually — check they do.
