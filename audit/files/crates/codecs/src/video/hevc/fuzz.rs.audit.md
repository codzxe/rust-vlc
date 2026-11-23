# AUDIT: crates/codecs/src/video/hevc/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08) |
| Date | 2026-11-23 |
| Lines / functions | 93 / 1 fn + 1 const + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (fuzz-infra only): sole caller fuzz/fuzz_targets/fuzz_codec_hevc.rs:19 `codecs::video::hevc::fuzz::fuzz_codec_hevc(data);` — cargo-fuzz target, campaign "SCAFFOLDED, NOT WIRED" (fuzz/README.md); in-crate tests reach it (executed by me). `pub mod fuzz` unconditional (hevc/mod.rs:48). The HEVC tree itself has NO shipped decode lane (CLI: pipeline_host.rs:376 typed `no decoder for \`{other}\``; mp4.rs:29 "No software h264/hevc decoder is dispatched in this") |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg symbol checks (annexb_nals nal.rs:390, LengthPrefixedNals nal.rs:463, HvcCConfig nal.rs:554/593, parse_with_buffer vps.rs:318/sps.rs:204/pps.rs:111); `./target/debug/deps/codecs-bae0f1756cc0a809 video::hevc::fuzz::tests --nocapture` -> 2 passed, 0 failed, exit 0; `--list | grep -c hevc::parser` -> 19 |

