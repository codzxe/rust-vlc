# AUDIT: crates/codecs/src/video/mpeg12/headers.rs#part2 (lines 526–1105; function-integrity overlap 526–552)
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2026-11-21 |
| Lines / functions | 580 audited (assigned ≈553–1105 of 1106; start pulled back to 526 to keep `impl PictureCodingExtension` whole — 27-line overlap with W06-A04's part1, not re-audited) / 18 items in table (2 prod functions + 1 prod struct + test Writer + 13 tests + struct docs) |
| File verdict | UNREACHABLE (code is REAL quality — no shipped entry point reaches the mpeg12 module) |
| Reachable from shipped entry point? | NO. Only consumer of this chunk's API is `crates/codecs/src/video/mpeg12/facts.rs` (facts.rs:109 `PictureCodingExtension::parse`, :119 `GopHeader::parse`, :142 `SliceHeader::parse`); `Mpeg12Facts`/`descriptor_gate` are consumed ONLY by `crates/codecs/tests/mpeg12_corpus.rs` (:29,:244,:274). `rg -ln "mpeg12"` over crates/+apps/ outside the module and tests → only `crates/codecs/src/video/mod.rs:39 pub mod mpeg12;` (declaration) and comment mentions in subtitles/cc_data.rs. Demux mints `mpeg1video`/`mpeg2video` descriptors as hard-coded strings (crates/demux/src/ts/mod.rs:351-352, crates/demux/src/ps/mod.rs:771-784) and apps/cli/src/pipeline_host.rs has no arm for them (falls to `cli.codec_unavailable`, :374-380). The module's own header (mpeg12/mod.rs:5-11) admits: "There is **no software MPEG-1/2 decode here** — this module parses and classifies bitstream structure only". |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:4 |
| Commands run | `grep -n "^fn \|^pub fn \|^impl \|^struct…"` over headers.rs (structure map for chunk split); `rg -n "PictureCodingExtension\|SliceHeader::parse\|decode_macroblock_address_increment\|GopHeader"` over crates/+apps/ (facts.rs only); `rg -ln "mpeg12"` over crates/+apps/ (module decl + subtitles comments only → unreachable); `rg -n '"mpeg1video"\|"mpeg2video"'` over demux+cli (descriptors minted by demux, no pipeline_host arm); baseline test log sed for `video::mpeg12::headers::tests` (all 13 ok, test-workspace.log.txt:758-771); Table B-1 / §6.2.4 / extension-id values checked against ISO/IEC 11172-2 / 13818-2 layouts; every quote sed-verified. |

## 1. One-paragraph verdict
This chunk (picture-coding-extension parse, slice header + the Table B-1 macroblock-address VLC, and the 13-test suite) is careful, spec-faithful parse code — bit widths and field order match ISO/IEC 13818-2 §6.2.3/§6.3.5 and all 33 Table B-1 codes match 11172-2 — but NONE of it is reachable from a shipped entry point: the mpeg12 module is a documented parse-only substrate for a "Phase 6+ self-build" and a "Phase-7 hw-feed seam" that do not exist in the tree, its only non-test consumer is the same-module facts scanner, and the facts scanner's only consumer is the test suite. The defects are all zero-blast-radius polish: `quantiser_scale_code` documents "0 forbidden" then accepts 0 unvalidated, `picture_structure` reserved 0 and forbidden `f_code` 0/15 pass silently (contradicting the module's "no damage smoothing" posture), a mid-VLC truncation is folded into `None` instead of a typed `Truncated`, and the table-coverage test duplicates the production table (a shared transcription error passes; prefix-freeness is claimed but never verified). The module is honest about being a non-decoder; the "software decode blocked" feature-matrix posture matches the code.

