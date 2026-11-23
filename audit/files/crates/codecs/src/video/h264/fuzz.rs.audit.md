# AUDIT: crates/codecs/src/video/h264/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08) |
| Date | 2026-11-23 |
| Lines / functions | 101 / 1 fn + 1 const + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (fuzz-infra only): sole caller is fuzz/fuzz_targets/fuzz_codec_h264.rs:19 `codecs::video::h264::fuzz::fuzz_codec_h264(data);` — the cargo-fuzz target, which fuzz/README.md honestly marks "SCAFFOLDED, NOT WIRED"; in-crate tests reach it (executed by me). `pub mod fuzz` compiles unconditionally (h264/mod.rs:55) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg FUZZ_NAL_BUDGET/fuzz_codec_h264 (crates/, fuzz/, apps/, tools/) -> 3 callers, all fuzz-infra/test; sed verifications; `./target/debug/deps/codecs-bae0f1756cc0a809 video::h264::fuzz::tests --nocapture` -> 2 passed, 0 failed, exit 0 |

## 1. One-paragraph verdict
A real, bounded, panic-contract fuzz harness that drives every context-free surface of the H.264 parser self-build (Annex-B walk, per-NAL SPS/PPS via the reusable-buffer entry, SEI walk, avcC record walk, length-prefixed walk at length_size 1/2/4), swallowing typed errors by documented design. Both in-crate hostile-input tests pass (executed by me from the prebuilt binary, and baseline cargo-test-workspace.txt:645-646). The libFuzzer shim exists and is registered (fuzz/Cargo.toml:230-231), but the campaign is not wired (Lead nightly decision outstanding, honestly documented in both this file and fuzz/README.md). Two documentation defects: the "every parser surface" headline is false (slice headers — the largest hostile surface — are excluded), and the "no allocation per NAL / RBSP buffer reused" bound claim is false for the SEI leg.