## 1. One-paragraph verdict
A real, bounded, panic-contract fuzz harness — the structural twin of h264/fuzz.rs minus the SEI leg (HEVC parser has no sei module) plus the VPS parse. It drives every context-free HEVC parser surface (Annex-B walk, per-NAL VPS/SPS/PPS via the shared reusable RBSP buffer, hvcC record walk, length-prefixed walk at 1/2/4), swallowing typed errors by documented design. Both hostile-input tests pass (executed by me; the buffer-reuse bounds claim is TRUE here — all three per-NAL parses share `rbsp`, unlike the h264 twin's SEI leg). The libFuzzer shim exists, registered, honestly unwired. Same headline defect as the h264 twin: "every parser surface" is false — the slice-header seam is excluded — plus twin-file duplication of the budget const and a smoke set that never feeds SPS/PPS-typed headers.

## 2. What actually works (verified)
- Harness executes all four passes on arbitrary bytes without panic: `./target/debug/deps/codecs-bae0f1756cc0a809 video::hevc::fuzz::tests --nocapture` -> `2 passed; 0 failed` (empty/marker inputs incl. a valid VPS-typed header `0x40 0x01`, 4096-byte 0x00 flood, 0xff flood, 256×8-byte start-code flood).
- Called surfaces are real and typed (read-verified): annexb_nals (nal.rs:390), VideoParameterSet::parse_with_buffer (vps.rs:318), SeqParameterSet::parse_with_buffer (sps.rs:204), PicParameterSet::parse_with_buffer (pps.rs:111), HvcCConfig::parse (nal.rs:593), LengthPrefixedNals walk (nal.rs:463-489).
- Bounds claim TRUE for this file: "the RBSP buffer is reused across NALs" (:35-36) — :48-52 all three parses share `&mut rbsp`.
- Wiring honesty: fuzz/fuzz_targets/fuzz_codec_hevc.rs exists and calls this harness; fuzz/Cargo.toml registers it; fuzz/README.md marks the campaign scaffolded-not-wired.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/hevc/fuzz.rs:5-7 (contradicted by :19-20)
Code:
```
//! One call = one untrusted byte buffer through **every** parser
//! surface of the landed parser self-build (card 5.G1; decode stays
//! hw-first per ADR-001 and rides its own suites):
```
What is wrong: "every parser surface" is false — the HEVC slice-header seam (parser::slice, part of the landed self-build) is excluded by the module's own admission at :19-20 ("The slice-header seam needs active VPS/SPS/PPS contexts (its own unit suites own that pass); every context-free surface is here."). Once the campaign is wired, the heaviest hostile surface (HEVC slice headers: CTB addressing, ref-set walks, MVP) is never fuzzed. Why it matters: the headline misleads exactly like the h264 twin's; the campaign's biggest hole is institutionalized in both trees. What correct looks like: "every context-free parser surface" + a tracked follow-up card for context seeding. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/codecs/src/video/hevc/fuzz.rs:38-40 (twin: h264/fuzz.rs:41-44)
Code:
```
/// Maximum NALs parsed per pass `[PROPOSAL: 4096 — far above any
/// sane AU's NAL count; a second belt over the input-length bound]`.
pub const FUZZ_NAL_BUDGET: usize = 4096;
```
What is wrong: copy-paste twin of h264/fuzz.rs — same const name, value, and doc prose in both trees; rg finds no other consumers. The two harness bodies are structural duplicates. A future budget change in one file silently leaves the other at 4096. What correct looks like: one shared const (e.g. in a codecs-internal fuzz-support module) referenced by both harnesses, or per-tree names if divergence is intended. Confidence: HIGH — Verification: read + rg.
F-003 [S4]
Where: crates/codecs/src/video/hevc/fuzz.rs:74-91
Code:
```
        // NAL type 32 (VPS) header byte after a start code.
        fuzz_codec_hevc(&[0x00, 0x00, 0x01, 0x40, 0x01]);
```
What is wrong: every smoke input that gets past NAL-type filtering carries only VPS-typed headers (0x40); SPS (type 33, 0x42) and PPS (type 34, 0x44) parse legs are only ever exercised as wrong-NAL-type refusals by this file's tests — their syntax walks are covered solely by the parser's own 19 unit tests (verified: `--list | grep hevc::parser` -> 19, incl. corpus_0044_parameter_sets_and_slice_prefixes_parse). Blast radius small; the smoke set still overstates its own coverage. What correct looks like: add one 0x42-typed and one 0x44-typed header to the flood pattern. Confidence: HIGH — Verification: read + ran --list.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub const FUZZ_NAL_BUDGET` | 38-40 | REAL | S4 | duplicate twin const (F-002); no external consumers |
| `pub fn fuzz_codec_hevc` | 45-67 | REAL | — | 4 passes, bounded (take cap + input-length-bounded iterators), shared RBSP buffer across all per-NAL parses, typed errors swallowed per contract; executed via tests |
| `tests::empty_and_marker_inputs_survive` | 73-79 | REAL | — | honest panic-contract smoke incl. valid VPS header |
| `tests::hostile_wide_inputs_survive` | 81-92 | REAL | S4-adjacent | floods real; SPS/PPS legs unexercised (F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| empty_and_marker_inputs_survive | YES for panic/hang on those inputs | Contract is absence-of-panic; errors swallowed by design (:22-24) |
| hostile_wide_inputs_survive | YES for panic/hang/OOM on floods; would catch a LengthPrefixedNals zero-length spin (nal.rs iterator) via hang | Same swallow caveat; SPS/PPS syntax walks not driven (F-003) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = ... through **every** parser surface" | fuzz.rs:5-7 | FALSE (as worded) | slice seam excluded, self-admitted :19-20 (F-001) |
| "harness contract is absence-of-panic/hang/unbounded-allocation, nothing else" | fuzz.rs:22-24 | REAL | Code matches: all results discarded `let _ =` (:50-52, :56) |
| "cargo-fuzz target is fuzz/fuzz_targets/fuzz_codec_hevc.rs" | fuzz.rs:26-31 | REAL | File exists, calls harness at :19; README honest |
| "Memory is bounded by the input ... RBSP buffer is reused across NALs" | fuzz.rs:33-36 | REAL | :48-52 — all three parses share `&mut rbsp` (UNLIKE the h264 twin — that file's variant of this claim is false) |
| Registry name `fuzz_codec_hevc` (doc 04 §8.5, table §4.1 row HEVC/H.265) | fuzz.rs:1-3 | REAL | fuzz/Cargo.toml target + fuzz/README.md registry |

## 7. Dead / duplicated / unreachable code
Not dead: consumed by the (unwired) fuzz target + tests. Duplication: whole-file structural twin of h264/fuzz.rs (F-002). The entire HEVC tree is production-unreachable in this build (no hevc decoder dispatched — pipeline_host.rs:376, mp4.rs:29, baseline mp4-smoke "no decoder for `hevc`" exit 65).

## 8. Dependencies & flags
None beyond the parent crate. `pub mod fuzz` unconditional (hevc/mod.rs:48) — compiles into the shipped crate, never called by production.

## 9. Open questions for the next agent
- Same as h264/fuzz.rs: slice-header fuzzing hole (F-001) — is a shared context-seeding card filed for both trees?
- If HEVC ever gains a software decoder, re-check this file's reachability verdict.
