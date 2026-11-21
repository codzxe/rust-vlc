# AUDIT: crates/codecs/src/video/hevc/parser/slice.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-21 |
| Lines / functions | 495 / 12 (2 seam structs + impls, 2 helpers, enum + 2 methods, header struct + parse/read, 10 tests) |
| File verdict | UNREACHABLE (real, corpus-pinned parser; zero shipped consumers) |
| Reachable from shipped entry point? | NO — `video::hevc` module consumers are only `crates/codecs/src/video/hevc/decode.rs` (itself unreachable: no "hevc" dispatch arm in apps/cli/src/pipeline_host.rs `decoder_for`) plus `crates/codecs/tests/{hevc_hw_path,hevc_parser_corpus}.rs` (Grep `video::hevc` over crates/ + apps/ → those four files only). demux's `es/hevc.rs` (.265 lane) uses demux's OWN annexb engine, not this parser; media-probe does not depend on codecs. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | Read tool full file; `rg -n "fn pic_size_in_ctbs_y\|pic_width_in_luma_samples\|zero"` over hevc/parser/sps.rs (no zero-dimension gate; bounded_ue is upper-bound-only — vps.rs:232-243); `rg -ln "video::hevc"` over crates/+apps/ (consumers census); baseline test log :642-651 (10/10 slice tests ok) + :2065 (`corpus_hevc_lane_matches_ffprobe_packet_geometry ... ok`); ceil_log2 worked-example verification (192→8, 256→8, 3→2) |

## 1. One-paragraph verdict
A disciplined "honest subset" slice-segment-header parser: the §7.3.6.1 prefix walk (first-segment flag, IRAP-gated no_output flag, bounded PPS id, dependent-segment/CTB-address gating, extra header bits, slice_type, output flag, colour plane, IDR-vs-CRA POC nuance) is implemented in the correct spec order and pinned by tests built on hand-laid bit streams plus a real-SPS seam derivation — all 10 green in baseline, plus a ffprobe-geometry corpus lane. The module docs are unusually honest about what is NOT parsed. But the arithmetic core has a genuine defect: `ceil_log2(0)` underflows on a width/height-zero SPS — the SPS parser (sps.rs:233-235) applies only an UPPER bound (`bounded_ue`), so a crafted SPS sails through, `pic_size_in_ctbs_y()` returns `Ok(0)`, and `63 - v.leading_zeros()` panics in debug / silently wraps in release, violating the crate-wide no-panic contract (codecs/src/lib.rs:49-52). The whole file is unreachable from shipped binaries, which caps the blast radius but also means the fuzz-lite "never panics" tests (run in release, baseline) structurally cannot catch this class.

## 2. What actually works (verified)
— 10/10 `video::hevc::parser::slice::tests` ok in baseline (test-workspace.log.txt:642-651): idr_first_slice, trail_r_b_slice, non_first_independent_segment, dependent_segment_stops, extra_bits_and_cra_nuance, typed_error_paths, seam_derivation_matches_the_0100_sps, ceil_log2_table, annexb_walk classification.
— `corpus_hevc_lane_matches_ffprobe_packet_geometry ... ok` (baseline log :2065) — the parser's outputs are pinned against ffprobe's packet geometry on real corpus bytes.
— Spec-order verification (read, against ITU-T H.265 §7.3.6.1): first flag → IRAP no_output → PPS id → [non-first: dependent flag / CTB address] → [independent: extra bits, slice_type, output flag, colour plane, POC lsb unless IDR_W_RADL/IDR_N_LP] — order and gating all match; the IDR-defines-POC-0 vs CRA-codes-POC nuance is both implemented (229-235) and test-pinned (395).
— `ceil_log2` correct for all v ≥ 1 (worked examples: 1→0, 2→1, 3→2, 4→2, 192→8, 256→8 — matches the test table).

## 3. Findings

