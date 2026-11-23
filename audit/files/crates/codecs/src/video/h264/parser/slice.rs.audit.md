# AUDIT: crates/codecs/src/video/h264/parser/slice.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 2123 / 45 (24 prod incl. impls, 21 test-only) |
| File verdict | REAL (parser verified by execution) — but UNREACHABLE from shipped entry points (see §2/§7) |
| Reachable from shipped entry point? | NO — only production caller `H264Decoder::parse_vcl` (decode.rs:516) is never constructed by any shipped code path; CLI h264 lane uses `rust_h264` (sw.rs) instead |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 2 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 video::h264::parser::slice --nocapture` -> 9 passed / 0 failed (4 of 9 silently skipped, corpus absent); `ls /tmp/my-project/corpus-v4/media` -> empty; `which ffmpeg ffprobe` -> both present |

## 1. One-paragraph verdict
This is a genuinely careful, spec-faithful H.264 slice-header parser: bit reader, Exp-Golomb, all §7.3.3 gates (POC types 0/1/2, weighted prediction with chroma_array_type gating, ref-list mods, MMCO, CABAC init, deblock), FMO typed refusal, bounded loops, checked POC arithmetic. The unit tests I executed (hand-built streams, invalid-value sweep, POC helpers, slice-type table) are real and would fail under mutation. But: (a) the only production call site is inside `H264Decoder` (the hw lane), which no shipped entry point constructs — the CLI registers `H264SwDecoder` (the external `rust_h264` engine) for "h264"/"avc1", and `H264Decoder::from_setting` appears only in comments (apps/cli/src/pipeline_host.rs:342); (b) 4 of 9 tests — the ones carrying the strongest evidence claims ("matches the ffmpeg trace", truncation sweep, 240-round mutation property) — silently skip because the external corpus directory does not exist, so the baseline "4744 passed" counts them as passes while they assert nothing; (c) several error-path defects: wrong error type for deblock idc, mislabeled truncation errors as "budget exceeded", a documented-but-unreachable `read_se` error, and a hand-built IDR test whose bitstream omits the dec_ref_pic_marking bits so the "slice_qp_delta 3" comment is false.

## 2. What actually works (verified)
— `./target/debug/deps/codecs-bae0f1756cc0a809 video::h264::parser::slice --nocapture` -> 9 passed, 0.01s. Real tests that ran: `slice_type_mapping_covers_all_ten_codes_with_aliases`, `hand_built_headers_exercise_the_optional_syntax_gates`, `idr_header_with_colour_planes_and_poc_type2`, `invalid_values_are_the_typed_error_paths`, `poc_arithmetic_helpers_follow_the_spec_rules`. These assert exact parsed values and error variants; they would fail if the parser were broken. Skipped vacuously: `corpus_0019_...`, `corpus_0014_...`, `truncated_headers_...`, `mutated_slice_streams_...` (output: "corpus: 0019-raw-video.h264 not present — skipping").
— Spec-order review: POC type-0 delta_bottom gate (`pps.bottom_field_pic_order_in_frame_present_flag && !field_pic`, L788), type-1 delta1 gate (L802), weighted-pred gating `(weighted_pred_flag && P/SP) || (weighted_bipred_idc==1 && B)` (L885-886), chroma weights gated on `chroma_array_type != 0` (L1063) — all match §7.3.3/§7.3.3.2; 4:2:2 (chroma_format_idc=2) flows through the same gate correctly. `num_ref_idx_*_active_minus1` bounded to 0..=31 on both the override (L833/L838) and the PPS defaults (L854/L860), which bounds weight-table allocation at 32 entries/list — no unbounded alloc.

## 3. Findings
F-001 [S2]
Where: slice.rs:1512-1516, 1849-1851, 2070-2072 (skip guards), 1399-1404 (corpus gate)
Code: (L1513-1516)
    let Some((sps, pps, slices)) = corpus_0019() else {
        eprintln!("corpus: 0019-raw-video.h264 not present — skipping");
        return;
    };
What is wrong: 4 of the module's 9 tests — including the two named "matches_the_ffmpeg_trace" (26/185-slice counts, adaptive=42, qp ranges) and the hostile-input tests (byte-by-byte truncation sweep; 240-round mutation property) — silently return when the external corpus is absent. `/tmp/my-project/corpus-v4/media` does not exist here (verified `ls`), so in this environment and in any CI without the corpus these tests pass while asserting nothing. Why it matters: the module's headline evidence claims (ffmpeg-trace parity, panic-free mutation sweeps) are carried exclusively by tests that never run; the orchestrator baseline "4744 passed" includes these vacuous passes — evidence inflation by skip-guard. What correct looks like: `#[ignore]` + explicit corpus gate reported as ignored (visible in summary), or check the corpus into the repo (the repo already ships h264 fixtures, e.g. crates/codecs/tests/fixtures/h264/D_high_ipb_640x360_45f.h264, which this module does not use).
Confidence: HIGH — Verification: ran (skip lines printed in test output).

