# AUDIT: crates/codecs/src/video/h264/parser/sps.rs#part1
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-21 |
| Lines / functions | 1–478 of 957 (chunk part1 of 2; part2 ≈479–957 = `read_vui`, `skip_hrd`, test module — W06-A07; `skip_scaling_list` 471-484 straddles, audited here as a coherent unit) / 16 items |
| File verdict | UNREACHABLE (real, spec-conformant parser; no shipped entry point reaches it — see §Reachability) |
| Reachable from shipped entry point? | NO. `SeqParameterSet::parse` callers: `h264/decode.rs:368` (the hw-only `H264Decoder`, itself unreachable — product.toml:169 `hw-decode = false`) and tests. The SHIPPED sw lane (`pipeline_host.rs:347` → `H264SwDecoder`, sw.rs:92-93) imports only `parser::nal` (framing/EPB/avcC) and lets rust_h264 parse SPS itself — `rg SeqParameterSet sw.rs` → zero hits. `rg` over apps/, demux/, media-probe/ → zero hits. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `rg -n "^(fn…)" sps.rs` (map); `rg -n "\.coded_size\(\)|SeqParameterSet::parse"` (caller census); `rg -n "\.$name\(\)"` per derivation method (production-caller census); `sed -n '319,335p' vaapi/session.rs`, `sed -n '139,152p' videotoolbox/session.rs` (twin comparison); `rg -n "parser::sps::tests" test-workspace.log.txt` (8/8 `ok`, 622-629); `sed -n` on every quoted range |

## 1. One-paragraph verdict
Part1 is the SPS syntax walk and its derivation surface: a clean-room §7.3.2.1.1 parse with typed range checks on every bounded field, crop/dimension guards, and pure derivations (chroma format, coded/display size, frame-num/POC moduli, ffprobe-style names). The parse itself is genuinely good — every spec bound I checked is enforced, scaling lists are walked bit-accurately, and the in-repo REAL_SPS fixture test pins exact field values (all 8 sps tests executed and passed in baseline, no corpus needed). But the whole file is production-dead: the shipped software lane never parses SPS in-house (rust_h264 does it), and the only consumer (the hw `H264Decoder`) is policy-off and caller-less. On top of that sit three same-named `coded_size` twins with *divergent* overflow semantics (this one silently clamps, both hw ones refuse), and a `pix_fmt_name` whose "ffprobe spelling" doc is false for >8-bit and full-range-gray cases.