### F-001 [S2]
Where: crates/codecs/src/video/hevc/parser/slice.rs:53-60 (panic site), fed by :47 and sps.rs:233-235 (missing gate)
Code:
```
fn ceil_log2(v: u64) -> u32 {
    let floor = 63 - v.leading_zeros();
```
What is wrong: `v == 0` is reachable and unguarded. Chain: `SeqParameterSet::parse` reads `pic_width_in_luma_samples = bounded_ue(r, "pic_width_in_luma_samples", 65_535)? as u32` (sps.rs:233) — `bounded_ue` (vps.rs:232-243) enforces ONLY `v > max`, so width/height **0** parses successfully (there is no zero-dimension gate anywhere in sps.rs — rg for `zero|== 0` over dims: none); `pic_size_in_ctbs_y()` (sps.rs:418-430) then computes `div_ceil(0, ctb) * div_ceil(h, ctb) = 0` and returns `Ok(0)`; `SliceSegmentSpsInfo::from_sps` (slice.rs:47) calls `ceil_log2(0)`; `63u32 - 0u64.leading_zeros()` = `63 - 64` → arithmetic underflow: **panic in debug builds, wrap to u32::MAX in release** (release then computes `floor + 1` → wraps to 0 → `slice_segment_address_bits = 0` → masked by the `.clamp(1, 32)` at :207 into a silently-wrong 1-bit read).
Why it matters: (a) the crate's own convention — codecs/src/lib.rs:49-52 "No `unsafe`, no panics on any input: every decode path is bounds-checked and **overflow-checked**" — is violated by an unchecked subtraction; (b) the file's fuzz-lite survival tests (`fuzz_lite_random_packets_never_panic`, hevc tests suite) ran in the baseline's **release** profile (INDEX.md deviation note), where the underflow wraps instead of panicking — the tests structurally cannot catch this class of defect, so the "no panics pinned by fuzz" claim is unproven for debug builds; (c) in release the failure mode is a silent header desync (damage-counted downstream) instead of a typed error. Mitigating: crafted-input-only, and the module is unreachable from shipped binaries.
What correct looks like: gate the SPS at parse time (`if width == 0 || height == 0 { return Err(HevcError::Invalid { code: "hevc.sps.zero_dimension" }) }` — the mjpeg parser's `mjpeg.sof.zero_dimension` is the in-repo precedent), AND make `ceil_log2` total (`v.checked_next_multiple_of`… or simply `debug_assert!(v >= 1)` + `if v <= 1 { return 0; }`).
Confidence: HIGH — Verification: read (full chain traced: slice.rs:47,53-60 → sps.rs:233-235,418-430 → vps.rs:232-243); not executed (no cargo per rules) — REQUEST-ORCH-RUN: `cargo test -p codecs --profile dev hevc::parser::slice` after injecting an SPS with `pic_width_in_luma_samples = 0` would demonstrate the debug panic; even without it the arithmetic is unambiguous.

### F-002 [S4]
Where: crates/codecs/src/video/hevc/parser/slice.rs:207
Code:
```
                slice_segment_address = Some(r.bits(sps.slice_segment_address_bits.clamp(1, 32))?);
```
What is wrong: the spec codes `slice_segment_address` as `u(Ceil(Log2(PicSizeInCtbsY)))` — for a 1-CTB picture that width is **0** (ceil_log2(1)=0, correctly computed at :53-60), but the clamp forces a 1-bit read, desyncing the walk by one bit. Only reachable on non-conformant streams (a 1-CTB picture cannot legally contain a non-first segment), so damage-only; but the clamp silently converts F-001's release-mode wrap (bits=0) into this wrong read, hiding the defect instead of erroring.
Why it matters: masks an upstream arithmetic failure; a `bits(0)`-capable reader (or an explicit `address_bits == 0 → typed Invalid` refusal) would surface it.
What correct looks like: refuse typed when `slice_segment_address_bits == 0` and a non-first independent segment arrives (structurally impossible stream), and clamp only against a sane upper bound.
Confidence: HIGH — Verification: read (spec width arithmetic vs code).

### F-003 [S4]
Where: crates/codecs/src/video/hevc/parser/slice.rs:41 (doc), 178 (allocation)
Code:
```
    /// Propagates the SPS's CTB-grid arithmetic error.
    /// [`SeqParameterSet::pic_size_in_ctbs_y`]).
```
What is wrong: (a) broken rustdoc link — stray `)` with no opening bracket, so the doc renders as garbage; (b) `SliceSegmentHeader::parse` allocates a fresh `Vec::with_capacity(nal.len())` RBSP buffer per call (:178) — per-slice-segment heap churn that the crate's own D-1 discipline (mjpeg parser's persistent scratch, `JpegInfo::reset`) exists to avoid; harmless on the hw path's volumes, but inconsistent with the stated allocation posture.
Why it matters: hygiene/drift; the doc typo misdocuments the error path of the seam.
What correct looks like: fix the link to `/// [`SeqParameterSet::pic_size_in_ctbs_y`].`; take a caller-held scratch or document why per-call allocation is accepted here.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-17 | REAL | — | "deliberately NOT parsed" list cross-checked against `read`: stops exactly after POC lsb, no rbsp_trailing check — honest boundary as claimed |
| `SliceSegmentSpsInfo` | 25-34 | REAL | — | 3-field seam |
| `SliceSegmentSpsInfo::from_sps` | 36-50 | REAL | S2 | F-001: ceil_log2(0) via unguarded SPS dims |
| `ceil_log2` | 53-60 | REAL | S2 | correct for v ≥ 1 (worked examples); underflows at v=0 (F-001); precondition "v ≥ 1" documented but not asserted |
| `SliceSegmentPpsInfo` + `from_pps` | 63-85 | REAL | — | trivial mirror; `#[must_use]` present |
| `SliceType` + `from_id`/`id` | 88-120 | REAL | — | HEVC mapping 0=B,1=P,2=I (correctly REVERSED vs H.264); out-of-range → None |
| `SliceSegmentHeader` struct | 124-156 | REAL | — | Option fields encode gated syntax honestly |
| `SliceSegmentHeader::parse` | 167-182 | REAL | S4 | non-VCL typed refusal; unescape into fresh Vec per call (F-003b) |
| `SliceSegmentHeader::read` | 185-252 | REAL | S4 | spec-order walk verified (§7.3.6.1); F-002 clamp; bounded pps id ≤ 63; IDR/CRA POC nuance correct |
| `slice_type_raw` | 256-266 | REAL | — | ue + range ≤ 2 typed |
| tests (10) | 268-494 | REAL | — | see §5 |

