# AUDIT: crates/codecs/src/video/h264/parser/slice.rs#part3 (lines 1062–1592)
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-11-21 |
| Lines / functions | 531 / 17 (5 production fns — one straddling the 1062 boundary, 8 test-scaffolding helpers, 2 tests, BitWriter 4 fns) — chunk of a 2123-line file, equal 4-way split: part1=1–531, part2=532–1061, part3=1062–1592, part4=1593–2123 |
| File verdict | UNREACHABLE (real, spec-correct, test-executed code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `parse_slice_header`'s only production caller is `crates/codecs/src/video/h264/decode.rs:516` (`H264Decoder`, the hw lane), which no shipped host constructs (rg: zero `H264Decoder`/`from_setting` references in apps/cli, apps/desktop, apps/desktop-tauri — pipeline_host.rs:342 mentions it in a comment only); the POC helpers' only production caller is the VA-API session (hw/vaapi/session.rs:1023,1204), Phase-7 opt-in, also unwired. The shipped H.264 lane (`sw.rs`) imports only `parser::nal` framing (sw.rs:92–94), never `parser::slice`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n` structure map (all 40 items); `sed -n '1010,1592p'` full read of range + context; `rg "parse_slice_header|pic_order_cnt_msb_type0|next_frame_num_offset|pic_order_cnt_type2"` across crates+apps (callers: decode.rs:516/532-538, vaapi/session.rs:1023/1204); `rg "H264Decoder"` in apps (comment only); `sed -n '22,94p' sw.rs` (imports — nal only); `sed -n '49,110p' fuzz.rs` (fuzz coverage check); `rg "skipping"` audit/runs/baseline/test-workspace.log.txt (absent → corpus tests RAN, log lines 614/618) |

## 1. One-paragraph verdict
The production half of this chunk (pred-weight entries, the MMCO loop, the three POC/frame-num arithmetic helpers) is textbook-clean: I checked each against ITU-T H.264 §7.3.3.2/§7.3.3.4/§8.2.1.1/§8.2.1/§8.2.1.3 and found exact matches, including the spec's asymmetric `>=`/`>` boundary in the POC type-0 wrap rule and checked arithmetic on every add/mul. The test half is real differential work (hand-built bitstreams + a 26-slice corpus lane asserted against an ffmpeg trace — executed in baseline, not skipped). Two structural defects: (1) all of this is dead code in the shipped binary — the sw decoder path never calls the in-house slice parser, and the hw consumers (decode.rs, VA-API) are not wired into any shipped host; (2) the corpus/ffmpeg/ffprobe-dependent tests default to a hard-coded Lead-sandbox path and silently self-skip anywhere else, and the fuzz harness that claims to cover "every parser surface" never touches the slice-header parser at all.

## 2. What actually works (verified)
— `pic_order_cnt_msb_type0` (1122–1143) matches spec §8.2.1.1 exactly, including the deliberate asymometry (`>=` on the wrap-forward branch, `>` on the wrap-backward branch — the same shape as ffmpeg's `compute_poc`); both branches `checked_add/sub`. — verified by read + spec comparison.
— `next_frame_num_offset` (1144–1168) and `pic_order_cnt_type2` (1169–1188) match §8.2.1/§8.2.1.3 (IDR → 0; wrap → +MaxFrameNum checked; non-reference → −1); every arithmetic op checked.
— `read_mmco_ops` (1072–1121): Table 7-9 idc 0..=6 mapped field-for-field, unknown idc typed-refused, loop bounded at 257 stored ops.
— Corpus differential tests executed for real in baseline: `test video::h264::parser::slice::tests::corpus_0019_slice_headers_match_the_ffmpeg_trace ... ok` and `...corpus_0014_mkv_lane_matches_the_ffmpeg_trace ... ok` (audit/runs/baseline/test-workspace.log.txt:614,618) with zero "skipping" lines in the log → the corpus and system ffmpeg/ffprobe were present.

## 3. Findings
### F-001 [S3] Corpus/ffmpeg-dependent tests silently skip outside the Lead sandbox; default path is a hard-coded `/tmp/my-project/...` literal
Where: crates/codecs/src/video/h264/parser/slice.rs:1399-1405 (and skip arms 1514-1517, 1613-1616)
Code:
```rust
    pub(crate) fn corpus_path(name: &str) -> Option<std::path::PathBuf> {
        let dir = std::env::var("LUMEN_H264_CORPUS") // lumen-allow-literal: cfg(test) corpus-gate env-var name — test harness knob, not product config
            .unwrap_or_else(|_| "/tmp/my-project/corpus-v4/media".to_string()); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
```
What is wrong: The default corpus location is an absolute Lead-sandbox path that does not exist in any other clone. When absent, `corpus_0019_slice_headers_match_the_ffmpeg_trace` and `corpus_0014_mkv_lane_matches_the_ffmpeg_trace` print to stderr and **pass** — the strongest tests this module has (26 real slices vs an ffmpeg trace, MMCO census, POC ranges) become no-ops. Same for the `ffmpeg`/`ffprobe` process spawns (1438–1460, 1462–1478): absent binaries → silent skip. Baseline proves they ran in the Lead sandbox (log lines 614/618, no skip messages); every other environment gets green lies.
Why it matters: The module's spec-conformance evidence is environment-conditional and quietly degrades to "tests that pass while testing nothing" — the exact pattern this audit hunts.
What correct looks like: commit small corpus fixtures under `tests/fixtures/` (the repo already has that convention — `cargo xtask fixtures --check` PASS, baseline #12), or fail the test loudly when `LUMEN_H264_CORPUS` is unset and no default exists.
Confidence: HIGH — Verification: read + baseline log inspection; `/tmp/my-project` existence not checked in this sandbox (UNVERIFIED whether it exists here — irrelevant, the portability defect stands).
### F-002 [S3] Slice-header parser has zero fuzz coverage while the fuzz harness claims "every parser surface"
Where: crates/codecs/src/video/h264/fuzz.rs:5-8 (the claim) vs slice.rs:738 (the uncovered surface)
Code (fuzz.rs doc):
```rust
//! One call = one untrusted byte buffer through **every** parser
//! surface of the landed parser self-build (card 5.F1; decode itself
```
What is wrong: `fuzz_codec_h264` (fuzz.rs:49-73) exercises the Annex-B walk, SPS, PPS, SEI, avcC and the length-prefixed walk — but never `parse_slice_header`, `read_pred_weight_table`, `read_pred_weight_entry`, or `read_mmco_ops`, the largest hostile-input surface in the parser module (the 738–1121 line range). The doc's "every parser surface" is stale at best (slice.rs landed as card 5.F1b) and false at worst. The h264 module doc's totality promise ("any byte string yields either a parsed structure or a typed error — no panics, no unbounded work") cites this fuzz target as its (self-admitted "future") guarantee — for slice headers there is no fuzz evidence at all.
Why it matters: The MMCO/weight-table/POC helpers take attacker-controlled Exp-Golomb values; only unit tests with hand-built and x264-authored streams guard them. A fuzz target that names itself after the parser but skips the parser's biggest file provides false assurance to future maintainers.
What correct looks like: Add a pass to `fuzz_codec_h264` that runs `parse_slice_header` over every VCL NAL with the parsed SPS/PPS (or minimal bootstrap contexts), and fix the doc wording to enumerate what is actually covered.
Confidence: HIGH — Verification: read of fuzz.rs:49-73 (no slice call) + `rg` for other callers.
### F-003 [S4] Loop-budget numbers contradict: enum doc says "adaptive-marking operations ≤ 32", the enforced budget is 256 (fires at 257)
Where: crates/codecs/src/video/h264/parser/slice.rs:1113-1121 (the check) vs 206-210 (the doc, part1's range) and 726-731 (the const, part2's range)
Code:
```rust
        if ops.len() > MAX_OPS_PER_LOOP {
            return Err(SliceHeaderError::OpBudgetExceeded {
                kind: "dec_ref_pic_marking",
                ops: ops.len(),
            });
```
What is wrong: `SliceHeaderError::OpBudgetExceeded`'s doc claims spec-conformant streams have "adaptive-marking operations ≤ 32"; `MAX_OPS_PER_LOOP = 256` and the check fires only at 257 stored ops; the const's own comment says "spec-conformant streams never reach a tenth of this" (≈ 25). Three different numbers for the same supposed bound, none with a spec citation (the spec has no hard MMCO-count limit — the loop terminates via idc 0 and is input-bounded). Also: the enum doc lists only kinds `"ref_pic_list_modification"`/`"dec_ref_pic_marking"` while part2's `read_pred_weight_table` (1026–1041) emits undocumented kinds `"pred_weight_l0"`/`"pred_weight_l1"` — and mislabels truncation errors as budget trips (noted for part2's agent, §9).
Why it matters: Internal contradictions in error-taxonomy docs are how later "evidence" gets fabricated from prose.
What correct looks like: One number, one comment, one doc: budget 256 (defensive, input-bounded anyway), doc updated, kinds enumerated.
Confidence: HIGH — Verification: read of all three sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `read_pred_weight_entry` (straddles 1062 boundary — audited whole, flagged to part2 agent) | 1055–1071 | REAL (unreachable) | — | §7.3.3.2 exact: luma flag → 2×se, chroma flag (only `chroma_array_type != 0`) → 4×se; all reads typed `BitReadError`; no panics |
| `read_mmco_ops` | 1072–1121 | REAL (unreachable) | S4 | Table 7-9 exact for idc 0..=6; unknown → `InvalidMmco`; bounded 257 (F-003) |
| `pic_order_cnt_msb_type0` | 1122–1143 | REAL (unreachable) | — | §8.2.1.1 exact incl. `>=`/`>` asymmetry; `checked_add/sub`; `#[allow(clippy::missing_panics_doc)]` is unnecessary-looking but harmless |
| `next_frame_num_offset` | 1144–1168 | REAL (unreachable) | — | §8.2.1 exact; `checked_add` on wrap; IDR → 0 |
| `pic_order_cnt_type2` | 1169–1188 | REAL (unreachable) | — | §8.2.1.3 exact; field-picture offsets honestly declared out of scope; all arithmetic checked |
| `annexb_nals` (test scaffolding) | 1199–1234 | REAL | — | 3/4-byte start codes; adjacent-code empty NALs skipped; no panics (indexing guarded by `end <= start` continue) |
| `strip_emulation_prevention` (test scaffolding) | 1236–1255 | REAL | — | `00 00 03 {<=03}` removal correct; a trailing `00 00 03` at EOS is kept (malformed input, harmless) |
| `bootstrap_sps` (test scaffolding) | 1257–1310 | REAL | — | `unwrap_or_default()` everywhere → a truncated SPS silently yields defaults; acceptable test-only (corpus asserts would catch wrong geometry), but fragile |
| `bootstrap_pps` (test scaffolding) | 1314–1348 | REAL | — | `assert_eq!(num_slice_groups_minus1, 0)` — panics on FMO fixtures; test-only, documented |
| `BitWriter::new/put/ue/se/rbsp_trailing` (test scaffolding) | 1351–1397 | REAL | — | MSB-first writer, correct ue/se encoding (verified against read_ue/read_se semantics in part1) |
| `corpus_path` (test scaffolding) | 1399–1405 | REAL | S3 | F-001 |
| `corpus_0019` (test scaffolding) | 1408–1436 | REAL | — | silent-skip ladder; `inspect_err` prints parse failures |
| `corpus_0014_annexb` (test scaffolding) | 1438–1460 | REAL | — | spawns system `ffmpeg` |
| `ffprobe_frame_count` (test scaffolding) | 1462–1478 | REAL | — | spawns system `ffprobe`; parse failure → skip |
| `slice_type_mapping_covers_all_ten_codes_with_aliases` | 1481–1510 | REAL | — | see §5 |
| `corpus_0019_slice_headers_match_the_ffmpeg_trace` | 1512–1609 | REAL | S3 | see §5; F-001 skip hazard |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `slice_type_mapping_covers_all_ten_codes_with_aliases` (1481) | YES | all 10 Table 7-7 codes with kind+alias pinned, plus 4 invalid codes asserted typed — a mapping mutation fails it deterministically |
| `corpus_0019_slice_headers_match_the_ffmpeg_trace` (1512) | YES — **when the corpus is present**; NO-OP otherwise | 40+ concrete assertions against a real x264 stream (26 slices, I2/P8/B16, single IDR with exact marking flags, POC lsb < 64, qp range −26..=−24, deblock idc 1, adaptive count 7, ffprobe frame-count identity); with the corpus absent it prints and passes (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every entry point is total: any byte string yields either a parsed structure or a typed error — no panics, no unbounded work (the `fuzz_codec_h264` target's **future** guarantee)" | h264/mod.rs:44–47 | REAL-UNPROVEN for slice.rs | hand-built/truncated-input unit tests exist (part4's range) and the corpus lane ran (baseline log 614/618), but no fuzz target exercises this file (F-002); the doc's own "future" hedge is honest |
| "One call = one untrusted byte buffer through **every** parser surface of the landed parser self-build" | fuzz.rs:5–6 | FALSE | `fuzz_codec_h264` (49–73) never calls the slice-header parser (F-002) |
| "spec-conformant streams never do: … adaptive-marking operations ≤ 32" | slice.rs:206–210 (OpBudgetExceeded doc) | FALSE as a spec claim | no such spec bound; code enforces 256 (F-003) |

## 7. Dead / duplicated / unreachable code
- Everything in this chunk is unreachable at runtime from shipped entry points (see header) — it is future hw-lane code (decode.rs/VA-API/VideoToolbox consumers) compiled into every shipped binary. The tests keep it honest in the Lead sandbox only (F-001).
- Duplication note: `annexb_nals`/`strip_emulation_prevention` here duplicate production `parser::nal` framing (`AnnexBNals`, `unescape_rbsp_into`) for test independence — intentional seam, documented (module docs "test-only; production equivalents are card 5.F1a's files"), acceptable.

## 8. Dependencies & flags
- No feature gates on the parser: `pub mod parser` is unconditional (h264/mod.rs:52). Compiles into every shipped build.
- Wave-6 hostile question: the shipped H.264 decode lane does NOT go through this file — `H264SwDecoder` (sw.rs) uses `rust_h264`'s internal parsing and only `parser::nal` framing (sw.rs:92–94), seeded at pipeline_host.rs:346 (default features, see mjpeg/decoder.rs#part2.audit.md §8 for the full flag trace). So H.264 decode IS reachable and works (baseline: sample-420.mp4 plays), but this parser module is a parallel in-house implementation that the shipped path bypasses entirely.

## 9. Open questions for the next agent
- part2 agent: `read_pred_weight_table` (1010–1053) maps **truncation** `BitReadError`s to `OpBudgetExceeded { kind: "pred_weight_l0/l1" }` (1026–1031, 1036–1041) — a truncated bitstream is reported as a loop-budget trip; also emits kinds the enum doc doesn't list (F-003). Verify whether any caller distinguishes these.
- part4 agent: the truncated-hostile-input tests (1848+) and `poc_arithmetic_helpers_follow_the_spec_rules` (2029+) — confirm they pin the `>=`/`>` boundary cases of `pic_order_cnt_msb_type0` (my read says the code is spec-exact; the tests should prove it).
- decode.rs / hw-lane owners: when `H264Decoder` gets wired (Phase 7), this file becomes reachable overnight — the F-002 fuzz gap should close first.