## 2. What actually works (verified)
— All 8 sps tests `ok` in baseline (test-workspace.log.txt:622-629) on the committed in-repo `REAL_SPS` fixture — real executions, corpus-independent.
— Spec bounds verified in `read`: sps_id ≤31 (208), chroma idc ≤3 (218), bit depths ≤6 (222-223), log2 frame-num ≤12 (242), poc type ≤2 (243), poc lsb ≤12 (249), max refs ≤16 (272), mb dims ≤65535 (276-286).
— `ue()` truncation-safety verified: `BitReader::ue` (nal.rs:655-663) caps leading zeros at 31 → values ≤ 2³²−1, so every `r.ue()? as u32` in this file is lossless (but see F-004).
— Crop-unit math (392-401) matches §7.4.2.1.1: ChromaArrayType 0 → (1, 2−frame_mbs_only); else (SubWidthC, SubHeightC·(2−frame_mbs_only)); separate_colour_plane forced to the monochrome units — cross-checked against the passing `crop_arithmetic_matches_spec_crop_units` test (baseline :623).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/video/h264/parser/sps.rs:353-362
Code:
```
    pub fn coded_size(&self) -> Option<(u32, u32)> {
        let w = (u64::from(self.pic_width_in_mbs_minus1) + 1)
            .checked_mul(16)?
            .min(u64::from(u32::MAX)) as u32;
```
What is wrong: The doc says "checked arithmetic", but the `.min(u32::MAX)` converts any overflow into a silent clamp: a hand-built SPS with `pic_width_in_mbs_minus1 = u32::MAX` returns `Some((u32::MAX, _))` — nonsense reported as valid. Meanwhile two same-named, same-signature twins refuse: `hw/vaapi/session.rs:319-325` (`checked_add(1)?.checked_mul(16)?` → `None`) and `hw/videotoolbox/session.rs:139-148` (→ `None`; its own test `coded_size_is_macroblock_math` pins the `None`). Three divergent implementations of one derivation.
Why it matters: Identical input yields `Some(4294967295×…)` here and `None` there — any consumer migrating between them gets silent behavior change; the clamp also contradicts the file's own corruption philosophy (every other degenerate value is a typed `None`, e.g. display_size:405-407). Only hand-built SPS values can trigger it (parse bounds width ≤ 65535 → ≤ 1,048,576 px), which is exactly what the hw sessions' test code does with pub fields.
What correct looks like: One shared `coded_size` (on this type), pure checked arithmetic returning `None` on overflow; the hw twins deleted; the clamp removed.
Confidence: HIGH — Verification: read + arithmetic by hand + twin sources compared.
### F-002 [S3]
Where: crates/codecs/src/video/h264/parser/sps.rs:53-66
Code:
```
    pub fn pix_fmt_name(self, full_range: bool, bit_depth: u8) -> String {
        let depth = if bit_depth > 8 {
            format!("{bit_depth}le")
        } else if full_range {
            "j".to_string()
```
What is wrong: The doc (50-51) claims "ffprobe's `pix_fmt` spelling", but for bit_depth > 8 the output is `yuv10le420p` where ffprobe prints `yuv420p10le` (the depth suffix belongs after the `p`); and full-range monochrome yields `grayj`, which ffprobe never emits (it reports `gray`, or `yuvj420p` for its neutral-chroma rendering view — the test at 651-657 documents that divergence for this very fixture but still calls the function "ffprobe's spelling").
Why it matters: This is the corpus assertions' comparison surface (`h264_parser_corpus.rs`); a 10-bit corpus stream would compare our `yuv10le420p` against ffprobe's `yuv420p10le` and fail spuriously — or worse, someone "fixes" the test to match our wrong string. Zero production callers today (see §7), so the blast radius is evidence tooling.
What correct looks like: `format!("yuv{}p{}{}", …)` with depth/`j` placement per ffprobe, or an honest doc that this is *our* spelling with the known ffprobe deltas listed.
Confidence: HIGH — Verification: read; ffprobe spelling convention (yuv420p10le) is public knowledge; the grayj divergence is admitted in-file at 651-657.
### F-003 [S4]
Where: crates/codecs/src/video/h264/parser/sps.rs:255
Code:
```
                let cycles = ue_limited(r, "num_ref_frames_in_pic_order_cnt_cycle", 256)?;
```
What is wrong: §7.3.2.1.1 bounds `num_ref_frames_in_pic_order_cnt_cycle` to 0..=255; the parser admits 256 — one value beyond spec, accepted as valid.
Why it matters: A technically-invalid SPS parses cleanly; the parse-vs-spec tolerance is off by one in the permissive direction. No memory hazard (256 i32s), no downstream misbehavior — a conformance nit.
What correct looks like: max 255.
Confidence: HIGH — Verification: read vs spec syntax bound.
### F-004 [S4]
Where: crates/codecs/src/video/h264/parser/sps.rs:274-275
Code:
```
        let pic_width_in_mbs_minus1 = r.ue()? as u32;
        let pic_height_in_map_units_minus1 = r.ue()? as u32;
```
What is wrong: The `as u32` is lossless only because `BitReader::ue` (nal.rs:655-663) happens to reject >31 leading zeros (value ≤ 2³²−1). That invariant lives in another file with no cross-reference; loosening nal.rs's cap would make these casts silently truncate *before* the 65535 range check below them.
Why it matters: Hidden cross-file invariant on a hostile-input path; the safety is real today (verified) but undocumented at the consumption site.
What correct looks like: A comment naming the dependency, or an explicit `u32::try_from(v).map_err(...)` here.
Confidence: HIGH — Verification: read of nal.rs:655-667.
### F-005 [S4]
Where: crates/codecs/src/video/h264/parser/sps.rs:460
Code:
```
            other => format!("{other}"),
```
What is wrong: `profile_name` handles 66/77/88/100/110/122/244/44/83/86/118/128, but 138/139 (High 4:2:2 Intra / High 4:4:4 Intra) — which ARE in the chroma-signalling profile list at 216 — fall through to the raw number, while ffprobe names them. Also missing: 118 "Stereo High"/128 "Multiview High" are present, so the omission is inconsistent, not a policy.
Why it matters: The "comparison surface" is again incomplete for exactly the profiles the parser itself special-cases one screen earlier.
What correct looks like: Add 138/139 arms (ffprobe spellings), or document the coverage cut.
Confidence: HIGH — Verification: read; list at 214-217 vs arms at 432-461.