## 2. What actually works (verified)
- All 13 headers tests pass in baseline — verified by `audit/runs/baseline/test-workspace.log.txt:758-771` (`video::mpeg12::headers::tests::* … ok`, 13 entries) inside the codecs 506-passed unit suite.
- `PictureCodingExtension::parse` field order/widths correct per 13818-2 §6.3.5 (f_code 4×4b, intra_dc_precision 2b, picture_structure 2b, then the ten 1-bit flags in spec order; extension id `1000` correct) — verified by read against the spec layout; `picture_coding_extension_roundtrip` exercises every field.
- Table B-1 macroblock_address_increment codes 1..=33 — all 33 (code,length) pairs match ISO/IEC 11172-2 Table B-1, hand-checked; `mb_increment_table_covers_one_through_thirty_three` additionally checks the mapping is a bijection onto 1..=33 and unknown suffixes (escape `0b00000001000`, all-zeros) return `None`.
- `SliceHeader::parse` extra_bit_slice/intra_slice/reserved-8-bit structure matches §6.2.4; the greedy prefix-walk VLC matcher (668-678) is sound because the table is prefix-free.
- Truncation typing: all reads go through `BitReader` `read_bits?` — no panic path in the chunk (zero unwrap/expect/index in 526-686).

## 3. Findings

