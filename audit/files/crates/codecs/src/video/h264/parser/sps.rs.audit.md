# AUDIT: crates/codecs/src/video/h264/parser/sps.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 956 (594 source + 362 tests) / 18 non-test fns + 9 tests |
| File verdict | REAL (genuine, thorough, spec-grade parser; baseline-tested) — but see F-005: no shipped entry point currently reaches it |
| Reachable from shipped entry point? | NO for the shipped software-only lane (chain below); YES from test binaries |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `rg`/`sed -n`+`od -c` quote verification (5 quotes, byte-exact); `rg SeqParameterSet` caller map; manual bit-decode of REAL_SPS fixture (this audit, all 20 asserted fields reproduced); baseline citation audit/runs/baseline/cargo-test-workspace.txt:719-726 |

## 1. One-paragraph verdict
This is a real, unusually careful H.264 SPS parser: correct profile-gated chroma branch (the 13-profile set matches §7.3.2.1.1 exactly), typed range checks on every ue field (sps_id ≤ 31, idc ≤ 3, depths ≤ 6, log2 frame-num ≤ 12, poc type ≤ 2, refs ≤ 16), spec-exact crop-unit arithmetic including monochrome/interlace and separate-colour-plane cases, checked arithmetic in every derivation, and a hostile test suite (truncation sweep, emulation damage, crafted ue/se overflows) that passed in the orchestrator baseline (cargo-test-workspace.txt:719-726, 8/8 ok). I hand-decoded the embedded real SPS bit-by-bit and every asserted field value is bit-consistent. Two genuine defects: the scaling-list skip uses ChromaArrayType instead of chroma_format_idc (reads 8 lists where the spec reads 12 when separate_colour_plane_flag=1 — contradicts the module header's "bit-accurate walk" claim), and pix_fmt_name emits non-ffprobe spellings for >8-bit streams ("yuv10le420p" vs ffprobe's "yuv420p10le") despite documenting itself as "ffprobe's pix_fmt spelling". Nothing here is fake, but the file's only production consumer (the hw `H264Decoder`) is itself test-only today.

## 2. What actually works (verified)
— Manual bit-decode of `REAL_SPS` (bytes 67 64 00 0d f3 65 11 ec 05 b2 ...) performed in this audit: reproduced profile 100, level 13, sps_id 0, chroma_format_idc 0 (monochrome), log2_max_frame_num 0, poc type 0, poc_lsb 2, max_num_ref_frames 4, 4x3 MBs → 64x48, frame_mbs_only, full_range, video_format 5, timing (1,20,false), max_dec_frame_buffering 4 — all match test assertions at sps.rs:614-653, so the parse walk is correct on a real x264 SPS.
— `rg -n "h264::parser::sps" audit/runs/baseline/cargo-test-workspace.txt` -> 8/8 sps tests ok (lines 719-726) + corpus `corpus_raw_annexb_0019_field_assertions ... ok` (line 1156).
— `rg -rln "SeqParameterSet"` -> consumers mapped: decode.rs:368 (store_sps), hw/vaapi/session.rs (max_frame_num/coded_size/va_profile_for), tests/h264_parser_corpus.rs, fuzz.rs.

## 3. Findings
F-001 [S3]
Where: sps.rs:225-236 (+ sps.rs:465-468)
Code: (verbatim)
```
                // seq_scaling_matrix_present: walk (not retain) the
                // scaling lists — 4:4:4 signals 12, everything else 8.
                let count = if chroma_or_idc(idc, separate_colour_plane) == 3 {
                    12
                } else {
                    8
                };
```
and
```
fn chroma_or_idc(idc: u8, separate: bool) -> u8 {
    if separate { 0 } else { idc }
}
```
What is wrong: The §7.3.2.1.1.1 loop condition is `i < 8 || (i < 12 && chroma_format_idc == 3)` — it keys on the syntax element `chroma_format_idc`, not on ChromaArrayType. When `separate_colour_plane_flag == 1` (which requires chroma_format_idc == 3), the spec still reads 12 `seq_scaling_list_present_flag`s; this code computes ChromaArrayType (0) and reads 8. Why it matters: a valid separate-colour-plane SPS with a scaling matrix desyncs the bit reader → every subsequent field is garbage → spurious typed error (or silently wrong fields if the desync happens to re-sync). The module header (lines 6-7) claims "Scaling lists and HRD structures are *skipped correctly* (bit-accurate walk)" — false for this input class. No test constructs separate_colour_plane=1 with a scaling matrix, so nothing would catch it. What correct looks like: `let count = if idc == 3 { 12 } else { 8 };` (the separate_colour_plane override is correct in `display_size` crop units, where the spec DOES use ChromaArrayType — it is wrong here).
Confidence: HIGH — Verification: read (spec text from ITU-T H.264 §7.3.2.1.1.1; no in-repo spec copy — UNVERIFIED against a local copy; no execution possible without compiling).

