# AUDIT: crates/codecs/src/video/h264/parser/slice.rs#part4
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-21 |
| Lines / functions | ~1593–2123 of 2124 (chunk part4 of 4; part1 ≈1–530 W06-A05, part2 ≈531–1061 W06-A04, part3 ≈1062–1592 W06-A04) / 7 tests (one boundary-straddling) |
| File verdict | REAL (subject code) — test evidence split: hand-built lanes REAL, corpus lanes vacuous in baseline |
| Reachable from shipped entry point? | YES for the parser: `parse_slice_header` ← `H264SwDecoder` config/decode path (`pipeline_host.rs:347` `"h264" \| "avc1"` → `codecs::video::h264::H264SwDecoder`), and hw lane (unreachable) uses it too. Parser itself is `pub` in an unconditional module (codecs/Cargo.toml: h264 in-house, no feature gate). Test module: `#[cfg(test)]`, not shipped. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg -n "fn \|test"` (structure map); `ls /tmp/my-project/corpus-v4/media` → absent; `awk '/506 passed/' audit/runs/baseline/test-workspace.log.txt` → line 783: `finished in 0.11s`; `sed -n` on every quoted range; spec check of §8.2.1.1/§8.2.1.3 arithmetic by hand |

## 1. One-paragraph verdict
Part4 is the back half of the slice-header test module: two corpus differentials vs ffmpeg traces, a bit-exact hand-built "all optional syntax gates" header, an IDR/colour-plane/poc-type-2 header, a truncation sweep, a typed-error matrix, POC arithmetic goldens, and a deterministic mutation property test. The hand-built lanes are genuinely hostile and correct (I re-derived the se/ue encodings by hand). But four of the eight tests in/straddling this range are corpus-gated on `/tmp/my-project/corpus-v4/media` (absent) and **skip-passed vacuously in the recorded baseline** — the codecs lib suite ran 506 tests in 0.11 s (baseline line 783), which is impossible if even one ffmpeg/ffprobe subprocess had run, so every "matches the ffmpeg trace" differential in this file produced zero executed evidence. Additionally, the IDR fixture omits the two `dec_ref_pic_marking` flag bits the spec requires and the asserted values are accidentally the first two bits of the following `se(3)`.

## 2. What actually works (verified)
— Hand-built lanes executed in baseline: `idr_header_with_colour_planes_and_poc_type2`, `invalid_values_are_the_typed_error_paths`, `poc_arithmetic_helpers_follow_the_spec_rules`, `hand_built_headers_exercise_the_optional_syntax_gates`, `slice_type_mapping…` all `ok` (test-workspace.log.txt:613-620) — no corpus needed, no subprocess, consistent with 0.11 s.
— POC goldens verified by hand against §8.2.1.1/§8.2.1.3: `pic_order_cnt_msb_type0(0,62,2,64)=64` (62−2=60≥32), `(64,2,60,64)=0` (58>32), type-2 `2·(9+5)=28` / non-ref `27`, `2·(2³²−1+2³²−1)=17179869180` — the code's `>=` vs `>` branch asymmetry (slice.rs:1128,1130) matches the spec text exactly.

## 3. Findings
### F-001 [S2]
Where: crates/codecs/src/video/h264/parser/slice.rs:2070-2073 (same pattern 1513-1516, 1612-1615, 1849-1852)
Code:
```
        let Some((sps, pps, _)) = corpus_0019() else {
            eprintln!("corpus: 0019 not present — skipping mutation property");
            return;
        };
