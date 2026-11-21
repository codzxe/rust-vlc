# AUDIT: crates/codecs/src/video/h264/parser/sps.rs#part2 (lines 479–956)
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-11-21 |
| Lines / functions | 478 / 14 (2 production fns: `read_vui`, `skip_hrd`; 8 tests; `SpsBits` builder with 6 methods) — chunk of a 956-line file, equal 2-way split: part1=1–478 (ChromaFormat, SPS struct, parse/read, accessors, scaling-list skip), part2=479–956 |
| File verdict | UNREACHABLE (real, spec-correct, test- and fuzz-executed code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `SeqParameterSet::parse` is called from `h264/decode.rs:368` (H264Decoder hw lane — never constructed in any shipped host) and `h264/fuzz.rs:56` (fuzz harness). The shipped H.264 lane (`sw.rs`) uses `rust_h264`'s own `parse_sps_info` (sw.rs:90), not this parser (sw.rs:92-94 imports only `parser::nal`). VA-API/VideoToolbox sessions import it but are Phase-7-unwired. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n` structure map; `sed -n '440,956p'` full read of range; `rg "SeqParameterSet::|parser::sps"` across crates+apps (callers listed above); `rg "parser::sps::tests::"` audit/runs/baseline/test-workspace.log.txt (all 8 tests ran ok — lines 622-629) |

## 1. One-paragraph verdict
The VUI/HRD walkers in this chunk are bit-exact against H.264 Annex E (I verified every flag order, field width, and the two-HRD/low_delay structure of §E.1.2/E.2.2), and the test half is the best test suite I have seen in this codebase: a real x264 SPS embedded as inline bytes (no external corpus dependency), asserted field-by-field against ffmpeg's `trace_headers`, plus a full prefix-truncation sweep, crafted oversize-crop and Exp-Golomb-overflow lanes — all executed in the baseline run (log lines 622–629). The single structural defect is the one that shadows the whole in-house H.264 parser: the shipped software decoder never calls any of it, so this is another maintained-but-unwired island feeding only the dormant hw lanes and the fuzz target. One S4: an out-of-spec `max_dec_frame_buffering` (>16, impossible per Annex A.3.1's MaxDpbFrames cap) is silently dropped to `None` where the parser's own philosophy demands a typed OutOfRange.

