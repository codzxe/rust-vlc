# AUDIT: crates/codecs/src/video/h264/parser/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-21 |
| Lines / functions | 33 / 5 module declarations + module-map/conventions docs (no functions — module root) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` h264 track → apps/cli/src/pipeline_host.rs:346 `H264SwDecoder::new` → crates/codecs/src/video/h264/sw.rs:93 `use …{AnnexBNals, AvcConfig, AvcNals, …, unescape_rbsp_into}` + :249 `AnnexBNals::new(extra)` / :267 `unescape_rbsp_into(payload, rbsp)` → this module tree. EXECUTED: baseline mp4-smoke-open-session-420.txt — h264 CB 4:2:0 MP4 plays end-to-end; h264_sw_differential 17/17 ok (6 lanes × annexb+mp4 entry forms, bit-exact vs ffmpeg) — every lane feeds this parser.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read tool full file; `ls` parser/ (all five declared modules exist); signature checks `rg "pub fn unescape_rbsp\|parse_with_buffer\|parse_sei"` (nal.rs:326,363; sps.rs:175; sei.rs:221); no-panic spot-verification: Read nal.rs:404-440 (AvcNals termination) + slice.rs:132-146 (golomb >31 guard); baseline greps: 35/35 `h264::parser` unit tests ok, h264_parser_corpus 2/2 ok (:1055-1060), h264_sw_differential 17/17 ok (:1063-1085), 420-MP4 open session |

## 1. One-paragraph verdict
A module root whose entire body is five accurate `pub mod` declarations and a conventions paragraph — and it is the most-executed file in my assignment: every shipped h264 open (including the baseline's playing 4:2:0 MP4) and every one of the 17 differential lanes drives this tree through the sw decoder's bridge, with 35 in-module unit tests, a 2-lane real-bytes corpus, and an ffmpeg bit-exactness suite all green in the baseline. The module map matches the files; the "typed errors, no panic, no hang, no decode claim" convention is read-verified on the two hot paths I cross-checked (AvcNals termination, the Exp-Golomb overflow guard) and executed-verified by the hostile garbage/truncation sweeps. The single defect is a conventions sentence that mispredicts the `unescape_rbsp*` family's input type.

## 2. What actually works (verified)
— End-to-end: baseline `sample-420.mp4` h264 open → Buffering → Playing → Stopped → Idle, EXIT=0 (mp4-smoke-open-session-420.txt) — the avcC/Annex-B framing this module owns ran in the shipped binary.
— Differential: `h264_sw_differential` 17/17 ok (baseline :1063-1085) — lanes a–f (baseline/main-CABAC/b-frames/high-IPB/odd-crop/still) × both entry forms (annexb + mp4), bit-exact vs ffmpeg 7.1.5, plus `hostile_garbage_packets_recover`, `hostile_garbage_then_real_recovers`, `hostile_truncation_sweep_is_panic_free`.
— Corpus: `h264_parser_corpus` 2/2 ok (:1055-1060) — `corpus_mkv_h264_tracks_field_assertions`, `corpus_raw_annexb_0019_field_assertions` (real bytes, field-level pins).
— 35/35 `video::h264::parser::*` unit tests ok in baseline.
— Module-map accuracy: `nal`, `sps`, `pps`, `slice`, `sei` all exist; their surfaces match the map's descriptions (nal: Annex-B + length-prefixed + avcC + EPB + BitReader — all seen in fuzz.rs/sw.rs usage; slice: "minimal SPS/PPS context seams" — same shape as the hevc twin I audited).

## 3. Findings

### F-001 [S4]
Where: crates/codecs/src/video/h264/parser/mod.rs:22-24 (conventions), vs nal.rs:326/:363
Code:
```
//! All functions take/return **complete NAL units** (payload *with*
//! their one-byte header) unless a name says `rbsp` (emulation
//! prevention already removed — the NAL layer's job).
```
What is wrong: the stated rule — "a name saying `rbsp` means emulation prevention is already removed (i.e., the function takes RBSP input)" — is followed by `parse_sei(rbsp: &[u8])` (sei.rs:221, fed unescape output by the fuzz harness) but INVERTED by the unescape family: `unescape_rbsp(nal: &[u8])` (nal.rs:363) and `unescape_rbsp_into(nal: &[u8], out: &mut Vec<u8>)` (nal.rs:326) take complete NALs with escapes present — "rbsp" in their names refers to the OUTPUT, not the input.
Why it matters: a reader applying the convention literally would call `unescape_rbsp` on already-unescaped data (double-processing — escapes are idempotent-safe only if none remain, and a real `00 00 01` inside RBSP payload would be wrongly stripped), or skip unescaping before `parse_sei`. Input-type documentation on a parsing library's public surface is load-bearing.
What correct looks like: reword — "…unless a name says `rbsp`, which then takes already-unescaped RBSP input (`parse_sei`); the `unescape_rbsp*` converters are the exception: they take complete NALs and produce that RBSP."
Confidence: HIGH — Verification: read (all four signatures cited).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (map + conventions) | 1-31 | REAL | S4 | map accurate (verified per module); conventions F-001; "clean-room" provenance claim UNVERIFIABLE from code (history, not artifact) |
| `pub mod nal` | 33 | REAL | — | exists; Annex-B/length-prefixed/avcC/EPB/BitReader surfaces confirmed via fuzz.rs + sw.rs consumers |
| `pub mod pps` | 33 | REAL | — | exists |
| `pub mod sei` | 33 | REAL | — | exists; parse_sei takes rbsp per convention ✓ |
| `pub mod slice` | 33 | REAL | — | exists; golomb reader's `leading_zeros > 31 → GolombOverflow` guard (slice.rs:132-146) — the h264 twin does NOT share the hevc ceil_log2 underflow defect (my hevc slice.rs F-001, cross-checked) |
| `pub mod sps` | 33 | REAL | — | exists; `parse_with_buffer(nal, rbsp)` — the reusable-buffer discipline the conventions claim ✓ (sps.rs:175) |

## 5. Test honesty
No tests in this file; the tree beneath it is the evidence (all green in baseline):
| Suite | Would it fail if the code were broken? | Reason |
|---|---|---|
| 35 unit tests `video::h264::parser::*` | YES | field-level pins incl. avcC parse, cropping, VUI (names verified in baseline log) |
| h264_parser_corpus (2) | YES | real mkv/raw-annexb bytes, field assertions vs ffprobe-derived expectations |
| h264_sw_differential (17) | YES | bit-exact vs ffmpeg on 6 content classes × 2 framings — any parser/framing regression flips bytes; hostile sweeps pin panic-freedom on garbage/truncation shapes |
| End-to-end 420 MP4 (baseline smoke) | YES | the shipped-binary path itself |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "All functions take/return complete NAL units … unless a name says `rbsp`" | mod.rs:22-24 | SHALLOW | parse_sei obeys; the unescape family inverts it (F-001) |
| "the RBSP unescape offers a caller-owned reusable buffer so steady-state parsing … performs no allocation per NAL" | mod.rs:25-27 | REAL | `unescape_rbsp_into` (nal.rs:326) + `parse_with_buffer(nal, &mut rbsp)` (sps.rs:175), consumed by sw.rs:267 — contrast the fuzz harness's SEI lane which still uses the allocating twin (my h264/fuzz.rs F-002) |
| "All parsers return typed errors: no panic, no hang, no decode claim" | mod.rs:28-29 | REAL (read, hot paths) / REAL-UNPROVEN (exhaustive) | read-verified: AvcNals cannot spin (nal.rs:404-440), golomb overflow guarded (slice.rs:138-139); executed on hostile shapes by the differential sweeps; exhaustive panic-freedom still rests on the never-run fuzz campaigns (my h264/fuzz.rs F-001) |
| "ITU-T H.264 03/2023 syntax, clean-room" | mod.rs:1-2 | UNVERIFIED | provenance is history, not code; no spec-copy artifacts checkable from the tree |
| Module map (nal/sps/pps/slice/sei + card attributions) | mod.rs:5-15 | REAL | all five exist with matching surfaces |

## 7. Dead / duplicated / unreachable code
None at this level: every declared module has production consumers (sw.rs bridge) and tests. Near-duplication with the hevc parser tree is structural (two codecs), documented as a deliberate mirror.

## 8. Dependencies & flags
None at this level (leaf module declarations). The tree is unconditional (`pub mod h264;` — video/mod.rs:29, in-house module), correctly so: the sw decoder that consumes it is default-on (`codec-h264-sw`).

## 9. Open questions for the next agent
- nal.rs owner: pin the `unescape_rbsp`-on-already-unescaped-data hazard with a unit test (double-unescape must be a no-op or typed error) — the F-001 wording gap makes the misuse plausible.
- The "clean-room" claim for both h264 and hevc parsers (both cite public specs): if any wave owns provenance verification, the claim is currently trust-based.