## 4. Per-function table (part1 range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ChromaFormat::from_idc` | 28-36 | REAL | — | Exhaustive 0..=3, `_ => None`. |
| `ChromaFormat::sub_size` | 41-48 | REAL | — | Table 6-1 exact; Monochrome → None. |
| `ChromaFormat::pix_fmt_name` | 53-67 | REAL (code) / FALSE (doc) | F-002 | Executes fine; the ffprobe-spelling claim is wrong for >8-bit and full-range gray. |
| `VuiParameters` (struct) | 74-99 | REAL | — | Honest Option/bool surface; HRD summarized as flag. |
| `ASPECT_TABLE` | 105 | REAL | — | 4 idcs only, documented as corpus-scoped — honest limitation. |
| `SeqParameterSet::parse` | 165-168 | REAL | — | Allocating wrapper; doc points to `parse_with_buffer`. |
| `SeqParameterSet::parse_with_buffer` | 175-187 | REAL | — | Type check (177-181), unescape, `r.finish()` trailing-bit gate — typed errors end to end. |
| `SeqParameterSet::read` | 190-329 | REAL | F-003, F-004 | The syntax walk: every bounded field checked (verified vs spec); High-family branch (214-240) with scaling-list skip count 8/12 correct per ChromaArrayType (228-232, `chroma_or_idc`); poc type-1 cycle values range-checked into i32 (259-265). |
| `chroma_format` | 335-342 | REAL | — | separate_colour_plane → 0; unsignalled → 4:2:0 default (§7.4.2.1.1). Pinned by REAL_SPS test (baseline :628). |
| `height_factor` | 346-348 | REAL | — | `u32::from(!frame_mbs_only)+1` — branchless 1/2. |
| `coded_size` | 353-362 | REAL (bounded) / misleading | F-001 | Clamp-vs-refuse divergence; never fires for parsed SPS (≤65535 mbs). |
| `max_frame_num` | 373-375 | REAL | — | 1<<(minus4+4), exponent parser-bounded to 12; doc states the invariant. Hand-built SPS with minus4>28 would shift-overflow — pub-field caveat, no production constructor does it. |
| `max_pic_order_cnt_lsb` | 382-384 | REAL | — | unwrap_or(0) → min modulus 16 for non-type-0, documented. |
| `display_size` | 390-409 | REAL | — | Crop units exact vs §7.4.2.1.1; crop > coded → typed None (405-407); pinned by tests (baseline :623, and the bad-crop test at 836-865). |
| `aspect_ratio` | 413-423 | REAL | — | VUI SAR > idc table > None; test-only consumer. |
| `profile_name` | 429-462 | REAL | F-005 | cs1/cs3 disambiguation (0x40/0x10 bit picks) correct; 138/139 gap. |
| `chroma_or_idc` | 466-468 | REAL | — | Trivial helper, correct. |
| `skip_scaling_list` (straddle) | 471-484 | REAL | — | §7.3.2.1.1.1 walk: `rem_euclid(256)`, last_scale update only on non-zero next — matches spec scaling_list() semantics; i64 headroom for se deltas. |

## 5. Test honesty
Tests live in part2 (W06-A07); from part1's perspective: the REAL_SPS/REAL_PPS consts are committed in-repo fixtures (598-603), so the 8 sps tests are corpus-independent and genuinely executed (baseline :622-629). `real_corpus_sps_parses_field_exact` pins 20+ fields against the recorded ffmpeg trace_headers oracle — content-strong.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Scaling lists and HRD structures are *skipped correctly* (bit-accurate walk) but not retained" | sps.rs:6-8 | REAL | skip_scaling_list (471-484) implements the spec walk incl. the next_scale≠0 gate; skip_hrd (part2) bounds cpb_count ≤31. |
| "checked arithmetic" (coded_size) | sps.rs:351 | FALSE (partially) | `.min(u32::MAX)` clamps instead of refusing (F-001). |
| "ffprobe's `pix_fmt` spelling" | sps.rs:50-51 | FALSE (for >8-bit, gray full-range) | F-002. |
| "7.B1 wiring: the VA-API DPB/POC reconstruction consumes this" (max_frame_num) | sps.rs:369-371 | REAL | session.rs:505 — but the consumer itself is UNREACHABLE (see my vaapi/session.rs#part4 audit). |

## 7. Dead / duplicated / unreachable code
- **Whole file UNREACHABLE in shipped builds**: shipped sw lane uses only `parser::nal`; sole production consumer is the policy-off hw `H264Decoder` (decode.rs:368). Compiles unconditionally (in-house module, no feature gate).
- `profile_name`, `aspect_ratio`, `pix_fmt_name`: zero production callers (rg census — tests only; `pix_fmt_name` not even corpus tests reach, since those skip-pass).
- Triplicated `coded_size` (F-001) across sps.rs / vaapi / videotoolbox.

## 8. Dependencies & flags
Depends only on `super::nal` (BitReader/H264Error/NalHeader/unescape_rbsp_into). No unsafe. No feature gate (in-house parser, unconditional per codecs/Cargo.toml). No unsafe blocks in range.

## 9. Open questions for the next agent
- W06-A07 (part2): does the test module cover `read_vui`'s Extended_SAR rejection (w==0/h==0 → sar None, 497-499) and `max_dec_frame_buffering > 16 → None` (552-554)? If not, two silent-drop paths are unpinned.
- Wave-owner question: should the in-house SPS/PPS/slice parser be deleted or wired into the sw lane (e.g. for probe facts), given rust_h264 does its own parsing? Today it is maintained dead weight (~2000 LOC across parser/).