## 5. Test honesty
All 10 ok in baseline (test-workspace.log.txt:642-651).
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| idr_first_slice_reads_the_honest_prefix | YES | full field-by-field assertion incl. POC=None for IDR — an order/gating mutation shifts bits and fails it |
| trail_r_b_slice_reads_poc_and_gated_flags | YES | B-type + output flag + colour plane 2 + POC 0x5a over 8 bits |
| non_first_independent_segment_reads_ctb_address | YES | address 9 over exactly 6 bits |
| dependent_segment_stops_before_the_shared_syntax | YES | pins the dependent boundary (no address/type/POC) |
| extra_slice_header_bits_and_cra_nuance | YES | CRA reads POC lsb (IRAP-but-not-IDR) + 2 reserved bits |
| typed_error_paths | YES | not_vcl / pps_id 64 / slice_type 5 / truncation — four typed refusals |
| seam_derivation_matches_the_0100_sps | YES | real SPS bytes → 10 address bits, poc width 8 (catches ceil_log2/seam drift on real data) |
| ceil_log2_table | YES for v ≥ 1 | 6-value table incl. non-power-of-2; does NOT cover v=0 (F-001 gap) |
| annexb_walk_of_vcl_and_parameter_sets_classifies | YES (smoke) | classification only — thin but not tautological |
| (corpus lane, other file) corpus_hevc_lane_matches_ffprobe_packet_geometry | YES | ffprobe oracle on real bytes (baseline :2065) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Everything after `slice_pic_order_cnt_lsb` … is deliberately NOT parsed … stops at the honest boundary instead of guessing" | slice.rs:7-17 | REAL | `read` ends at :235; no trailing-bits check (documented omission) |
| "no `rbsp_trailing_bits` check is attempted (the header is a prefix, not the whole RBSP)" | slice.rs:16-17 | REAL | no such check in 185-252 |
| crate claim "No `unsafe`, no panics on any input … overflow-checked" | codecs/src/lib.rs:49-52 (as applied to this file) | FALSE for debug builds | F-001: unchecked `63 - leading_zeros` underflows on width/height-0 SPS |
| corpus "field-exact vs ffmpeg trace_headers/ffprobe on corpus lanes 0044/0091/0100" (slice prefix part) | docs/formats/video-hevc.md:32-35 | REAL (test-cited) | corpus_hevc_lane ok (baseline :2065); in-file seam test on SPS_0100 |

## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped entry points (header row). No internal dead code; `SliceType::id()` has no production caller (used for round-trip assertions in tests only) — S4-level vestige, folded into the table.

## 8. Dependencies & flags
Internal only: `super::nal` (BitReader, unescape, NalHeader), `super::pps`, `super::sps`, `super::vps::bounded_ue`. No unsafe, no external crates, no feature gates (module unconditional — but see F-001 caveat: nothing in a shipped build executes it).

## 9. Open questions for the next agent
- sps.rs owner: add the zero-dimension gate (F-001's root) — it also protects `cropped_size()` and any other consumer of `pic_size_in_ctbs_y()`.
- hevc/fuzz.rs owner (mine, next): does the fuzz corpus ever mint a width-0 SPS? If yes, running the fuzz target in a debug-assertions profile would turn F-001 from read-proven to executed-proven. REQUEST-ORCH-RUN (logged): debug-profile fuzz run of the hevc corpus.
- h264/parser (sibling) — RESOLVED by me during this audit: `rg "ceil_log2|leading_zeros"` over h264/parser/*.rs shows the only `64 - leading_zeros` (sps.rs:740) is inside a TEST bit-builder where payload = v+1 ≥ 1 (no underflow possible), and the golomb reader (slice.rs:132-146) has an explicit `leading_zeros > 31 → GolombOverflow` guard. The H.264 twin does NOT share F-001's pattern — no escalation.
