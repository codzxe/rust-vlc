# AUDIT: crates/audio-engine/src/error.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 120 / 7 error constructors + 1 `#[test]` fn |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | **YES** — the constructors are the audio path's typed-failure home; e.g. `chunk_alignment` fires from the pipeline's per-quantum alignment check (crates/audio-engine/src/pipeline.rs:331-333, on the shipped AVI vertical's mix path), `partial_frame`/`plane_mismatch` from convert.rs, `channel_range` from channel_map.rs/pipeline.rs. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | rg per-constructor usage across crates/audio-engine/src (all 7 have live callers); full-file read |

## 1. One-paragraph verdict

Seven tiny typed-error constructors, one auditable home for the (stage, kind, code) mapping — stage always `Mix`, stable dotted codes, structured details (expected/found counts, no stringly-typed payloads). Every constructor has live callers in the crate's real code paths; the single test pins the taxonomy contract (stage tag, code stability, Display contains the code and the cause word). The only defect is a stale doc comment: `unsupported_format`'s example says "a rate mismatch — the micro-resampler is a later card", but the micro-resampler landed (card 3.B1, resample.rs) and — per my resample.rs audit — it does *not* fix track-rate conversion anyway (±1000 ppm correction only), so both halves of that sentence mislead.

## 2. What actually works (verified)

- All seven constructors have non-test callers (rg counts above: bad_gain → volume/effects; channel_range → pipeline/effects/channel_map; unsupported_format → eq/pipeline/effects; partial_frame → convert; plane_mismatch → convert/channel_map; track_range → pipeline/mix; chunk_alignment → pipeline/mix).
- The one test asserts the wire-relevant invariants (stage, code, Display text).

## 3. Findings

### F-001 [S4] — Stale doc on `unsupported_format`: the "later card" landed, and it does not cover the example given

Where: crates/audio-engine/src/error.rs:36-37.

Code (verbatim):
```rust
/// A format the v0 audio path does not convert (e.g. a rate mismatch —
/// the micro-resampler is a later card, doc 03 §4.7 e).
```

What is wrong: (a) the micro-resampler is not "a later card" — `crates/audio-engine/src/resample.rs` exists (card 3.B1, merged per phase3-exit.md:31); (b) the resampler that exists handles only ±1000 ppm drift correction, not track-rate conversion (resample.rs:47-54: "Track-rate conversion … is a different, later card"), so the doc's example (rate mismatch → wait for the resampler) is wrong twice: the resampler arrived and still refuses this case. The error path itself is correct — a rate mismatch today should indeed surface `audio.rate_conversion_unsupported` — only the explanation rots.

Confidence: HIGH — Verification: read + resample.rs module docs 47-54.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `STAGE` const | 13 | REAL-UNPROVEN | — | `PipelineStage::Mix` for all |
| `bad_gain` | 19-25 | REAL-UNPROVEN | — | Unsupported; callers volume/effects |
| `channel_range` | 28-34 | REAL-UNPROVEN | — | Unsupported; callers pipeline/effects/channel_map |
| `unsupported_format` | 38-44 | REAL-UNPROVEN | S4 | F-001 stale doc; callers eq/pipeline/effects |
| `partial_frame` | 50-58 | REAL-UNPROVEN | — | Corrupt::Truncated{expected, found}; caller convert |
| `plane_mismatch` | 62-70 | REAL-UNPROVEN | — | Corrupt::InvalidStructure; callers convert/channel_map |
| `track_range` | 73-79 | REAL-UNPROVEN | — | Unsupported; callers pipeline/mix |
| `chunk_alignment` | 86-94 | REAL-UNPROVEN | — | Corrupt::Timing; callers pipeline/mix; fires per-quantum on the shipped mix path |
| test (1) | 101-118 | REAL-UNPROVEN | — | taxonomy pin |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `codes_are_stable_and_lowercase` (101-118) | YES (for the pinned cases) | stage, code, and Display content asserted for three constructors |

Coverage is representative, not exhaustive (4 of 7 constructors unexercised in-file) — acceptable for pure constructors; each is transitively exercised by its caller's tests (86-test crate suite per W00).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the micro-resampler is a later card" | error.rs:37 | FALSE (stale) | resample.rs landed (3.B1); F-001 |
| "Every failure in this crate is an EngineError at stage Mix" | error.rs:3-4 | REAL-UNPROVEN | all constructors tag STAGE; failures raised by media-types/OS layers pass through unchanged (out of this file's scope) |
| "codes are part of the compatibility surface (tests may key on them)" | error.rs:5-6 | REAL | in-file test keys on them |

## 7. Dead / duplicated / unreachable code

None — all constructors have live callers.

## 8. Dependencies & flags

Depends on `media_types::error` only. No feature gates.

## 9. Open questions for the next agent

- The remaining audio-engine modules (mix.rs, channel_map.rs, convert.rs, volume.rs, governor.rs, buffer.rs, pipeline.rs) — other agents; this file's constructors are their failure vocabulary.
