# AUDIT: crates/codecs/src/video/h264/parser/slice.rs#part1 (lines 1-533)
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-21 |
| Lines / functions | 533 of 2123 (chunk part1; boundary: through `PicOrderSyntax`, next item `RefPicListModification` at :535 belongs to A06/part2) / 23 items |
| File verdict | UNREACHABLE (the code is real and correct; no shipped entry point executes it) |
| Reachable from shipped entry point? | NO — sole production caller chain is `decode.rs:516 → parse_slice_header_pub (:532-538) → parse_slice_header` on the hw-only `H264Decoder`, itself unreachable (see hw/mod.rs audit F-001); the shipped `H264SwDecoder` uses rust_h264's own SPS parser and never parses slice headers in-house |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | rg "parse_slice_header" (only decode.rs:516/532/538); rg "SliceSpsInfo" (decode.rs:101-111 adapter, hevc slice.rs, mod.rs docs); rg "RbspBitReader" (sei.rs:47 imports it — NOT nal's reader); sed -n nal.rs:590-700 (second BitReader read); rg "delta_pic_order_always_zero" (sps.rs:252 discards; decode.rs:106 hardcodes false); rg "parser::slice::tests::" baseline log (9 h264 tests ok :614-621,:634); independent re-derivation of read_ue/read_se range math |

## 1. One-paragraph verdict
Part1 is the bit-reader (`RbspBitReader`), the typed error taxonomy, and the SPS/PPS/NAL projection seams of the in-house H.264 slice-header parser. The engineering is genuinely careful: every read is bounds-checked and typed, Exp-Golomb overflow is refused at 31 leading zeros, the Table 7-7 slice-type mapping is correct (auditor re-derived), and the se(v) mapping cannot overflow i32 given read_ue's u32 domain (auditor re-derived the bound). But the whole subsystem is dead code in shipped builds: the only shipped H.264 decoder (`H264SwDecoder`, selected at `apps/cli/src/pipeline_host.rs:346`) delegates SPS parsing to rust_h264 (`sw.rs:90` `use rust_h264::nal::{..., parse_sps_info}`, `:292`, `:509`) and never calls the in-house slice parser — the sole caller is the hardware-path `H264Decoder` (decode.rs:516), which no shipped host constructs. Layered on that: a designed seam field (`delta_pic_order_always_zero_flag`) whose only production populator hardcodes `false`, and a duplicated bit-reader primitive with two docs that contradict each other about who shares what.

## 2. What actually works (verified)
— `rg -n "parser::slice::tests::" audit/runs/baseline/test-workspace.log.txt` → 9/9 h264 slice tests `ok` (:614-621, :634), including `corpus_0014_mkv_lane_matches_the_ffmpeg_trace` and `corpus_0019_slice_headers_match_the_ffmpeg_trace` — the reader + vocabulary match ffmpeg-derived traces on real streams.
— Auditor re-derivation (this session): `SliceType::from_code` Table 7-7 mapping (0/5→P, 1/6→B, 2/7→I, 3/8→SP, 4/9→SI) is correct; `read_ue` max value with 31 leading zeros is 2³²−2 (fits u32), so `read_se`'s odd arm maxes at (2³²−2+1)/2 = 2³¹−1 = i32::MAX and the even arm at −2147483647 > i32::MIN — no i32 overflow possible.
— `SliceNalInfo::from_header_byte` (0x80 forbidden-bit check, types 1/5 only) matches §7.3.1; `chroma_array_type` (separate_colour_plane_flag → 0) matches §7.4.2.1.1.

## 3. Findings