## 2. What actually works (verified)
— `read_vui` (488–570): every syntax element in §7.3.2.1.1 Annex E order — aspect (8-bit idc; Extended_SAR 16+16 with 0:0→unspecified), overscan, video_signal_type (3+1+flag+3×8), chroma_sample_loc (2×ue), timing (2×32+1, zero-guarded), NAL+VCL HRD with `low_delay` only when either present, pic_struct, bitstream_restriction (7 elements in exact order). — verified by read + spec comparison.
— `skip_hrd` (571–593): cpb_cnt_minus1 typed-checked against the spec's 0..=31, scales 4+4 bits, per-CPB (ue, ue, bit) × (cpb_count+1), four 5-bit length fields — exact §E.1.2.
— All 8 tests ran and passed in baseline: `test video::h264::parser::sps::tests::real_corpus_sps_parses_field_exact ... ok` etc. (audit/runs/baseline/test-workspace.log.txt:622-629) — inline fixtures, so they cannot silently skip in other environments (contrast slice.rs's corpus lane).
— Fuzz coverage of the parse entry exists (h264/fuzz.rs:56 calls `SeqParameterSet::parse_with_buffer` per NAL), unlike slice-header parsing.

## 3. Findings
### F-001 [S3] Unreachable from every shipped entry point — the shipped H.264 decoder parses SPS with rust_h264, not with this file
Where: crates/codecs/src/video/h264/parser/sps.rs:479-956 (the audited surface) and crates/codecs/src/video/h264/sw.rs:88-94 (what actually ships)
Code (sw.rs:88-94 — the shipped decoder's parser imports):
```rust
use rust_h264::decoder::OrderedDecoder;
use rust_h264::error::DecodeError;
use rust_h264::nal::{NalUnit, NalUnitType as EngineNalType, parse_sps_info};

use super::parser::nal::{
    AnnexBNals, AvcConfig, AvcNals, H264Error, NalHeader, NalUnitType, unescape_rbsp_into,
};
```
What is wrong: The reachable H.264 decode path (pipeline_host.rs:346 → `H264SwDecoder`) performs SPS interpretation via the external engine's `parse_sps_info`; the 956-line in-house SPS parser (and its VUI/HRD walkers) is consumed only by `decode.rs:368` (hw `H264Decoder`, unconstructed in shipped hosts), the VA-API/VideoToolbox sessions (Phase 7, unwired), and the fuzz harness. Every shipped binary compiles two complete SPS parsers and uses one of them.
Why it matters: Two parsers for the same syntax is drift made structural — e.g. the in-house parser's colorimetry/crop semantics (this file) can diverge from `rust_h264`'s interpretation (what users actually see), and no test compares them.
What correct looks like: Either route the sw path's extradata/dimension/colorimetry facts through this parser (it is stricter and better tested than what the engine exposes), or gate the in-house SPS/PPS/slice tree behind the future hw feature until Phase 7.
Confidence: HIGH — Verification: read + exhaustive `rg` (callers listed in header); baseline corrected smoke (sample-420.mp4 plays via sw lane) proves which parser serves users.
### F-002 [S4] Out-of-spec `max_dec_frame_buffering` silently dropped instead of typed-refused
Where: crates/codecs/src/video/h264/parser/sps.rs:550-556
Code:
```rust
        let _max_reorder = r.ue()?;
        let buffering = r.ue()?;
        if buffering <= 16 {
            max_dec_frame_buffering = Some(buffering as u32);
        }
```
What is wrong: Annex A.3.1 caps `MaxDpbFrames` at 16 (`Min(MaxDpbMbs/(PicSizeInMbs), 16)`), so a parsed `max_dec_frame_buffering` of 17+ is spec-impossible — corruption per this parser's own doctrine (nal.rs's sibling error doc: "a syntax value outside its spec-declared range … violations are corruption, not features"). Instead of `H264Error::OutOfRange`, the value is silently discarded and the field reads `None`, indistinguishable from "not signalled". The struct doc's "with a sane value" (96-97) documents the filtering but not the corruption reading.
Why it matters: A hostile/corrupt SPS gets a "clean" parse with an erased field — the exact silent-smoothing this parser family advertises it never does.
What correct looks like: `if buffering > 16 { return Err(H264Error::OutOfRange { field: "max_dec_frame_buffering", value: buffering, max: 16 }); }` — matching `skip_hrd`'s treatment of `cpb_cnt_minus1` two functions down.
Confidence: HIGH — Verification: read; spec cap recalled from Annex A.3.1 (no spec copy in sandbox — cited from parser's own § references).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `read_vui` | 488–570 | REAL (unreachable) | — | §E.2.1/E.2.2 order-exact (verified element by element); `ASPECT_TABLE` covers idc 1–4 with the gap honestly documented (Table E-1 entries 5..=16 → `sar=None`, `aspect_ratio_idc` retained — "never a guess", const doc 100-105); timing zero-guarded |
| `skip_hrd` | 571–593 | REAL (unreachable) | — | §E.1.2 exact; cpb_cnt_minus1 typed 0..=31; bounded loop (≤32) |
| `SpsBits::{new,byte,flag,ue,se,nal}` (test builder) | 713–773 | REAL | — | correct §9.1/§9.2 packing (se: v>0 → 2v−1, else −2v); `nal()` adds the rbsp stop bit |
| `real_corpus_sps_parses_field_exact` | 610–663 | REAL | — | 30+ assertions vs `trace_headers` oracle; inline REAL_SPS bytes — environment-independent |
| `real_corpus_pps_pairs_with_the_sps` | 665–687 | REAL | — | 15 field assertions on the paired PPS (exercises pps.rs, cross-module) |
| `truncation_forbidden_and_wrong_type_are_typed` | 690–710 | REAL | — | every prefix cut (1..len) must fail; forbidden bit; wrong NAL type; 1-byte NAL — the strongest totality test in the file |
| `invalid_sps_id_is_out_of_range` | 776–789 | REAL | — | crafted ue(32) → typed OutOfRange |
| `emulation_damage_in_sps_is_typed` | 791–799 | REAL | — | injected `00 00 03 99` → typed Emulation |
| `crop_arithmetic_matches_spec_crop_units` | 802–866 | REAL | — | 4:4:4 CropUnit 1×1: coded 16×16 → display 13×9; oversize crop → `display_size()==None` |
| `poc_type_1_cycle_offsets_parse_and_bound` | 869–932 | REAL | — | poc type 1 cycle list retained; 33-bit se → typed `h264.ue.overflow` (the reader's width limit enforced structurally) |
| `avc_length_prefixed_entry_form_parses_the_same_sps` | 935–956 | REAL | — | differential: Annex-B vs length-prefixed entry forms must yield identical SPS |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 8 tests | YES | every test asserts concrete parsed values (fields, sizes, typed error variants) against either an ffmpeg oracle (REAL_SPS), arithmetic identities (crop 16−3×16−7), or crafted bitstreams with exactly one hostile element; the truncation sweep covers every prefix; fixtures are inline constants — no environment-conditional skips |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Aspect-ratio idc table (Annex E, Table E-1) for the idcs the corpus carries; unlisted idcs resolve to None … never a guess" | sps.rs:100-105 | REAL (honest partial) | read of ASPECT_TABLE (4 entries) + retention of `aspect_ratio_idc` |
| "`max_dec_frame_buffering` when bitstream restrictions are signalled with a sane value" | sps.rs:95-97 | REAL but masks F-002 | the ≤16 filter silently drops out-of-spec values |
| (matrix row `codec-video-h264-software`) — see wave-6 question below | docs/feature-matrix.toml | REAL for the sw lane; the in-house parser is not what serves it | baseline sample-420.mp4 plays; sw.rs:90 uses rust_h264's `parse_sps_info` |

## 7. Dead / duplicated / unreachable code
- Both production functions are unreachable at runtime (F-001); they are reachable via the fuzz target and unit tests, which is what keeps them honest.
- Duplication: two complete SPS parsers ship in every binary (this file + rust_h264's internal one); three VUI-adjacent colorimetry derivations exist across sps.rs, sw.rs's BT709 fallback, and the hw lane's `HW_COLOR` twin.

## 8. Dependencies & flags
- No feature gates (unconditional in-house parser tree; codecs/Cargo.toml:26-27).
- Wave-6 hostile question: this file is NOT on the shipped H.264 path (F-001) — for the full reachability trace see mjpeg/decoder.rs#part2.audit.md §8 (verdict: H.264 decode IS reachable via sw.rs + default features; this parser is not part of it).

## 9. Open questions for the next agent
- part1 (sps.rs 1–478) auditor: check `display_size()`'s crop-unit math for 4:2:2 (SubWidthC=2, SubHeightC=1 — the asymmetry the crop test does not cover: it tests 4:4:4 only) and `coded_size()` for `frame_mbs_only_flag == 0` (the ×2 frame-height rule).
- sw.rs / decode.rs owners: when the hw lane lands, the PPS-pairing test here proves this parser and the engine agree on this one fixture — extend that differential to the sw lane's actual inputs (avcC blobs from real MP4s) before trusting either.