## 2. What actually works (verified)
- Harness executes all four documented passes on arbitrary bytes without panic: `./target/debug/deps/codecs-bae0f1756cc0a809 video::h264::fuzz::tests --nocapture` -> `2 passed; 0 failed` (empty/marker inputs, 4096-byte 0x00 flood, 0xff flood, 256×8-byte start-code flood).
- Called parser surfaces are real and typed (verified by reading): annexb_nals (nal.rs:310), SeqParameterSet::parse_with_buffer (sps.rs:175, wrong-NAL-type typed refusal at :181-185), PicParameterSet::parse_with_buffer (pps.rs:74), parse_sei (sei.rs:221), AvcConfig::parse (nal.rs:474+), AvcNals length-prefix walk with spin-refusal on zero length (nal.rs:426-432).
- Wiring honesty: fuzz/fuzz_targets/fuzz_codec_h264.rs exists and calls this harness; fuzz/README.md:79 registry row matches.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/h264/fuzz.rs:5-7 (contradicted by :22-23)
Code:
```
//! One call = one untrusted byte buffer through **every** parser
//! surface of the landed parser self-build (card 5.F1; decode itself
//! is the hw-first ADR-001 half and rides its own suites):
```
What is wrong: "every parser surface" is false. The slice-header seam (parser::slice, part of the landed parser self-build per h264/mod.rs:29-31 "slice headers + SEI (sibling card 5.F1b)") is excluded by the module's own admission at :22-23 ("The slice-header seam needs *active* SPS/PPS contexts (its own unit suites own that pass); every context-free surface is here."). The headline overstates coverage; the campaign, once wired, will never fuzz the largest hostile parsing surface (slice headers carry the heaviest arithmetic: MV prediction, ref-list walks). Why it matters: a future reader trusting the headline believes slice-header parsing is fuzz-covered; it is not. What correct looks like: headline says "every context-free parser surface", and the campaign gap is tracked as a follow-up card (context seeding or synthesized SPS/PPS contexts). Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/codecs/src/video/h264/fuzz.rs:38-39 and :50-52 (code at :58)
Code:
```
//! Memory is bounded by the input: NAL views are zero-copy, the RBSP
//! buffer is reused across NALs, a fixed number of passes per call.
```
and the SEI leg at :58: `if let Ok(rbsp_bytes) = super::parser::nal::unescape_rbsp(nal) {` — unescape_rbsp allocates a fresh Vec per NAL (nal.rs:363-367: `let mut out = Vec::with_capacity(nal.len());`).
What is wrong: "the RBSP buffer is reused across NALs" and "steady state performs no allocation per NAL" (:51-52) hold only for the SPS/PPS parses (:56-57, shared `rbsp` buffer); the SEI leg allocates a fresh NAL-sized Vec per NAL. Bounds remain input-derived (each alloc ≤ input size, transient), so the MP §12 safety claim survives, but the zero-alloc steady-state claim is false. What correct looks like: reuse one buffer via unescape_rbsp_into for the SEI leg too, or delete the zero-allocation claim. Confidence: HIGH — Verification: read (nal.rs:363-367 sed-verified).
F-003 [S4]
Where: crates/codecs/src/video/h264/fuzz.rs:82-100
Code:
```
        fuzz_codec_h264(&[]);
        fuzz_codec_h264(&[0x00, 0x00, 0x00, 0x01]);
        fuzz_codec_h264(&[0x00, 0x00, 0x01, 0x67]);
```
What is wrong: no smoke input contains an emulation-prevention sequence `00 00 03`, so this file's tests never drive the harness through the SEI leg's unescape-error branches (h264.ep.bad_suffix / h264.ep.dangling). Those branches are covered by parser nal.rs's own unit tests (nal.rs:796, :802 — verified by rg), so the risk is contained; the smoke set still misleads about its own coverage. What correct looks like: one input with `00 00 03 99` and one with trailing `00 00 03`. Confidence: HIGH — Verification: read + rg of test inputs.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub const FUZZ_NAL_BUDGET` | 41-44 | REAL | — | 4096 cap, second belt over input-length bound; pub but no external consumers (rg: only hevc/fuzz.rs:40 defines its own duplicate twin) |
| `pub fn fuzz_codec_h264` | 49-75 | REAL | — | 4 passes, bounded (take(FUZZ_NAL_BUDGET), input-length-bounded iterators), typed errors swallowed per documented contract; executed via tests |
| `tests::empty_and_marker_inputs_survive` | 81-86 | REAL | S4-adjacent | honest panic-contract smoke (see F-003) |
| `tests::hostile_wide_inputs_survive` | 89-100 | REAL | — | start-code flood, 0xff flood, 8-byte pattern flood; would fail on any panic/hang (mutation reasoning) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | YES for panic/hang on those inputs; NO for wrong-error regressions | Contract is absence-of-panic only; errors are swallowed by design (documented :25-27) |
| hostile_wide_inputs_survive | YES for panic/hang/OOM on floods (e.g. AvcNals zero-length spin at nal.rs:426 would hang this test) | Same swallow caveat; pass-4 error branches (zero_length, Truncated) ARE driven by 0x00 and 0xff floods (read-verified branch logic) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = ... through **every** parser surface" | fuzz.rs:5-7 | FALSE (as worded) | slice seam excluded, self-admitted :22-23 (F-001) |
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:25-27 | REAL | Code matches: every parse result discarded with `let _ =` (:56-59, :64) |
| "cargo-fuzz target is fuzz/fuzz_targets/fuzz_codec_h264.rs ... nightly wiring, see fuzz/README.md" | fuzz.rs:29-34 | REAL | Target file exists, calls harness at :19; README "SCAFFOLDED, NOT WIRED" |
| "Memory is bounded by the input ... RBSP buffer is reused across NALs" | fuzz.rs:36-39 | SHALLOW | Bounded-by-input: true; buffer-reuse: false for SEI leg (F-002) |
| Registry name `fuzz_codec_h264` (doc 04 §8.5, table §4.1) | fuzz.rs:1-3 | REAL | fuzz/Cargo.toml:230-231, fuzz/README.md:79 |

## 7. Dead / duplicated / unreachable code
Not dead: consumed by the (unwired) fuzz target + tests. `FUZZ_NAL_BUDGET` is pub with no external users — S4-grade over-visibility; its hevc twin is a copy-paste duplicate (see hevc/fuzz.rs audit).

## 8. Dependencies & flags
None beyond the parent crate (`codecs`). No feature gating: `pub mod fuzz` unconditional (h264/mod.rs:55), so it ships compiled in the release lane even though nothing in lumen-cli/desktop calls it.

## 9. Open questions for the next agent
- Slice-header fuzzing (context seeding) — the campaign's biggest hole (F-001). Who owns the follow-up card?
- When the Lead nightly decision lands, does CI pin a corpus/crash triage owner? fuzz/README.md is silent on triage.