### F-001 [S2] Entire in-house slice-header parser unreachable from shipped builds — the shipped software decoder never parses slice headers itself
Where: crates/codecs/src/video/h264/parser/slice.rs:1-533 (whole file, this part included); consumption chain decode.rs:516
Code:
```
//! A slice header cannot be parsed statelessly: its field widths and
//! optional-field gates come from the *active* SPS and PPS (e.g.
```
```
impl<'a> RbspBitReader<'a> {
    /// Reader over a full RBSP (bit 0 = MSB of byte 0).
    pub fn new(data: &'a [u8]) -> RbspBitReader<'a> {
```
What is wrong: No shipped binary executes this code. Proof chain: (a) `rg "parse_slice_header"` → the only non-test caller is `crates/codecs/src/video/h264/decode.rs:516` inside `H264Decoder` (the hw-path decoder); (b) `H264Decoder` is constructed nowhere outside codecs-crate tests (rg `H264Decoder\b` — see my hw/mod.rs audit F-001 for the full flag trace: product.toml:168 `hw-decode = false`, CLI hard-codes `H264SwDecoder`); (c) the shipped software decoder uses the in-house parser ONLY for NAL framing/EPB (`sw.rs:92` imports `super::parser::nal::{...}`) and rust_h264's own `parse_sps_info` (`sw.rs:90,292,509`) — slice headers go straight into the rust_h264 engine.
Why it matters: ~2100 lines of parser (plus its 9-test suite and fuzz target) are maintained, reviewed and compiled into every shipped binary as dead weight; the docs' "the LANDED parser (5.F1) meets a hardware adapter" framing (h264/mod.rs:33-35) accurately concedes the only consumer is the hw lane — but nothing states that the shipped sw track bypasses the slice parser entirely. Any bug found here has zero user-visible impact; any fix here improves nothing shipped. That inverts audit priorities and is exactly the "feature flags gating real code that shipped builds never enable" smell, minus the flag.
What correct looks like: Either route `sw.rs` through the in-house slice-header facts (the original ADR-001 plan's damage accounting would then be uniform), or gate the parser behind a `codec-h264-parse` feature and drop it from default builds, or document at h264/mod.rs level that the parser is hw-lane-only.
Confidence: HIGH — Verification: read + rg caller traces on both decoders; baseline confirms both suites pass independently (slice tests :614-621; sw differential per feature-matrix row notes).

### F-002 [S3] The `delta_pic_order_always_zero_flag` seam field exists, is honored by the parser, but its only production populator hardcodes `false`
Where: crates/codecs/src/video/h264/parser/slice.rs:310-312 (the field, this part); decode.rs:106 and sps.rs:252 (cross-file)
Code:
```
    /// `delta_pic_order_always_zero_flag` — when set, the type-1
    /// `delta_pic_order_cnt` syntax is absent.
    pub delta_pic_order_always_zero_flag: bool,
```
Cross-file (decode.rs:98-106):
```
/// SPS → slice-parser projection. `delta_pic_order_always_zero_flag`
/// is `false` because the LANDED SPS parser discards that bit (see
/// module docs — recorded limitation, never a guess).
...
        delta_pic_order_always_zero_flag: false,
```
Cross-file (sps.rs:252): `let _always_zero = r.bit()?; // delta_pic_order_always_zero_flag`
What is wrong: The slice parser consumes the flag correctly (slice.rs:795 `if sps.delta_pic_order_always_zero_flag` skips the two se(v) deltas; the test bootstrap at :1286 even reads it faithfully), but the production adapter feeds it a constant `false` because the SPS parser reads the bit and throws it away. Consequence: every `pic_order_cnt_type == 1` stream with the flag set (spec-legal, realistic in broadcast encodes) has two phantom `se(v)` reads at :797-799 that desync the bit position — every subsequent field of every slice header of that stream is garbage, surfacing as typed parse damage (decode.rs:23-27 admits "a set-flag stream surfaces as slice-header parse damage").
Why it matters: The seam was designed for exactly this bit; one `let _ =` in sps.rs converts a correct parser into a wrong one for a whole stream class. It is honestly documented (decode.rs:23-27 "recorded as a parser follow-up, not papered over") and currently latent (only the unreachable hw lane calls the parser — F-001), but the first hw wiring to ship will mis-parse these streams.
What correct looks like: sps.rs stores the parsed bit; decode.rs:106 maps `sps.delta_pic_order_always_zero_flag`. Two one-line changes; the field, the parse branch and the tests already exist.
Confidence: HIGH — Verification: read all four sites; consumption branch at :795 (A06's range) read for impact analysis.
### F-003 [S3] Duplicated bit-reader primitives with contradictory sharing claims
Where: crates/codecs/src/video/h264/parser/slice.rs:34-39 vs parser/nal.rs:590-594 (cross-file), evidence sei.rs:47
Code (slice.rs:34-39):
```
/// MSB-first bit reader over an RBSP slice (§9: `u(n)`, `ue(v)`,
/// `se(v)`).
///
/// Lives here because the slice header is this card's consumer; at
/// Lead merge it should hoist to `h264/parser/bitreader.rs` and serve
/// F1a's SPS/PPS/NAL parsers too (same reading semantics).
```
Code (nal.rs:590-594):
```
/// MSB-first bit reader over an RBSP (emulation prevention already
/// removed) with Exp-Golomb support (§9.1/§9.2) — the shared
/// primitive of every `h264::parser` syntax reader (SPS/PPS here;
/// slice headers/SEI in sibling card 5.F1b).
```
What is wrong: Both claims are false. The Lead merge happened (the unified tree exists), but the hoist never did: `h264/parser/bitreader.rs` does not exist, slice.rs and nal.rs each carry a full MSB-first + Exp-Golomb reader, and sei.rs:47 `use super::slice::{BitReadError, RbspBitReader};` — so nal.rs's "shared primitive of every syntax reader" is wrong for 2 of its 4 claimed consumers (slice headers AND SEI use slice's reader). The two readers also diverge semantically: nal's `bits()` rejects `n == 0` and `n > 32` with a typed error; slice's `read_bits(0)` silently returns `Ok(0)` and `read_bits(33)` silently truncates the high bit.
Why it matters: Two implementations of the most safety-critical primitive in the parser invite divergence (already present in the n=0/n>32 behavior); a future fix to one (e.g. a golomb edge case) will not reach the other. The docs actively mislead an auditor into thinking there is one reader.
What correct looks like: One reader in `h264/parser/bitreader.rs` (or nal.rs), typed-error compatible, with slice.rs/sei.rs/sps.rs/pps.rs all importing it; delete the duplicate; fix both doc blocks.
Confidence: HIGH — Verification: read both readers end-to-end; rg RbspBitReader users (slice.rs, sei.rs only).
### F-004 [S4] `read_se` documents an unreachable error; dead clamp; unvalidated pub accessors can shift-overflow
Where: crates/codecs/src/video/h264/parser/slice.rs:152-169, 326-333
Code:
```
    /// Read one `se(v)` signed Exp-Golomb code (§9.1.1).
    ///
    /// # Errors
    /// [`BitReadError::GolombOverflow`] when the mapped value exceeds
    /// `i32::MIN..=i32::MAX`, plus whatever
    /// [`RbspBitReader::read_ue`] reports.
```
```
    pub fn max_pic_order_cnt_lsb(&self) -> u32 {
        1u32 << (u32::from(self.log2_max_pic_order_cnt_lsb_minus4) + 4)
    }
```
What is wrong: (a) read_ue can return at most 2³²−2 (31 leading zeros ⇒ value ≤ 2³¹−1+2³¹−1), so the se mapping always lands in −2147483647..=2147483647 — the documented GolombOverflow for se(v) cannot fire, and the odd-arm `.min(i32::MAX as u64)` clamp at :162 is dead code. (b) `max_pic_order_cnt_lsb`/`max_frame_num` shift left by `field+4` with no validation inside the accessor; a struct built out-of-range (field ≥ 28) panics in debug (`1u32 << 32`) or silently wraps in release. Only `parse_slice_header`'s entry (:744) validates; the accessors are `pub` and trust their caller.
Why it matters: Doc-promised errors that cannot occur and panicking pub accessors are latent traps for the next consumer (hevc/parser/slice.rs already mirrors this seam pattern). Zero impact today: every in-tree caller goes through the validated entry.
What correct looks like: Delete the dead clamp, reword the se doc ("cannot fail given read_ue's range"), and either make the accessors saturating or document the precondition.
Confidence: HIGH — Verification: read; range math re-derived (see §2).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-30 | REAL-UNPROVEN | — | Damage-model claims verified by baseline tests (truncated/mutated/corpus all ok). |
| `RbspBitReader` (struct) | 40-44 | REAL-UNPROVEN | — | `bit_pos: usize`; Copy; no panics in any method (see read_bit note). |
| `BitReadError` (enum) | 48-63 | REAL-UNPROVEN | — | Truncated{needed,available}/GolombOverflow — honest variants. |
| `impl Display/Error for BitReadError` | 65-80 | REAL-UNPROVEN | — | Straightforward. |
| `RbspBitReader::new` | 84-86 | REAL-UNPROVEN | — | Trivial. |
| `RbspBitReader::bits_left` | 89-94 | REAL-UNPROVEN | — | saturating_mul/sub — 32-bit safe. |
| `RbspBitReader::bit_pos` | 97-99 | REAL-UNPROVEN | — | Diagnostic. |
| `RbspBitReader::read_bit` | 101-112 | REAL-UNPROVEN | — | Non-saturating `len()*8` at :102 (vs bits_left) — on 32-bit a ≥512MiB RBSP wraps the guard EARLIER (fail-closed truncation, no OOB); 64-bit immune. Note only. |
| `RbspBitReader::read_bits` | 114-125 | REAL-UNPROVEN | S4 | `n==0` → Ok(0), `n>32` → silent high-bit loss (nal's twin rejects both) — folded into F-003/F-004. |
| `RbspBitReader::read_ue` | 127-150 | REAL-UNPROVEN | — | Correct §9.1; >31 zeros typed; u64 math; max 2³²−2 fits u32 (auditor-verified). |
| `RbspBitReader::read_se` | 152-169 | REAL-UNPROVEN | S4 | Correct Table 9-3 mapping; documented error unreachable + dead clamp (F-004). |
| `SliceHeaderError` (enum) | 173-220 | REAL-UNPROVEN | — | 12 typed variants; every parse failure mode named. |
| `impl Display/Error/From for SliceHeaderError` | 222-273 | REAL-UNPROVEN | — | Complete match arms; From wires Bit→Slice. |
| `PocError` + impls | 276-297 | REAL-UNPROVEN | — | Two overflow variants for §8.2 helpers (part3's consumers). |
| `SliceSpsInfo` (struct) | 301-322 | REAL-UNPROVEN | S3 | The F1a/F1b seam; delta flag field neutered in production (F-002). |
| `SliceSpsInfo::max_pic_order_cnt_lsb` | 326-328 | REAL-UNPROVEN | S4 | 1<<(x+4); shift-panic on unvalidated x≥28 (F-004b). |
| `SliceSpsInfo::max_frame_num` | 331-333 | REAL-UNPROVEN | S4 | Same pattern. |
| `SliceSpsInfo::chroma_array_type` | 337-343 | REAL-UNPROVEN | — | Correct §7.4.2.1.1 rule. |
| `SliceSpsInfo::validate` | 345-363 | REAL-UNPROVEN | — | Bounds all four risky fields; called at parse entry :744. |
| `SlicePpsInfo` + `validate` | 368-402 | REAL-UNPROVEN | — | Only weighted_bipred_idc bounds-checked (others are u32 with documented ranges; num_slice_groups handled at parse entry). |
| `SliceNalInfo` + `from_header_byte` | 407-445 | REAL-UNPROVEN | — | Correct 0x80/1/5 logic; nal_ref_idc=0 accepted (legal). |
| `SliceKind`/`SliceType` + `from_code`/`from_reader`/`is_inter_predictive`/`is_intrinsic` | 449-505 | REAL-UNPROVEN | — | Table 7-7 verified; `is_intrinsic` should be `is_intra` (H.264 vocabulary) — naming nit. |
| `PicOrderSyntax` (enum) | 508-533 | REAL-UNPROVEN | — | Carrier; type-1 None-variant design documented. |

## 5. Test honesty (tests live at :1189+, outside this part, but exercise it)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| slice_type_mapping_covers_all_ten_codes_with_aliases (:1481; baseline :620) | YES | Exhaustive 0..=9 mapping + >9 error — my from_code is fully pinned. |
| truncated_headers_are_typed_never_panics (:1848; baseline :634) | YES | Bit-by-bit truncation of hand-built headers; an OOB read_bit panics → test fails. |
| mutated_slice_streams_are_typed_or_ok_never_panics (:2069; baseline :621) | YES | Random mutations over corpus streams; read_ue/read_se edge paths hammered. |
| corpus_0014/0019 vs ffmpeg trace (:614/:618) | YES | Field-by-field equality against externally-derived truth — the strongest evidence class in this repo. |
| (gap) | — | NO dedicated unit test for read_ue's 31-zero boundary or read_se's extremes; covered only transitively by mutation tests. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "at Lead merge it should hoist to h264/parser/bitreader.rs and serve F1a's SPS/PPS/NAL parsers too" | slice.rs:37-39 | FALSE | No bitreader.rs exists (ls parser/); nal.rs:598 has its own BitReader; sei.rs:47 imports slice's. |
| "the shared primitive of every h264::parser syntax reader" (nal.rs BitReader) | nal.rs:592-593 | FALSE | slice.rs + sei.rs use RbspBitReader, not nal::BitReader. |
| "No panic on any input; loops always consume bits, so they terminate" | slice.rs:29-30 | REAL | Baseline :634/:621 (truncated/mutated never-panic suites) — for inputs reaching this parser. (Accessors-on-unvalidated-struct panic is outside "input" scope, F-004b.) |
| "Every failure is a typed SliceHeaderError" | slice.rs:23-24 | REAL | 12-variant taxonomy; all baseline error-path tests ok. |

## 7. Dead / duplicated / unreachable code
- Unreachable: the entire part (F-001) in shipped builds; in-tree consumers are decode.rs (hw, unreachable) + tests + fuzz target.
- Duplicated: `RbspBitReader` vs `nal::BitReader` (F-003); `SliceSpsInfo::max_*` vs `SeqParameterSet::max_*` (sps.rs:373-386, near-identical bodies — intentional per sps.rs comment "kept beside the field it derives from", but two sources of truth for the same spec constants).

## 8. Dependencies & flags
`core::fmt` only — no deps, no unsafe, no feature gates; compiled unconditionally (video/mod.rs:29 `pub mod h264;`). Wave-6 hostile question impact: this parser is NOT the shipped H.264 decode path — the shipped path is rust_h264 behind `codec-h264-sw` (default-on), so H.264 decode capability claims must NOT cite this file as their engine.

## 9. Open questions for the next agent
- part2 (A06): `parse_slice_header` :738-760 — confirm `first_mb_in_slice`/`pps_id` reads and the FMO refusal ordering; check `MAX_OPS_PER_LOOP = 256` (:730) against the allocation-bounds comment.
- part3/part4 (A05/A03): POC helpers (:1122-1188) consume `max_pic_order_cnt_lsb()` — verify they call validate() or construct only via parse (F-004b blast radius).
- decode.rs (unassigned in wave 6?): the AU assembler's use of `SliceSpsInfo` candidates (:504) — the delta-flag fix (F-002) lands there + sps.rs:252.