```
What is wrong: Four tests in this range (`corpus_0019_slice_headers_match_the_ffmpeg_trace`, `corpus_0014_mkv_lane_matches_the_ffmpeg_trace`, `truncated_headers_are_typed_never_panics`, `mutated_slice_streams_are_typed_or_ok_never_panics`) silently `return` — reporting `ok` — when the corpus directory is absent. The corpus default root is `/tmp/my-project/corpus-v4/media` (corpus_path, 1399-1404, part3's range); `ls /tmp/my-project/corpus-v4/media` → no such directory in this sandbox, and the recorded baseline codecs-lib suite is `506 passed … finished in 0.11s` (test-workspace.log.txt:783) — `corpus_0014_annexb()` shells out to `ffmpeg` (1440-1455) and the cross-check to `ffprobe` (1462-1478); a single subprocess would alone exceed 0.11 s. W05-A09 proved the identical pattern for every demux corpus lane (W05-A09.log.md:24,27).
Why it matters: The baseline "ok" lines for the ffmpeg-trace differentials are vacuous; anyone citing the baseline as proof the slice parser matches x264/ffmpeg output is citing tests that asserted nothing. The truncation sweep and the 240-round mutation property — the module's only malformed-input coverage on *real* streams — also never ran.
What correct looks like: Fail (or `#[ignore]` with a loud label) when the corpus env var is unset in CI; or commit a minimal in-repo Annex-B fixture; at minimum assert-skip must surface as `ignored`, not `ok`.
Confidence: HIGH — Verification: read + `ls` of the corpus root + baseline timing arithmetic (0.11 s / 506 tests) + cross-wave proof (W05-A09).
### F-002 [S3]
Where: crates/codecs/src/video/h264/parser/slice.rs:1817-1825
Code:
```
        w.put(0b10, 2); // colour_plane_id
        w.put(0xABCD, 16); // frame_num, 16 bits
        w.ue(9); // idr_pic_id
        w.se(3); // slice_qp_delta
        w.rbsp_trailing();
```
What is wrong: For an IDR picture with `nal_ref_idc != 0`, §7.3.3 requires `no_output_of_prior_pics_flag` u(1) and `long_term_reference_flag` u(1) between `idr_pic_id` and `slice_qp_delta`. The fixture omits both, so the parser consumes the first two bits of `se(3)` (= "00110") as those flags — `0`,`0` — and the asserted `DecRefPicMarking::Idr { false, false }` (1837-1843) is an accident of bit alignment. The real `slice_qp_delta` parsed is `se("110") = +1`, not the commented `3`; the test never asserts `slice_qp_delta`, hiding the corruption.
Why it matters: The fixture is a spec-invalid stream that the parser accepts; the comment lies about what the bytes mean; anyone extending this test (e.g., asserting qp) or copying the fixture pattern inherits a misaligned stream. A parser that read the IDR flags in the wrong order would still pass.
What correct looks like: `w.put(0,1); w.put(0,1);` for the two IDR marking flags before `w.se(3)`, plus an assertion on `slice_qp_delta == 3` to lock the alignment.
Confidence: HIGH — Verification: read; manual ue/se decode: `se(3)` → k=5 → "00"+"110"; consumed as `0`,`0` flags + `se("110")=1`.
### F-003 [S4]
Where: crates/codecs/src/video/h264/parser/slice.rs:1603-1607
Code:
```
        if let Some(frames) = ffprobe_frame_count(&path) {
            assert_eq!(slices.len() as u32, frames);
        } else {
            eprintln!("corpus: ffprobe unavailable — frame-count cross-check skipped");
        }
```
What is wrong: The ffprobe cross-check degrades to a silent no-op when ffprobe is missing (nested under F-001's corpus gate, so doubly conditional); the test name still says "matches the ffmpeg trace".
Why it matters: Two independent skip conditions mean the "trace match" claim rests entirely on hard-coded in-test constants; there is no executed external oracle anywhere in this file's evidence.
What correct looks like: Treat a present corpus + missing ffprobe as an environment error, not a pass.
Confidence: HIGH — Verification: read.

## 4. Per-function table (part4 range; `corpus_0019_slice_headers…` 1511–1608 straddles the part3 boundary — its tail 1593–1608 is in range)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `corpus_0019_slice_headers_match_the_ffmpeg_trace` (tail) | 1593-1608 | SHALLOW (in baseline) | F-001, F-003 | B-slice direct-pred flag assert + ffprobe count cross-check; both vacuous when corpus absent. Assertions themselves are strong (26 slices, I2/P8/B16, adaptive=7, qp range −26..=−24, deblock idc 1). |
| `corpus_0014_mkv_lane_matches_the_ffmpeg_trace` | 1610-1670 | SHALLOW (in baseline) | F-001 | 185 slices, I4/P46/B135, idr ids {0,1}, adaptive=42, qp −3..=4; would be a real differential with the corpus mounted; today: zero executed assertions. |
| `hand_built_headers_exercise_the_optional_syntax_gates` | 1672-1793 | REAL | — | Bit-exact writer/parser round-trip across every optional gate (field pic, poc type 1, redundant_pic_cnt, ref-list mod, weighted pred ×4 entries, mmco, CABAC idc, deblock offsets); I verified writer order == parser order element-by-element. Executed in baseline. |
| `idr_header_with_colour_planes_and_poc_type2` | 1795-1845 | REAL | F-002 | Colour-plane (§7.3.3 separate_colour_plane_flag), 16-bit frame_num, type-2 POC, Idr marking — but the fixture bit-stream is spec-invalid (F-002); asserted fields land correctly by alignment accident. |
| `truncated_headers_are_typed_never_panics` | 1847-1877 | SHALLOW (in baseline) | F-001 | Every prefix cut of a real slice NAL must be `Ok` only when `cut > 2` else typed Bit error — good design, vacuous without corpus. |
| `invalid_values_are_the_typed_error_paths` | 1879-2026 | REAL | — | Nine typed-refusal lanes: InvalidPpsId(256), InvalidSliceType(10), FmoUnsupported, InvalidContext, NotAVclNal(0x67), ForbiddenBitSet(0xC1/0x85), InvalidMmco(7), InvalidModOfPicNumsIdc(6), InvalidNumRefIdxActive(32), GolombOverflow — all executed, all content-asserted. |
| `poc_arithmetic_helpers_follow_the_spec_rules` | 2028-2064 | REAL | — | Spec arithmetic goldens incl. overflow boundaries (i64::MAX wrap, u32::MAX accumulation); verified by hand. |
| `mutated_slice_streams_are_typed_or_ok_never_panics` | 2068-2122 | SHALLOW (in baseline) | F-001 | 240-round deterministic LCG mutation (xorshift seed 0x5F1BC0DE5EED), flips biased into the first 16 bytes, truncation derivative every 3rd round, `errs > 50` bite-guard — excellent property test, vacuous without corpus. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `hand_built_headers…`, `invalid_values…`, `poc_arithmetic…`, `idr_header…` (partly) | YES (with F-002 caveat) | Hand-built bits, exact-equality on typed errors and goldens; executed in baseline. |
| `corpus_0019…`, `corpus_0014…`, `truncated_headers…`, `mutated_slice_streams…` | NOT IN THE RECORDED RUN | Corpus absent → silent `return` before the first assertion (F-001). With the corpus mounted they are strong differentials. |
| `idr_header…` `dec_ref_pic_marking` assert | WEAK | Values supplied accidentally by misaligned bits (F-002); only `false/false` is ever tested. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "matches the ffmpeg trace" (test names) | slice.rs:1512, 1611 | REAL-UNPROVEN | Strong hard-coded expectations exist, but no executed ffmpeg/ffprobe oracle in the recorded baseline (F-001/F-003); the worklog's trace_headers ground truth was generated at dev time, not by this suite. |
| Mutation property "+FB (card 5.F1b): deterministic mutation property — every mutant yields Ok or a typed error, never a panic or hang" | slice.rs:2066-2067 | REAL-UNPROVEN | Design is real and deterministic; not executed in baseline (F-001). |

## 7. Dead / duplicated / unreachable code
- No dead code in range. Cross-part note for W06-A04 (part3 owns it): `corpus_path`'s hard-coded default `/tmp/my-project/corpus-v4/media` (1399-1404) is a sandbox-layout literal — root cause of F-001; the `lumen-allow-literal` marker acknowledges it.

## 8. Dependencies & flags
- No feature gates: parser modules are unconditional in-house code. Test-only subprocess deps: `ffmpeg`, `ffprobe` on PATH (dev-time oracles, absent → silent skips). No unsafe in this range.

## 9. Open questions for the next agent
- W06-A04/A05 (parts 3/2): do `parse_slice_header`'s own unit paths (738-973) have non-corpus malformed coverage, or is the truncation/mutation lane the only one (making F-001 a coverage hole for the parser core)?
- REQUEST-ORCH-RUN (filed in my log): mount corpus-v4 and rerun `cargo test -p codecs --lib video::h264::parser::slice` to convert F-001's four vacuous lanes into real evidence.
