# AUDIT: crates/codecs/src/video/h264/parser/pps.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 374 / 5 prod fns + 3 tests |
| File verdict | REAL for Main/Baseline-shaped PPS; rejects the High-profile norm (transform_8x8 extension) with a misleading damage code |
| Reachable from shipped entry point? | ONLY via the opt-in hw lane: settings `playback.hw-decode-policy` → `H264Decoder::from_setting` (apps/cli/src/pipeline_host.rs:342 comment) → h264/decode.rs:382 `store_pps` → `PicParameterSet::parse`. The default SW lane (rust_h264) uses only the NAL framing (sw.rs:92 imports), not this parser |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 h264::parser::pps` → 3 passed; 0 failed, exit 0; rg/sed (callers, finish(), settings) |

## 1. One-paragraph verdict
A clean, typed, bounds-checked parse of the PPS fields its card names — I
ran its 3 tests live (crafted full-field PPS, four typed error paths, FMO
refusal) and they pass. But the field list ends at
`redundant_pic_cnt_present_flag`: the parser never implements the
`more_rbsp_data()` extension grammar of §7.3.2.2 (`transform_8x8_mode_flag`,
`pic_scaling_matrix_present_flag`, `second_chroma_qp_index_offset`). A
High-profile PPS carrying that extension — x264's default shape for High,
i.e. the most common real-world H.264 PPS — falls into `rbsp_trailing_bits`
validation and is rejected as "damage" with the misleading
`h264.rbsp.tail` code instead of the typed `Unsupported` the module's own
FMO-honesty paragraph promises. The shipped default (software) lane never
runs this parser; the opt-in hw lane does.

## 2. What actually works (verified)
— verified by `./target/debug/deps/codecs-bae0f1756cc0a809 h264::parser::pps`
→ `test result: ok. 3 passed; 0 failed` (exit 0): full field walk on a
handcrafted CABAC PPS (QP offsets, bipred idc 2, ref-index defaults),
wrong-NAL/forbidden/truncated/OutOfRange typed paths, FMO
`h264.pps.fmo_slice_groups` refusal, and the SPS-pairing helper.

## 3. Findings

F-001 [S2]
Where: crates/codecs/src/video/h264/parser/pps.rs:88-129 (read) + :84
`r.finish()?`
Code:
```rust
        let deblocking_filter_control_present = r.bit()? == 1;
        let constrained_intra_pred = r.bit()? == 1;
        let redundant_pic_cnt_present = r.bit()? == 1;
        Ok(PicParameterSet {
```
What is wrong: after `redundant_pic_cnt_present_flag` the parser stops
reading and `parse_with_buffer` calls `r.finish()` (:84). Per §7.3.2.2, a
PPS with `more_rbsp_data()` continues with `transform_8x8_mode_flag u(1)`,
`pic_scaling_matrix_present_flag u(1)`, optional scaling lists, and
`second_chroma_qp_index_offset se(v)` before the real stop bit. For any
such PPS, `finish()` scans forward, finds the first `1` bit — the
`transform_8x8_mode_flag` — treats it as the stop bit, and then sees
nonzero bits after it (the se code / real stop bit) →
`Err(Invalid { "h264.rbsp.tail" })`. Net effect: every High-profile PPS
with `transform_8x8_mode_flag = 1` (x264's High default) is rejected as
structural damage with a code that misattributes the cause. On the opt-in
hw lane (h264/decode.rs:382) that PPS failure fails configure/decode for
exactly the streams hardware decode is most used for.
Why it matters: the module's FMO-honesty doctrine (:14-16 "The refusal is
honest and observable, never a silent skip") is violated by a
lie-by-error-code; realistic streams fail with a corruption diagnosis.
What correct looks like: check `more_rbsp_data()` after
`redundant_pic_cnt_present_flag`; either parse the extension fields or
refuse with `Unsupported { "h264.pps.extension" }`.
Confidence: HIGH — Verification: read (bit-level trace through
nal.rs:708-730 finish()); UNVERIFIED by execution (no High-extension PPS
fixture in tree; the "High-profile" test at :176-180 admits it is
"Main-sized").
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/codecs/src/video/h264/parser/pps.rs:155-170
Code:
```rust
    if v < min || v > max {
        return Err(H264Error::OutOfRange {
            field,
            value: v.unsigned_abs(),
            max: max.unsigned_abs(),
        });
    }
```
What is wrong: sign information is discarded in the error: a
`pic_init_qp_minus26` of −40 vs bound −26 reports "value 40 out of range
(max 26)". Diagnosable but lossy — the OutOfRange variant carries u64s, so
the negative bound is reported by magnitude only.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/codecs/src/video/h264/parser/pps.rs:35 and :118; tests :191-199, :273-281, :341-349
Code:
```rust
    pub num_slice_groups_minus1: u32,
...
            num_slice_groups_minus1: 0,
```
What is wrong: (a) the public field is a constant — every value >0 is
refused at :94, so the constructed value is hard-coded 0; a field that can
never vary is API noise suggesting FMO values are representable. (b) the
`push_ue` closure is copy-pasted verbatim three times across the three
tests.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PicParameterSet::parse / parse_with_buffer | 65-86 | REAL | - | NAL-type gate, unescape, finish() |
| PicParameterSet::read | 88-130 | REAL (Main-shaped only) | S2 | F-001: no more_rbsp_data extension branch |
| PicParameterSet::references | 135-137 | REAL | - | trivial contains |
| bounded_ue | 142-152 | REAL | - | typed bound |
| bounded_se | 155-170 | REAL | S4 | F-002 sign loss |
| 3 tests | 181-373 | REAL | - | ran live; no High-extension fixture (F-001 untested) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| crafted_pps_covers_the_full_field_list | YES | every field asserted, including negative QP offsets |
| error_paths_are_typed | YES | exact error kinds/values |
| fmo_slice_groups_are_unsupported_not_silent | YES | exact Unsupported code |
| (missing) High-profile extension PPS | - | the exact realistic shape F-001 rejects has no test |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "clean-room parse of the card's field list" | pps.rs:1-6 | REAL (that list) / INCOMPLETE | the list omits the §7.3.2.2 extension; F-001 |
| "The refusal is honest and observable, never a silent skip" (FMO) | pps.rs:14-16 | REAL for FMO / FALSE as a module invariant | High-extension PPS refused via misleading `h264.rbsp.tail` |
| "no real-world encoder in our corpus ... emits FMO" | pps.rs:12-14 | UNVERIFIED | corpus absent in sandbox; plausible |
## 7. Dead / duplicated / unreachable code
None in-file. (Context: h264 nal.rs `BitReader::more_rbsp_data` — the
naive any-1-bit variant the hevc sibling explicitly fixes — is itself
uncalled in-tree; relevant to F-001's missing branch.)
## 8. Dependencies & flags
Sibling nal.rs only. Unconditional compile (in-house module, video/mod.rs:29).
## 9. Open questions for the next agent
- hw/feed.rs (next file): does the hw feed validate PPS capabilities the
  same way — i.e., would a High-extension PPS fail before or after
  expensive adapter work?
- Craft a transform_8x8 PPS fixture and confirm the `h264.rbsp.tail`
  rejection end-to-end (blocked here: no corpus, no build budget).