F-002 [S3]
Where: slice.rs:923-928 (vs enum at 193-194)
Code:
    let deblocking = if pps.deblocking_filter_control_present_flag {
        let idc = r.read_ue()?;
        if idc > 2 {
            // 3 was removed from the vocabulary by Corigendum 1;
            // treat out-of-range as the typed invalid-value class.
            return Err(SliceHeaderError::InvalidSliceType(idc));
What is wrong: out-of-range `disable_deblocking_filter_idc` returns `InvalidSliceType(idc)` — the wrong variant. `SliceHeaderError::InvalidDeblockIdc(u32)` is declared (L194) and rendered (L242) but constructed nowhere (rg across crates: only L194/L242). A deblock idc of 3 surfaces as "slice_type 3 outside 0..=9 (Table 7-7)" — an actively false diagnostic naming a different syntax element. Why it matters: the "typed, never-mislabeled" damage model the module doc promises is violated on this path; any caller distinguishing error kinds gets the wrong class. What correct looks like: `return Err(SliceHeaderError::InvalidDeblockIdc(idc))`.
Confidence: HIGH — Verification: read + rg (`InvalidDeblockIdc` -> only definition and Display arm).

F-003 [S3]
Where: slice.rs:1029-1037 (and twin 1040-1048)
Code:
    for idx in 0..=num_ref_idx_l0_active_minus1 {
        table
            .l0
            .push(read_pred_weight_entry(r, chroma_array_type).map_err(|_| {
                SliceHeaderError::OpBudgetExceeded {
                    kind: "pred_weight_l0",
                    ops: idx as usize + 1,
                }
            })?);
What is wrong: every `BitReadError` from a weight entry (truncated RBSP, golomb overflow) is rewritten to `OpBudgetExceeded` with kind "pred_weight_l0"/"pred_weight_l1". No budget exists on these loops — they are statically bounded ≤ 32 iterations by the num_ref_idx validation; the real failure is truncation. The map_err discards the true error class. Why it matters: a hostile truncated stream inside pred_weight_table is misdiagnosed as a loop-budget trip; the module's own doc (L202-203) reserves `OpBudgetExceeded` for "a malformed-stream loop budget tripped", which is false here. What correct looks like: propagate the `BitReadError` via `?` (From impl already exists).
Confidence: HIGH — Verification: read.

F-004 [S3]
Where: slice.rs:152-169
Code:
    /// # Errors
    /// [`BitReadError::GolombOverflow`] when the mapped value exceeds
    /// `i32::MIN..=i32::MAX`, plus whatever
    /// [`RbspBitReader::read_ue`] reports.
    pub fn read_se(&mut self) -> Result<i32, BitReadError> {
        let k = u64::from(self.read_ue()?);
        ...
        let mapped: i64 = if k % 2 == 1 {
            ((k + 1).div_ceil(2).min(i32::MAX as u64)) as i64
        } else {
            -(k as i64 / 2)
        };
What is wrong: the documented GolombOverflow error is unreachable. Odd k is clamped with `.min(i32::MAX as u64)` (codeNum 2^32−1 should map to +2^31; it is silently clamped to 2147483647), and even k tops out at −(2^31−1) which fits i32 — so `i32::try_from` at L166 never fails. The doc comment asserts an error path that cannot fire, and the clamp silently distorts an adversarial value instead of refusing typed. Why it matters: parse-level consumers get a wrong-but-plausible `se(v)` value (e.g. slice_qp_delta = i32::MAX) with no error; doc and code disagree. What correct looks like: return `GolombOverflow` when `(k+1)/2 > i32::MAX` instead of clamping (or fix the doc and drop the dead branch).
Confidence: HIGH — Verification: read (both k parities exhaustively reasoned over u32 range).

F-005 [S3]
Where: slice.rs:1823-1824 (fixture) vs asserts 1836-1844
Code:
        w.ue(9); // idr_pic_id
        w.se(3); // slice_qp_delta
        w.rbsp_trailing();
What is wrong: this hand-built IDR stream (nal_ref_idc=3, so §7.3.3 requires `dec_ref_pic_marking` = 2 flag bits after idr_pic_id) omits those bits. The parser consumes the first two bits of `se(3)` (= '00') as no_output_of_prior_pics_flag/long_term_reference_flag, then reads slice_qp_delta from the remaining '110…' → parses 0, not 3. The test passes only because it never asserts `slice_qp_delta`. The comment "// slice_qp_delta" describes bits that are actually consumed as marking flags. Why it matters: the fixture does not encode what its comments claim; a broken qp-delta read would not fail this test — the gate it claims to exercise is partially untested (mutation in slice_qp_delta decode is invisible here). What correct looks like: `w.put(0,1); w.put(0,1);` (marking flags) before `w.se(3)`, plus `assert_eq!(h.slice_qp_delta, 3)`.
Confidence: HIGH — Verification: read + hand-decoded the bit stream (se(3)='00'+'110'; parser consumes '0','0' as flags, then ue '1…' → 0).

F-006 [S4]
Where: slice.rs:619-622
Code:
/// One MMCO operation (§8.2.5.4 command table; syntax per
/// §7.3.3.4 — operand mapping verified against ffmpeg
/// `trace_headers` on the corpus: op 1 reads only
/// `difference_of_pic_nums_minus1`, op 0 is the terminator).
What is wrong: the verification claim is UNVERIFIABLE in this environment (corpus absent, so the trace-parity tests skip); code comments assert evidence that cannot be checked here. What correct looks like: point at a committed transcript rather than an absent corpus.
Confidence: HIGH that it is unverified — Verification: UNVERIFIED (corpus-gated).

F-007 [S4]
Where: slice.rs:316-318 vs validate() 345-363
Code:
    /// `separate_colour_plane_flag` (only settable when
    /// `chroma_format_idc == 3`).
...
    fn validate(&self) -> Result<(), SliceHeaderError> {
What is wrong: `validate()` checks log2 fields, poc type, chroma_format_idc ≤ 3, but never enforces the documented constraint `separate_colour_plane_flag ⇒ chroma_format_idc == 3`; an inconsistent context parses instead of returning `InvalidContext`. Blast radius: a wrong colour_plane_id read position on such streams. What correct looks like: one more arm in validate().
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| RbspBitReader::new/bits_left/bit_pos | 84-99 | REAL | — | trivial, correct |
| RbspBitReader::read_bit | 101-112 | REAL | — | MSB-first, bounds-checked |
| RbspBitReader::read_bits | 118-125 | REAL | — | correct for n≤32 incl. 32 |
| RbspBitReader::read_ue | 131-150 | REAL | — | 31-zero cap, u32 checked |
| RbspBitReader::read_se | 158-169 | REAL | S3/F-004 | doc error path dead; clamp distorts |
| Display BitReadError / SliceHeaderError / PocError | 65-78, 222-297 | REAL | — | accurate except dead variant F-002 |
| SliceSpsInfo::max_pic_order_cnt_lsb / max_frame_num / chroma_array_type | 326-343 | REAL | — | shifts bounded by validate |
| SliceSpsInfo::validate | 345-363 | REAL | S4/F-007 | missing colour-plane constraint |
| SlicePpsInfo::validate | 396-401 | REAL | — | only weighted_bipred_idc; defaults range-checked later at 854-865 |
| SliceNalInfo::from_header_byte | 423-444 | REAL | — | VCL 1/5 only, forbidden bit |
| SliceType::from_code/from_reader/is_inter_predictive/is_intrinsic | 479-504 | REAL | — | Table 7-7 exact, tested |
| parse_slice_header | 738-971 | REAL | S3/F-002 | full §7.3.3; all gates verified vs spec; deblock error mislabeled |
| read_ref_pic_list_mod_ops | 974-1007 | REAL | — | idc 0..=5 + 3-terminator, budget 256 |
| read_pred_weight_table | 1010-1052 | REAL | S3/F-003 | bounded ≤32+32 entries; error mislabeled |
| read_pred_weight_entry | 1055-1069 | REAL | — | luma/chroma flags per §7.3.3.2 |
| read_mmco_ops | 1072-1112 | REAL | — | idc 0..=6, End kept in list |
| pic_order_cnt_msb_type0 | 1122-1136 | REAL | — | §8.2.1.1 exact (>= / > asymmetry right) |
| next_frame_num_offset | 1144-1161 | REAL | — | §8.2.1 wrap rule, checked |
| pic_order_cnt_type2 | 1169-1187 | REAL | — | §8.2.1.3 incl. −1 non-ref |
| tests: annexb_nals / strip_emulation_prevention | 1199-1251 | REAL (test-only) | — | production uses parser::nal equivalents instead |
| tests: bootstrap_sps / bootstrap_pps | 1257-1343 | REAL (test-only) | — | unwrap_or_default swallows truncation; assert! panics on scaling/FMO (test-only, acceptable) |
| tests: BitWriter (new/put/ue/se/rbsp_trailing) | 1346-1397 | REAL (test-only) | — | correct ue/se encodings (decoded by hand for F-005) |
| tests: corpus_path / corpus_0019 / corpus_0014_annexb / ffprobe_frame_count | 1399-1478 | SHALLOW | S2/F-001 | skip silently; ffmpeg present but corpus absent |
| tests: 9 #[test] fns | 1480-2123 | 5 REAL / 4 SHALLOW | F-001, F-005 | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| slice_type_mapping_covers_all_ten_codes_with_aliases | YES (ran) | exact table incl. aliases + invalid codes |
| corpus_0019_slice_headers_match_the_ffmpeg_trace | NO (in this env) | early-returns; corpus absent — vacuous pass |
| corpus_0014_mkv_lane_matches_the_ffmpeg_trace | NO (in this env) | same skip guard |
| hand_built_headers_exercise_the_optional_syntax_gates | YES (ran) | asserts exact values across ~15 fields; strongest test in file |
| idr_header_with_colour_planes_and_poc_type2 | PARTIAL (ran) | colour_plane/frame_num/idr/POC2/marking asserted; slice_qp_delta NOT asserted and actually parses as 0 vs comment "3" (F-005) |
| truncated_headers_are_typed_never_panics | NO (in this env) | skip guard |
| invalid_values_are_the_typed_error_paths | YES (ran) | exact Err variant equality on 8 paths |
| poc_arithmetic_helpers_follow_the_spec_rules | YES (ran) | exact Ok/Err incl. overflow |
| mutated_slice_streams_are_typed_or_ok_never_panics | NO (in this env) | skip guard |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no entropy-coded macroblock data is touched" | slice.rs:5-7 | REAL | parse ends at deblock control; header_bits records position (L969) |
| "No panic on any input; loops always consume bits" | slice.rs:29-30 | REAL-UNPROVEN | mutation test exists but skips here (F-001); manual review of all 3 loops found no panic path; UNVERIFIED by execution |
| "operand mapping verified against ffmpeg trace_headers on the corpus" | slice.rs:620-622 | UNVERIFIED | corpus absent; F-006 |
| read_se "# Errors GolombOverflow …" | slice.rs:152-157 | FALSE | error branch unreachable (F-004) |
| FMO "absent from every real corpus stream" | slice.rs:27-28 | UNVERIFIED | no corpus to check; typed refusal is honest either way |
| registry/CLI: h264 decodes in shipped build | apps/cli/src/pipeline_host.rs:346 | REAL but via rust_h264, NOT this parser | sw.rs uses external engine; this parser only feeds the unconstructed hw lane |

## 7. Dead / duplicated / unreachable code
- `SliceHeaderError::InvalidDeblockIdc` — declared + displayed, never constructed (F-002).
- `parse_slice_header` production reachability: sole prod caller `H264Decoder::parse_vcl` (decode.rs:516). `H264Decoder` is exported (h264/mod.rs:58) but constructed nowhere in apps/ or production crates — `from_setting` is only referenced in a comment (apps/cli/src/pipeline_host.rs:342); the shipped h264 lane instantiates `H264SwDecoder` (pipeline_host.rs:346) which wraps the external `rust_h264` engine and imports only `parser::nal` (sw.rs:92), not this slice parser. Verdict: UNREACHABLE from shipped entry points as wired today (library-exported, settings-gated hw lane not constructed).
- Test-only `annexb_nals`/`strip_emulation_prevention` duplicate the production `parser::nal` equivalents (deliberate test seam per module docs; duplication noted).

## 8. Dependencies & flags
No external deps (core::fmt + std only). No feature gates on this file; reachability loss is wiring, not cfg. Test corpus knob `LUMEN_H264_CORPUS` (L1400) defaults to /tmp/my-project/corpus-v4/media (absent here).

## 9. Open questions for the next agent
- decode.rs / hw/mod.rs / vaapi session audit: is `H264Decoder` (hw lane) reachable via ANY settings path in the desktop app, or is the whole hw lane (and this parser with it) dead shipped code? (pipeline_host.rs:342 suggests Phase-7 opt-in never landed.)
- If someone lands corpus-v4 in CI, re-run the 4 skipped tests; the trace constants (26/185 slices, adaptive=42/7, qp ranges) are otherwise unchecked numbers.
