# AUDIT: crates/codecs/src/audio/mp2/seek.rs
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-21 |
| Lines / functions | 223 / 9 (4 fns + 1 struct + helper + 4 tests) |
| File verdict | UNREACHABLE (real, tested code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `rg -n "cbr_seek\|sample_at_byte\|refine_to_frame\|duration_nanos"` over the whole repo: the ONLY callers are in-crate tests (`decoder.rs:660-677` test, this file's tests, `crates/codecs/tests/mp2_corpus_differential.rs` incl. `cbr_seek_on_the_padded_corpus_stream_refines_exactly`). No demuxer, no container host, no engine lane calls the mp2 seek surface; the one shipped mp2 lane (PS host) seeks through the demuxer, not this module. Exported at `mp2/mod.rs:79` `pub use seek::{…}` — public API, zero production consumers. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | repo-wide `rg` for all four function names (callers enumerated above); `rg -n "pub use\|pub mod" mp2/mod.rs` (export surface, :71/:79); baseline logs: `test-workspace.log.txt:404-407`, `orchrun-test-workspace-nofailfast.log.txt:368-371,1096` (all 4 in-file tests + the padded-corpus refine test PASS). |

## 1. One-paragraph verdict
The arithmetic itself is sound and well-tested — exact frame math with clamping, a cross-module pin against the family (mp3) planner, and a genuine padded-stream refine test that passed in baseline. But the entire module is dead weight in the shipped product: no production code anywhere calls it, despite the module doc billing `sample_at_byte` as "the demuxer's duration/progress arithmetic" — no demuxer does. And that inverse arithmetic's documented accuracy ("an estimate within one frame") is mathematically false on fractional-rate streams: padding drift accumulates linearly (at 32 kbps/44.1 kHz the estimate passes one frame of error every ~215 frames, ~1.1 s over a 4-minute track), not bounded.

## 2. What actually works (verified)
- Baseline `test-workspace.log.txt:404-407`: `cbr_seek_agrees_with_the_family_planner`, `cbr_seek_is_exact_frame_arithmetic`, `sample_at_byte_inverts_on_integral_frame_sizes`, `refine_snaps_a_padded_stream_to_a_real_header` — all `ok`; `orchrun…:1096` `cbr_seek_on_the_padded_corpus_stream_refines_exactly ... ok` (real corpus 0119, 44.1 kHz padded frames).
- Hand check of the padded test: 6 frames, frames 0 and 3 padded (+1 slot each): nominal offset of frame 4 = 4×1253 = 5012, real = 5014; `refine_to_frame(data, 5012)` snapping to 5014 is the correct forward-snap behavior (nominal ≤ real always, since padding only adds bytes ahead).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/audio/mp2/seek.rs:18-20
Code:
```rust
//! The inverse direction ([`sample_at_byte`]) is the demuxer's
//! duration/progress arithmetic: fractional nominal frames make it an
//! estimate within one frame, documented on the function.
```
What is wrong: no demuxer uses it. The whole seek surface (`cbr_seek`, `sample_at_byte`, `duration_nanos`, `refine_to_frame`, `Mp2SeekPlan`) has zero production callers — repo-wide rg finds only in-crate tests. The doc's framing ("the demuxer's duration/progress arithmetic") asserts a consumer that does not exist; the PS host (the only shipped mp2 lane) computes neither duration nor seek through this module.
Why it matters: dead exported API plus a doc that misattributes its purpose — a future dev wiring PS-host seeking will trust "the demuxer's arithmetic" claim that was never true, and the crate carries tested-but-unreachable surface that the shipped binary never exercises (the tree-shake rationale of card 7.K1 applies: this is exactly the kind of code `--no-default-features` consumers still pay for since `mp2::seek` is not feature-gated).
What correct looks like: either wire it (PS host seek/duration for mp2 tracks) or demote the doc to "library/test surface, unwired" and gate or fold it into the test module.
Confidence: HIGH — Verification: repo-wide rg for every public name; only test files match.

### F-002 [S3]
Where: crates/codecs/src/audio/mp2/seek.rs:56-60
Code:
```rust
/// The inverse arithmetic: which sample index a byte offset falls on
/// (floor to the frame start). An estimate within one frame on
/// fractional-rate streams (padding slots accumulate ahead of the
/// nominal count); exact on 48/24/12 kHz where Layer II frame sizes
/// are integral.
```
What is wrong: the "within one frame" bound is false. The estimate uses the nominal (unpadded) frame size, so on fractional rates it runs AHEAD of reality by the accumulated padding, which grows linearly: error ≈ frac(frame_size) per frame. At 32 kbps/44.1 kHz (frame size 104.49, frac 0.49) the error passes one frame every ~215 frames (~5.6 s of audio) and reaches ~43 frames (~1.1 s) on a 4-minute track. Even at 384 kbps (1253.06, frac 0.06) it passes one frame per ~1800 frames. Only the doc's "exact on 48/24/12 kHz" clause is true.
Why it matters: a public function's accuracy contract that any progress-bar/duration consumer would rely on is off by an unbounded, stream-length-proportional amount; the module doc (lines 18-20) repeats the false claim as the demuxer's arithmetic.
What correct looks like: state the true bound (error grows by frac(frame_size)·frames, i.e. proportional to stream length; refine with a header snap like the forward direction does, or compute from real frame walk).
Confidence: HIGH — Verification: read + arithmetic re-derivation (error(bytes B) = B/nominal − B/real_avg frames; frac values from the Layer II table at 44.1 kHz).
Note: zero production callers (F-001) is the only reason this is S3 and not S2.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Mp2SeekPlan` (struct) | 28-37 | REAL | — | Carrier; consumed by tests only. |
| `cbr_seek` | 46-54 | REAL (unreachable) | — | Exact frame arithmetic; spf/total clamps guard div-by-zero and past-end; nominal byte can sit early on padded streams (refine snaps forward) — correct direction analysis re-derived (nominal ≤ real always). |
| `sample_at_byte` | 62-69 | REAL (unreachable) | S3 | False "within one frame" doc bound (F-002); `.max(1)` guards; clamps to total_samples. |
| `duration_nanos` | 74-79 | REAL (unreachable) | — | Saturating exact arithmetic from walked geometry; "truncated" documented. |
| `refine_to_frame` | 86-88 | REAL (unreachable) | — | Thin delegate to the family `scan_sync` — reused, not duplicated; false-sync-inside-payload risk is the family walker's accepted semantics. |
| test helper `lsf_frames` | 98-108 | REAL | — | Builds 5 walked LSF frames. |
| test `cbr_seek_is_exact_frame_arithmetic` | 110-138 | REAL | — | Zero/edge/mid/past-end assertions — would fail on arithmetic drift. |
| test `cbr_seek_agrees_with_the_family_planner` | 140-162 | REAL | — | Cross-module pin vs `mp3::cbr_seek` over 5 targets — genuine drift guard. |
| test `sample_at_byte_inverts_on_integral_frame_sizes` | 164-179 | REAL | — | Inversion + clamp + duration pin (360 ms exact). |
| test `refine_snaps_a_padded_stream_to_a_real_header` | 181-222 | REAL | — | Real padded geometry, snap-forward + past-end None; hand-verified offsets. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 4 in-file tests + `cbr_seek_on_the_padded_corpus_stream_refines_exactly` | Yes | Exact integer assertions on offsets/indices; the padded test uses real 1253/1254-byte frame geometry; the family-planner pin fails on any divergence between the two CBR implementations. |
| Gap | — | No test asserts the "within one frame" property of `sample_at_byte` on a long fractional-rate stream — which is why the false bound (F-002) survived. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`sample_at_byte` … the demuxer's duration/progress arithmetic" (18-19) | module doc | FALSE | No demuxer calls it (repo-wide rg; F-001). |
| "An estimate within one frame on fractional-rate streams" (57-58) | fn doc | FALSE | Linear drift: ~1 frame per 215 frames at 32 kbps/44.1 kHz (F-002 math). |
| "nominal … can sit a few slots early — never late" (11-13) | module doc | REAL | Padding only adds bytes ahead: nominal ≤ real for every frame. |
| docs/formats/audio-mp3-mp2.md:34 "seek = CBR arithmetic both directions + refine_to_frame snapping" | format doc | REAL-UNPROVEN | Arithmetic exists and is tested; no shipped lane executes it (F-001). |

## 7. Dead / duplicated / unreachable code
The entire file is the dead surface (F-001): exported at `mp2/mod.rs:79`, consumed by tests only. Not duplicated (delegates to family `scan_sync` rather than re-implementing).

## 8. Dependencies & flags
Depends only on `super::header::Mp2Geometry` and the family `mp3::scan_sync` — no new deps. Not feature-gated separately (rides `codec-mp2`).

## 9. Open questions for the next agent
- Does the PS container host (`apps/cli/src/containers/ps.rs`) implement in-session seeking for mp2 audio tracks at all, and if so through what arithmetic? If a future card wires it, F-001 downgrades to "wired" and F-002 upgrades to S2.
