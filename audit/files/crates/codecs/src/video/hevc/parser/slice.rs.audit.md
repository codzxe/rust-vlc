# AUDIT: crates/codecs/src/video/hevc/parser/slice.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 494 / 7 production fns + 9 tests |
| File verdict | REAL (spec-correct minimal parser, tests run live) — but UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | **NO** — callers are hevc/decode.rs + hevc/hw.rs only; `HevcDecoder` has no `"hevc"` arm in apps/cli/src/pipeline_host.rs (see W06-A05's audit of hevc/decode.rs, same conclusion); nothing outside crates/codecs tests constructs the chain |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
A clean, honest minimal HEVC slice-segment-header prefix parser (§7.3.6.1 up to `slice_pic_order_cnt_lsb`): the syntax walk order matches the spec exactly (first-slice flag → IRAP-gated `no_output_of_prior_pics_flag` → bounded PPS id ue → non-first dependent flag + CTB address → independent-segment shared prefix with reserved bits, slice_type range-gated, output flag, colour plane id, IDR-vs-CRA POC-lsb nuance), every gate is an `Option` rather than a guessed value, and the module doc states precisely what is NOT parsed. I verified the bit-level walk against the spec by reading and ran its 9 tests live (pass, incl. a real-fixture SPS/PPS seam derivation). Its only sins: the whole HEVC lane is dead code in the shipped build (no `"hevc"` decoder arm in the CLI), and the `clamp(1, 32)` guards diverge from the spec's v=0 inference in a corner no valid stream can reach.

## 2. What actually works (verified)
- `timeout 100 ./target/debug/deps/codecs-bae0f1756cc0a809 hevc --test-threads=2` → `21 passed; 0 failed` (the slice module's 9 tests included: IDR/no-POC, TRAIL_R gated flags, non-first CTB address, dependent-segment early stop, CRA-POC nuance + extra header bits, typed errors, seam-vs-fixture, ceil_log2 table, AnnexB classification).
- Spec-order verification by read (§7.3.6.1): each gated field read condition matches the standard's syntax table; IDR_W_RADL/IDR_N_LP POC-skip vs CRA-read distinction is correct (L229-235).

## 3. Findings

F-001 [S3 — the module is unreachable from every shipped entry point]
Where: slice.rs:167-182 (the public parse API)
Code:
```
    pub fn parse(
        nal: &[u8],
        sps: &SliceSegmentSpsInfo,
        pps: &SliceSegmentPpsInfo,
    ) -> Result<SliceSegmentHeader, HevcError> {
```
What is wrong: nothing in the function — the problem is the call graph. `SliceSegmentHeader::parse` is called only by hevc/decode.rs:490 and hevc/hw.rs (via the same AU assembler); `HevcDecoder` is constructed only by crates/codecs tests (rg over apps/: zero refs). The shipped CLI's decode factory has no `hevc` arm, so `lumen open` on an HEVC track fails at `cli.codec_unavailable` before any of this parses a byte (orchestrator baseline mp4-smoke.md, exit 65). The parser compiles into the shipped binary (hevc module is unconditional per video/mod.rs) but never runs in production.
Why it matters: parser-only lanes are the repo's honest pattern (documented), but 494 loc of spec code shipped dark is dead weight until the registration card lands.
Confidence: HIGH — Verification: rg (callers) + prior agent's independent reachability audit of hevc/decode.rs (same conclusion).

F-002 [S4 — clamp guards diverge from spec in unreachable corners]
Where: slice.rs:207, 234
Code:
```
                slice_segment_address = Some(r.bits(sps.slice_segment_address_bits.clamp(1, 32))?);
```
What is wrong: per §7.3.6.1, `slice_segment_address` has width `Ceil(Log2(PicSizeInCtbsY))` and is *not coded* when that is 0 (single-CTB picture); the code clamps 0 → 1 and would read a bit. Unreachable for valid streams (a 1-CTB picture cannot have a non-first segment), and the upper clamp 32 can never bind for any level-legal geometry (PicSizeInCtbsY ≤ 512² → 18 bits). Same shape for the POC-lsb clamp. Defensive, but it implements "read 1 bit" where the spec says "infer 0".
Confidence: HIGH (divergence) / impact-nil. Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SliceSegmentSpsInfo::from_sps | 42-49 | REAL | — | derives from parsed SPS; CTB arithmetic propagated |
| ceil_log2 | 53-60 | REAL | — | power-of-two exact; table-tested |
| SliceSegmentPpsInfo::from_pps | 78-84 | REAL | — | straight projection |
| SliceType::from_id / id | 102-119 | REAL | — | 0/1/2 B/P/I, others None |
| SliceSegmentHeader::parse | 167-182 | REAL | S3 | NAL split, VCL gate, EPB unescape, delegates to read |
| SliceSegmentHeader::read | 185-252 | REAL | S4 | §7.3.6.1 prefix walk — spec-exact; F-002 clamps |
| slice_type_raw | 256-266 | REAL | — | ue + typed range check |
| tests ×9 | 292-493 | REAL | — | bit-built NALs; all pass live; real-fixture seam cross-check |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| idr_first_slice_reads_the_honest_prefix | YES | hand-built bits assert every field incl. POC absence |
| trail_r_b_slice_reads_poc_and_gated_flags | YES | gated flags + 8-bit POC lsb values |
| non_first_independent_segment_reads_ctb_address | YES | CTB address bits + P slice |
| dependent_segment_stops_before_the_shared_syntax | YES | prefix-boundary move for dependent segments |
| extra_slice_header_bits_and_cra_nuance | YES | CRA reads POC, IDR doesn't — the subtle case |
| typed_error_paths | YES | non-VCL / pps-id range / slice-type range / truncation |
| seam_derivation_matches_the_0100_sps | YES | real SPS/PPS fixture → 10 address bits, 8 POC bits |
| ceil_log2_table | YES | boundary values |
| annexb_walk_of_vcl_and_parameter_sets_classifies | YES | integration smoke of header split + is_vcl |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Everything after slice_pic_order_cnt_lsb [is] deliberately NOT parsed" | slice.rs:7-17 | TRUE | read walk stops there; no trailing-bits check (stated) |
| "minimal honest subset of card 5.G1" | slice.rs:1-5 | TRUE | field set matches code exactly |
| (matrix/docs claims that hevc parsing is "landed") | other agents' scope | — | the parser is landed but production-unreachable (F-001) |

## 7. Dead / duplicated / unreachable code
Whole module production-unreachable (F-001); exercised only by tests (hevc_hw_path 17 passed per W06-A05 + lib tests here).

## 8. Dependencies & flags
hevc module unconditional in codecs (video/mod.rs:41-43); no external deps; pure std.

## 9. Open questions for the next agent
- None for this file. The registration card (doc 01 §2.5(a)) is the gate that would make this reachable — worth checking whether any matrix row already claims HEVC "supported" (the shipped CLI error text listing codecs at pipeline_host.rs:377 does NOT list hevc, which is honest).