### F-001 [S4]
Where: crates/codecs/src/video/mpeg12/headers.rs:583-584 and 652
Code:
```
    /// `quantiser_scale_code` (5 bits; 1..=31, 0 forbidden).
    pub quantiser_scale_code: u32,
```
(652): `let quantiser_scale_code = r.read_bits(5)?;`
What is wrong: the field doc asserts 0 is forbidden, but `SliceHeader::parse` reads 5 bits and never range-checks — a slice with `quantiser_scale_code == 0` parses to `Ok`.
Why it matters: the module's stated posture (mod.rs:41-43 "typed `Result`, no panics, no damage smoothing") and this field's own doc are contradicted by the code; a downstream consumer (the advertised Phase-7 hw feed) would receive a value the spec forbids. Blast radius today: zero (module unreachable).
What correct looks like: `if quantiser_scale_code == 0 { return Err(Mpeg12Error::OutOfRange { … }) }` — matching the `aspect_ratio_information`/`frame_rate_code` gate precedent (791-811, part1's checks).
Confidence: HIGH — Verification: read (all uses of quantiser_scale_code grep-verified: 584, 652, 681 — no check anywhere).

### F-002 [S4]
Where: crates/codecs/src/video/mpeg12/headers.rs:539-546
Code:
```
        let mut f_code = [[0u32; 2]; 2];
        for row in &mut f_code {
            for slot in row {
                *slot = r.read_bits(4)?;
            }
        }
        let intra_dc_precision = r.read_bits(2)?;
        let picture_structure = r.read_bits(2)?;
```
What is wrong: no validation of values the spec reserves/forbids: `picture_structure == 0` (reserved — only 1 top, 2 bottom, 3 frame defined) and `f_code` 0 or 15 (both forbidden per 13818-2 §6.3.5) parse to `Ok`.
Why it matters: same posture contradiction as F-001; these fields feed the advertised hw seam verbatim, and a hardware decoder handed `f_code=0` is in the spec's undefined territory.
What correct looks like: range-gate `picture_structure` to 1..=3 and reject `f_code` 0/15 with the module's typed `OutOfRange`, consistent with part1's chroma_format gate (`mpeg12.ext.chroma`).
Confidence: HIGH — Verification: read (no validation between the reads and the `Ok` at 557).

### F-003 [S4]
Where: crates/codecs/src/video/mpeg12/headers.rs:670-678
Code:
```
        for length in 1..=11u32 {
            let bit = r.read_bit().ok();
            let Some(bit) = bit else { break };
            code = (code << 1) | bit;
            if let Some(value) = decode_macroblock_address_increment(code, length) {
                increment = Some(value);
                break;
            }
        }
```
What is wrong: a truncation inside the VLC (`read_bit` returns None mid-walk) is silently folded into `first_macroblock_address_increment = None` — indistinguishable from a legitimate escape/no-match, never surfaced as the module's typed `Truncated` error, though the field doc (:590-591) pre-admits it ("escape or damage").
Why it matters: contradicts "Individual header parsers are total … no damage smoothing" (mod.rs:41-43): here damage IS smoothed, by a design choice recorded only in a field comment. A caller cannot distinguish "escape VLC" from "buffer ran out".
What correct looks like: if `read_bit` fails before a match, return `Err(Mpeg12Error::Truncated …)`; keep `None` only for the 11-bit no-match (escape) case.
Confidence: HIGH — Verification: read (`.ok()` swallows the None; BitReader's exhaustion behavior is part1's bitreader.rs).

### F-004 [S4]
Where: crates/codecs/src/video/mpeg12/headers.rs:1032-1065
Code:
```
        for (code, len) in [
            (0b1u32, 1),
            (0b011, 3),
            (0b010, 3),
```
What is wrong: the test transcribes the SAME table the production function encodes (599-635) and asserts the bijection 1..=33. A permutation mutation inside the production table fails (good), but a spec-transcription error made identically in both lists passes — the test's "truth" is a copy, not the spec. The comment (1070-1073) admits prefix-freeness is only "spot-checked by the value bijection", which verifies nothing about prefixes: a table where one code is a proper prefix of another (a real decode-correctness hazard for the greedy walk at 670) still passes.
Why it matters: the greedy VLC matcher's correctness DEPENDS on prefix-freeness, and the one test that could pin it doesn't; the differential corpus (mpeg12_corpus.rs, ffmpeg-authored streams) only pins codes that occur in those streams.
What correct looks like: enumerate all bitstrings of length ≤ 11 and assert: no production code is a proper prefix of another, and the greedy walk yields the table's answer for every bitstring — a real prefix-freeness proof, ~20 lines.
Confidence: HIGH — Verification: read (both tables sed-compared — the test list 1032-1065 is the production list 601-633 verbatim).

## 4. Per-function table (chunk lines 526–1105)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `impl PictureCodingExtension::parse` | 531-572 | REAL | S4 | field order/widths spec-correct (§6.3.5); id gate 0b1000 correct; F-002 missing value gates |
| `struct SliceHeader` | 579-593 | REAL | — | field docs honest incl. the None semantics |
| `decode_macroblock_address_increment` | 599-636 | REAL | — | all 33 codes match Table B-1 (hand-checked); `_ => None` honest escape handling; `#[must_use]` |
| `impl SliceHeader::parse` | 645-685 | REAL | S4 | zero-slice gate correct (646-650); §6.2.4 layout correct; greedy walk sound given prefix-free table; F-001 (qscale 0 unvalidated), F-003 (truncation→None) |
| `tests::Writer::{new,put,finish}` | 694-726 | REAL | — | test-side MSB-first mirror; `finish` pads correctly; no overflow (u32 shifts < 32) |
| `sequence_header_roundtrip_ffmpeg_shaped` | 728-750 | REAL(test) | — | part1 target; fails on bit-offset regression |
| `sequence_header_with_quant_matrices` | 752-778 | REAL(test) | — | part1 target; 128 matrix bytes exercised |
| `sequence_header_rejects_forbidden_codes` | 780-812 | REAL(test) | — | negative gates for part1's aspect/framerate checks |
| `sequence_header_marker_violation_is_typed` | 814-829 | REAL(test) | — | marker-bit enforcement pinned |
| `sequence_extension_roundtrip_and_chroma_gate` | 832-877 | REAL(test) | — | roundtrip + forbidden chroma 00 + foreign id — honest triple |
| `frame_rate_extension_refines` | 879-897 | REAL(test) | — | 25×(1+1)/(0+1)=50 — extension arithmetic pinned |
| `gop_header_roundtrip` | 899-914 | REAL(test) | — | timecode string pinned |
| `picture_header_i_p_b_shapes` | 916-961 | REAL(test) | — | I/P/B shapes + forbidden coding types 0/5/7 |
| `picture_coding_extension_roundtrip` | 963-996 | REAL(test) | — | every field + foreign id; covers F-002's parse |
| `slice_header_with_first_mb_increment` | 998-1027 | REAL(test) | — | VLC 1, VLC 2, escape→None; does NOT cover quantiser 0 (F-001 gap) |
| `mb_increment_table_covers_one_through_thirty_three` | 1029-1080 | REAL(test) | S4 | F-004: duplicated-table truth; prefix-freeness not actually verified |
| `frame_rate_table_matches_spec` | 1082-1095 | REAL(test) | — | independent spec constants (24_000/1001 etc.) — honest, catches transcription |
| `coding_type_letters_and_motion_vectors` | 1097-1104 | REAL(test) | — | trivial enum coverage |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all roundtrip/gate tests (above) | YES | bit-exact Writer→parse→assert on named fields; negative tests pin typed errors |
| `mb_increment_table_covers_one_through_thirty_three` | PARTIAL | catches in-table permutations (production copy vs test copy disagree) but NOT shared transcription errors nor prefix violations (F-004) |
| `frame_rate_table_matches_spec` | YES | expected values are independent spec literals, not copies of the table's own entries |
No `#[ignore]`, no mocks, no goldens generated by code-under-test (Writer is an independent mirror). All 13 executed ok in baseline (test-workspace.log.txt:758-771).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no software MPEG-1/2 decode here … parses and classifies bitstream structure only" | mpeg12/mod.rs:5-11 | REAL | no Decoder impl under mpeg12/ (grep: zero `impl Decoder` in the module tree); pipeline_host has no mpeg12 arm (`cli.codec_unavailable` fall-through :374-380) |
| "Software decode remains `blocked` in the feature matrix" | mpeg12/mod.rs:8-9 | REAL-UNPROVEN | matrix row audit belongs to the matrix-rows agents; code side confirmed: no decode path exists |
| "Individual header parsers are total … no damage smoothing" | mpeg12/mod.rs:41-43 | SHALLOW | F-001/F-002 forbidden values accepted; F-003 truncation smoothed to None — three gaps in this chunk alone |
| Table B-1 codes "1..=33 modeled; the escape and unused suffixes return `None`" | headers.rs:595-597 | REAL | hand-checked vs spec; escape `0b00000001000` returns None (test :1078) |
| wave-6 hostile question (H.264 reachability) | task prompt | N/A for this file | mpeg12 is not H.264; module compiles unconditionally (codecs/Cargo.toml:25-30 "IN-HOUSE decoders … stay unconditional") yet is production-dead |

## 7. Dead / duplicated / unreachable code
- The ENTIRE mpeg12 module (this chunk included) is production-dead: compiled into every build (unconditional `pub mod mpeg12`, video/mod.rs:39), consumed only by facts.rs, which is consumed only by tests/mpeg12_corpus.rs. UNREACHABLE from `lumen`/`lumen-tauri`. Demux mints mpeg video descriptors without touching this parser (ts/mod.rs:351-352, ps/mod.rs:771-784 hard-coded strings).
- The advertised consumers ("hw-feed seam" Phase-7 adapters, "future self-build") do not exist in the tree — cited as intent only.
- No duplication found within the chunk.

## 8. Dependencies & flags
- No external deps; std + `Mpeg12Error` (error.rs) + `BitReader` (bitreader.rs, part1) + testkit (dev). No feature gate — in-house modules unconditional (codecs/Cargo.toml comment :25-30). No unsafe. Lint-clean per baseline clippy (#5/#6 CLEAN).

## 9. Open questions for the next agent
- facts.rs owner: does `Mpeg12Facts::scan`'s output feed ANY product surface (probe/track-info)? My grep says no, but facts.rs is your file to confirm.
- The feature-matrix row for MPEG-1/2 video (software decode `blocked`) — matrix-rows agent should confirm the row's evidence string doesn't cite this parser as if it decoded.