F-002 [S3]
Where: sps.rs:50-66
Code: (verbatim)
```
    /// ffprobe's `pix_fmt` spelling for 8-bit streams (the corpus
    /// assertion's comparison surface).
    #[must_use]
    pub fn pix_fmt_name(self, full_range: bool, bit_depth: u8) -> String {
        let depth = if bit_depth > 8 {
            format!("{bit_depth}le")
```
(together with `ChromaFormat::Yuv420 => format!("yuv{depth}420p")` at line 63)
What is wrong: For bit_depth=10 the function produces `yuv10le420p`; ffprobe's spelling is `yuv420p10le` (suffix after the sampling, before the endianness — `yuv422p10le`, `yuv444p12le`, etc.). The doc comment markets it as "ffprobe's `pix_fmt` spelling ... (the corpus assertion's comparison surface)" and tests/h264_parser_corpus.rs:203-206 does compare this string against live ffprobe `pix_fmt`. It passes only because the H.264 corpus has no >8-bit lane today. Additionally `grayj` (monochrome + full range, 8-bit) is not a real ffmpeg pix_fmt name (ffmpeg has gray/gray10le...; ffprobe prints `gray`), though the corpus test bypasses monochrome through its own `ffprobe_visible_pix_fmt` mapping so the lie is never compared. The HEVC copy (hevc/parser/sps.rs:66-81) has the identical bug. What correct looks like: `format!("yuv420p{depth}")` etc., and drop or fix the grayj claim.
Confidence: HIGH — Verification: read (ffmpeg naming convention; no ffmpeg in sandbox to execute — UNVERIFIED by execution).

F-003 [S4]
Where: sps.rs:255
Code: (verbatim)
```
                let cycles = ue_limited(r, "num_ref_frames_in_pic_order_cnt_cycle", 256)?;
```
What is wrong: Spec range is 0..=255; the code accepts 256 (inclusive bound off by one), contradicting the header's "out-of-range ue/se fields" typed-refusal claim. Blast radius: one extra legal-but-out-of-spec value retained as a 256-entry offset list; no panic, no desync.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: sps.rs:185 (`r.finish()?`) — defect lives in nal.rs:708-735
Code: (verbatim, sps.rs:183-186)
```
        let mut r = BitReader::new(rbsp);
        let sps = Self::read(&mut r)?;
        r.finish()?;
```
What is wrong: `BitReader::finish` scans forward through any number of zero bits to find the next `1` and only then enforces "zeros after stop". §7.3.2.11 requires the stop bit to be the *next* bit; leading zeros before it mean `more_rbsp_data()` was true, i.e. the SPS carried unread data. The parser accepts such padded/crafted SPSs. Blast radius: laxness only (accepts slightly invalid streams); real encoders emit a proper stop bit.
Confidence: MEDIUM — Verification: read (nal.rs code read; behavior not executed).
F-005 [S4] (reachability — cross-file)
Where: apps/cli/src/pipeline_host.rs:342 (comment) vs rg evidence
What is wrong: The only shipped h264 decode lane is `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` (pipeline_host.rs:354), and sw.rs uses the *dependency's* `rust_h264::nal::parse_sps_info` (sw.rs:90,292) — not this file. This file's production consumer `H264Decoder` (decode.rs:368 store_sps) is constructed ONLY in crates/codecs/tests/h264_hw_path.rs; the promised `H264Decoder::from_setting` / `playback.hw-decode-policy` wiring (pipeline_host.rs:342 comment, decode.rs doc, product-config key) has no call site outside tests — a config key validated and merged but never read by shipped hosts. sps.rs is therefore test-reachable, production-unreachable today.
Confidence: HIGH — Verification: `rg -rn "from_setting|installed_bridges|negotiate_lane"` whole-repo -> only h264 module + tests.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ChromaFormat::from_idc | 28-36 | REAL | — | 0..=3 exhaustive, else None |
| ChromaFormat::sub_size | 41-48 | REAL | — | Table 6-1 exact (mono→None) |
| ChromaFormat::pix_fmt_name | 53-67 | SHALLOW | S3 | F-002: >8-bit spellings are not ffprobe's |
| SeqParameterSet::parse | 165-168 | REAL | — | allocating wrapper |
| SeqParameterSet::parse_with_buffer | 175-187 | REAL | — | wrong-NAL typed; unescape clears buffer (nal.rs:328 verified) |
| SeqParameterSet::read | 190-329 | REAL | S3/S4 | the walk; F-001 (scaling count), F-003 (cycle bound) |
| chroma_format | 335-342 | REAL | — | separate→0, unsignalled→4:2:0 default ✓ |
| height_factor | 346-348 | REAL | — | 1 + !frame_mbs_only ✓ |
| coded_size | 353-362 | REAL | — | checked u64 mul; ≤65536 MBs pre-bounded |
| max_frame_num | 373-375 | REAL | — | 1<<(x+4), x≤12 ✓; consumed by vaapi session (verified) |
| max_pic_order_cnt_lsb | 382-384 | REAL | — | None→16 documented fallback |
| display_size | 390-409 | REAL | — | crop units spec-exact incl. mono/separate/interlace; overflow→None |
| aspect_ratio | 413-423 | REAL | — | table lookup else None (never guesses) |
| profile_name | 429-462 | REAL | — | ffprobe spellings incl. cs1/cs3 variants; 134/135/138/139 fall to raw number |
| chroma_or_idc | 466-468 | REAL | S3 | correct for crop units, WRONG for scaling count (F-001) |
| skip_scaling_list | 471-484 | REAL | — | rem_euclid(256) matches spec (last+delta+256)%256; bit-accurate |
| read_vui | 488-567 | REAL | — | aspect/overscan/video/colour/loc/timing/HRD/bitstream-restrictions walk ✓ |
| skip_hrd | 571-592 | REAL | — | cpb_cnt_minus1 ≤ 31 ✓ spec |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| real_corpus_sps_parses_field_exact | YES | 20+ field-exact assertions on a real x264 SPS; my independent bit-decode reproduces them |
| real_corpus_pps_pairs_with_the_sps | YES | 15 PPS field assertions vs trace_headers-verified values |
| truncation_forbidden_and_wrong_type_are_typed | YES | every 1..len prefix must error; forbidden bit; wrong NAL; 1-byte NAL |
| invalid_sps_id_is_out_of_range | YES | crafted ue(32) must yield exact OutOfRange{32,31} |
| emulation_damage_in_sps_is_typed | YES | injected 00 00 03 99 must yield Emulation |
| crop_arithmetic_matches_spec_crop_units | YES | 4:4:4 coded 16x16 → 13x9; crop>size → None |
| poc_type_1_cycle_offsets_parse_and_bound | YES | offsets [1,-1,5] retained; 33-bit se → typed ue.overflow |
| avc_length_prefixed_entry_form_parses_the_same_sps | YES | AVC-framed == Annex-B parse equality |
| GAP: separate_colour_plane=1 + scaling matrix | — | no test exists → F-001 undetectable by the suite |
| GAP: 4:2:2 crop units / Yuv422 pix_fmt | — | sub_size(2,1) correct by inspection, never asserted |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Scaling lists and HRD structures are *skipped correctly* (bit-accurate walk) but not retained — nothing here decodes" | sps.rs:6-8 | SHALLOW | HRD skip verified correct; scaling skip wrong for separate_colour_plane (F-001); "nothing here decodes" is true |
| "out-of-range ue/se fields" typed errors (doc) | sps.rs:163-164 | SHALLOW | 256-vs-255 off-by-one accepted (F-003) |
| "7.B1 wiring: the VA-API DPB/POC reconstruction (hw/vaapi/session.rs) consumes this" | sps.rs:369-371 | REAL | session.rs uses `sps.max_frame_num()` (rg-verified); but that session is itself hw-path/test-only today (F-005) |
| "ffprobe's `pix_fmt` spelling" for pix_fmt_name | sps.rs:50-51 | FALSE for >8-bit | F-002 |

## 7. Dead / duplicated / unreachable code
None dead. Unreachable-in-production: the whole file's production consumer chain (H264Decoder → store_sps → parse) is test-only today (F-005). Duplication: ChromaFormat::pix_fmt_name is copy-pasted between h264 and hevc parsers with the same F-002 bug.

## 8. Dependencies & flags
Depends only on `super::nal` (BitReader/H264Error/NalHeader/unescape_rbsp_into) — in-crate. No feature gates on this file; the `codec-h264-sw` feature gates sw.rs (default ON, codecs/Cargo.toml:53), not this file.

## 9. Open questions for the next agent
1. decode.rs/pipeline_host.rs auditor: confirm whether the `playback.hw-decode-policy` → `H264Decoder::from_setting` wiring is a documented Phase-7 TODO or a dropped promise (comment at pipeline_host.rs:341-353 claims an "opt-in path" that has no call site).
2. hevc/parser/sps.rs auditor: same F-002 pix_fmt_name bug exists there (line 66-81).
3. Anyone with ffmpeg on PATH: run `ffprobe -show_streams` on a 10-bit H.264 file and compare `yuv420p10le` against `pix_fmt_name` output to convert F-002 from read-verified to executed.
