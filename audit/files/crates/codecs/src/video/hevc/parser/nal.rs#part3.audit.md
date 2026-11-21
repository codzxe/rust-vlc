# AUDIT: crates/codecs/src/video/hevc/parser/nal.rs#part3 (lines 1008–1517; 5-line overlap into part2)
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2026-11-21 |
| Lines / functions | 510 audited (assigned ≈1013–1517 of 1518; start pulled back to 1008 to keep `nal_classification_roundtrips_table_7_1` whole — 5-line overlap with W06-A04's part2, not re-audited; the test harness `Bits`/`corpus_path`/`splitmix64` at 893–1005 are part2's and referenced but not re-audited) / 12 items in table (10 tests + 1 helper fn + 1 corpus lane) |
| File verdict | UNREACHABLE (test-module chunk; the production code it tests is itself production-dead) |
| Reachable from shipped entry point? | NO — the tree's only `Decoder` impl (`decode.rs`'s `HevcDecoder`, re-exported hevc/mod.rs:52) has ZERO production callers: `rg -rn "HevcDecoder"` over crates/+apps/ → its definition (hevc/decode.rs), the mod.rs re-export, and `crates/codecs/tests/hevc_hw_path.rs` only (the test FakeBridge session). `apps/cli/src/pipeline_host.rs` has NO `"hevc"` arm (rg over the CLI host: no hevc/hev1/hvc1 dispatch); demux mints `hevc` descriptors no shipped decoder accepts — the baseline smoke proves it: `sample-hevc.mp4` → Opening → **Error** → Idle (INDEX.md corrected matrix). The parser types ARE imported inside the unreachable tree (hevc/hw.rs:63-67 `use super::parser::{nal::NalUnitType, pps…, slice…, sps…, vps…}`, decode.rs likewise) — unreachable transitively, since nothing outside tests constructs the decoder. The tests ARE executed by `cargo test` (baseline: test-workspace.log.txt:632-641, 781 — all ok) but test code is not a shipped entry point. |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:3 |
| Commands run | `grep -n "^fn \|^pub fn …"` (structure map); `rg -rn "HevcDecoder"` over crates/+apps/ (zero production callers — the unreachability proof, see header) + `rg -n "super::parser"` over hevc/ (hw.rs:63-67, decode.rs, fuzz.rs import the parser — initial `rg -ln "hevc::parser"` pattern missed the `super::` form; corrected); `ls /tmp/my-project/corpus-v4/media/0044-raw-video.265` (ABSENT) + `find / -maxdepth 4 -name corpus-v4` (no hit anywhere) — the corpus-skip evidence chain; baseline log sed :632-641, :781, :1088-1117 (hevc corpus test results + durations; hevc_parser_corpus 3 tests in 0.01s — sub-ffprobe-spawn time, i.e. skipped); `rg -n "0044"` over docs/feature-matrix.toml + docs/formats/video-hevc.md (evidence claims citing the corpus lanes); sed of tests/hevc_parser_corpus.rs:35-41 (corpus_dir default `/tmp/my-project/corpus-v4`, env `LUMEN_CORPUS` vs nal.rs's `LUMEN_HEVC_CORPUS`); hand-verification of the emulation-removal expected vectors and Table 7-1 classifications. |

## 1. One-paragraph verdict
This chunk is the test module of the HEVC NAL parser, and the tests are unusually good where they run: exhaustive 0..=63 NAL-type roundtrips, byte-exact field assertions against real x265 corpus constants pinned in-source (VPS/SPS/PPS_0100), the chained-escape emulation-prevention case pinned on real bytes, thorough typed-negative coverage of the length-prefixed splitter and hvcC record (truncation geometry, version, reserved bits, zero-length NALs, overruns), and a real fuzz-lite (10k random + 5k mutated seeds through every parser surface). The one real defect is evidence-shaped: the headline corpus lane (`corpus_0044_parameter_sets_and_slice_prefixes_parse`) silently self-skips when the external corpus-v4 tree is absent — and it IS absent in this sandbox (`/tmp/my-project/corpus-v4/media` does not exist; no corpus-v4 anywhere on the filesystem), so the baseline's green "1681 passed" includes at least one zero-execution pass, and `docs/formats/video-hevc.md`'s "3/3 corpus lanes PASS" claim plus feature-matrix row evidence citing "3/3 corpus lanes byte-verified" cannot be replicated from the baseline run (the sibling `hevc_parser_corpus.rs` binary finished 3 tests in 0.01s — physically too fast for its three ffprobe subprocess spawns, confirming the skip pattern in the baseline environment). All of this is production-dead anyway: nothing outside tests imports the hevc parser.

## 2. What actually works (verified)
- All 11 tests in this chunk pass in baseline — verified by test-workspace.log.txt:632-641 (`video::hevc::parser::nal::tests::* … ok`) and :781 (fuzz_lite ok) inside the codecs 506-passed suite.
- `nal_classification_roundtrips_table_7_1` — exhaustive `from_id(id).id() == id` for 0..=63 plus spot-checks against Table 7-1 (19=IDR_W_RADL, 21=CRA_NUT, 32/33/34=VPS/SPS/PPS, IRAP range 16..=23 with 24 excluded, VCL vs non-VCL) — hand-checked against ITU-T H.265 Table 7-1; mutation-killing.
- `emulation_removal_handles_overlapping_escapes` — the expected vector (14 escaped bytes → 11 RBSP bytes, three chained escapes collapsed to five zero bytes) hand-verified; this pins the exact case a consume-the-suffix unescaper gets wrong, on real x265 bytes.
- `hvcc_parse_field_exact_on_corpus_arrays` / `hvcc_typed_error_paths` — 20+ byte-exact field assertions on a faithfully rebuilt 0100 record (header layout hand-checked vs ISO/IEC 14496-15: version[0], profile bytes, level 0x5d=93 at [12], min_spatial_segmentation/parallelism at [13-14], chroma/bitdepth at [15-16], lengthSizeMinusOne at [21] high nibble → length_size 4, numOfArrays at [22]) and six distinct negative shapes — the strongest negative coverage in the parsers I audited this wave.
- `fuzz_lite_no_panics_on_random_and_mutated_bytes` — 10k splitmix64 random buffers framed under all three parameter-set headers plus 5k mutations (bit flips, byte sets, truncations, appends) of the real seeds through vps/sps/pps/hvcC/unescape; the Rust test harness fails on panic, so it is a real panic-freedom proof for those surfaces.

## 3. Findings

### F-001 [S2]
Where: crates/codecs/src/video/hevc/parser/nal.rs:1440-1448
Code:
```
    fn corpus_0044_parameter_sets_and_slice_prefixes_parse() {
        let Some(path) = corpus_path("0044-raw-video.265") else {
            eprintln!("SKIP hevc corpus: 0044-raw-video.265 absent (corpus re-imaged)");
            return;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("SKIP hevc corpus: 0044 unreadable");
            return;
        };
```
What is wrong: the corpus lane silently returns Ok when the external corpus is absent — the "skip" is an eprintln that cargo test captures (invisible unless the test fails), so the suite reports an indistinguishable `ok` for a test that executed ZERO assertions. The corpus is external (corpus-v4 lives at `/tmp/my-project/corpus-v4/media` by default, nal.rs:992-993) and is ABSENT in this sandbox (`ls /tmp/my-project/corpus-v4/media/0044-raw-video.265` → No such file or directory; `find / -maxdepth 4 -name corpus-v4` → no hit). The sibling binary `tests/hevc_parser_corpus.rs` (same skip pattern, same default root) ran 3 tests in 0.01s in baseline — each executed lane must spawn ffprobe (≥ tens of ms) — so those lanes skipped too; the same environment implies this test skipped in the baseline run as well.
Why it matters: docs/formats/video-hevc.md:182-190 claims "Corpus differential (live ffprobe 7.1.5): 3/3 lanes PASS … 0044-raw-video.265 … PASS", and docs/feature-matrix.toml:441 cites "cards 5.G1 parser 3/3 corpus lanes byte-verified" as row evidence — evidence the baseline run cannot reproduce because the lane skips green. A fresh clone/CI gets a green suite with zero real-stream verification of the HEVC parser, and nothing in the output distinguishes it. This is the "test proving nothing used as evidence" shape, softened only by the loud-at-failure eprintln.
What correct looks like: make corpus presence a first-class test dimension: `#[ignore = "requires corpus-v4 (LUMEN_HEVC_CORPUS)"]` so skips show in the `0 ignored` count, or panic (not return) when `LUMEN_HEVC_CORPUS` is explicitly set but the file is missing, or generate a small in-tree 0044-shaped fixture (the repo already has an in-tree generator philosophy — tests/fixtures/CORPUS.md CC0 lane) so the lane always runs.
Confidence: HIGH on the skip mechanism and corpus absence; MEDIUM on "skipped during the 2026-11-21 baseline run specifically" (the INDEX records no `LUMEN_HEVC_CORPUS`/`LUMEN_CORPUS` env in any command, and the directory is absent now — but I cannot prove it was absent at run time; the 0.01s hevc_parser_corpus duration is decisive for THAT binary, circumstantial for this one) — Verification: ran ls/find + read baseline log; UNVERIFIED: the exact baseline-time filesystem state.

### F-002 [S3]
Where: crates/codecs/src/video/hevc/parser/nal.rs:991-996 (helper, part2's range — cited because this chunk's only real-stream lane depends on it)
Code:
```
    pub(crate) fn corpus_path(name: &str) -> Option<std::path::PathBuf> {
        let dir = std::env::var("LUMEN_HEVC_CORPUS") // lumen-allow-literal: cfg(test) corpus-gate env-var name — test harness knob, not product config
            .unwrap_or_else(|_| "/tmp/my-project/corpus-v4/media".to_string()); // lumen-allow-literal: cfg(test) corpus default path (sandbox layout) — test-only, env-overridable
```
What is wrong: two different env-var names gate the same corpus: this file uses `LUMEN_HEVC_CORPUS` (default `/tmp/my-project/corpus-v4/media`), while `tests/hevc_parser_corpus.rs:36-38` uses `LUMEN_CORPUS` (default `/tmp/my-project/corpus-v4`, then joins `media`). Setting one does not set the other — a maintainer who points `LUMEN_CORPUS` at their corpus still gets silent skips from every in-module corpus lane (and vice versa).
Why it matters: the corpus-gate convention is already fragile (F-001); two names for one knob guarantees inconsistent local runs and makes the "skip-loud" contract unreliable — the exact mechanism that lets the baseline go green without the corpus.
What correct looks like: one shared `pub(crate) fn corpus_dir()` (name, default, env var) in the testkit or a single test-support module, consumed by both.
Confidence: HIGH — Verification: read (both sites sed-verified: nal.rs:992 and hevc_parser_corpus.rs:36-38).

### F-003 [S4]
Where: crates/codecs/src/video/hevc/parser/nal.rs:1036 (and 1043, 1131-1134, 1207, 1196, 1132)
Code:
```
        let (h, payload) =
            NalHeader::split(&[0x40, 0x01, 0xaa]).unwrap_or_else(|e| unreachable!("{e}"));
```
What is wrong: pervasive `unwrap_or_else(|e| unreachable!(…))` / `.unwrap_or_default()` in assertion-prelude positions. `.unwrap_or_default()` at 1092, 1099, 1102, 1122 is the dangerous one: if `unescape_rbsp` ever returns Err on a case the test believes is valid, the assert compares against the empty-vec DEFAULT instead of failing loudly — a wrong-parse regression could pass if the expected side were also degenerate (it is not here — the expected vectors are non-empty literals, so the assert would fail on len mismatch; the pattern is still a landmine for future edits).
Why it matters: test-side error swallowing weakens the mutation-killing property the rest of the suite earns; `unwrap_or_default` turns a parse failure into a misleading assertion message instead of "the parser refused".
What correct looks like: `.expect("valid input")` (tests may panic) everywhere the input is believed valid; reserve `unwrap_or_default` for nothing.
Confidence: HIGH — Verification: read (all `unwrap_or_default` sites sed-checked; current expected vectors are non-empty literals so the len assert still trips — defect is the pattern, not a live false-pass).

### F-004 [S4]
Where: crates/codecs/src/video/hevc/parser/nal.rs:934-939 (harness `Bits::ue`, part2's range — cited for this chunk's tests that depend on it)
Code:
```
        pub(crate) fn ue(&mut self, v: u64) {
            let v1 = v + 1;
            let width = 64 - v1.leading_zeros();
            self.put(width - 1, 0);
            self.put(width, v1);
        }
```
What is wrong: `v == u64::MAX` overflows `v + 1` to 0 → `leading_zeros() == 64` → `width == 0` → `width - 1` underflows (u32 panic in debug, wrap in release) and `put(0, …)` writes nothing — the helper silently mis-encodes the maximal value instead of rejecting it.
Why it matters: test-only, and no current caller passes ≥ 2^63; but the harness is `pub(crate)` "shared by every sibling test module" (:897-898), so the landmine is one future `ue(huge)` away.
What correct looks like: `debug_assert!(v < u64::MAX - 1)` or an explicit panic on out-of-model values (the `put` self-check at :915-918 already shows the right instinct).
Confidence: HIGH — Verification: read (arithmetic traced; `put`'s own assert does not catch it because put(0,0) is legal).

### F-005 [S4]
Where: crates/codecs/src/video/hevc/parser/nal.rs:1373-1375
Code:
```
                let _ = crate::video::hevc::parser::vps::VideoParameterSet::parse(&nal);
                let _ = crate::video::hevc::parser::sps::SeqParameterSet::parse(&nal);
                let _ = crate::video::hevc::parser::pps::PicParameterSet::parse(&nal);
```
What is wrong: the fuzz-lite drives every random buffer through ALL THREE parameter-set parsers regardless of the framed head (the `parse_vps/parse_sps/parse_pps` bools at :1366-1369 are computed and then only gate the extra `unescape_rbsp` call at :1376-1379 — the flags' names promise selectivity the code does not implement).
Why it matters: the dead booleans mislead a reader into thinking each head is selectively exercised; functionally harmless (over-coverage, not under-coverage) but it is exactly the "names promising more than the body" checklist item, in miniature.
What correct looks like: delete the booleans and always run the unescape, or actually gate the parse calls on them.
Confidence: HIGH — Verification: read (flag construction :1366-1370 vs use :1373-1379).

## 4. Per-function table (chunk lines 1008–1517)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `nal_classification_roundtrips_table_7_1` | 1008-1031 | REAL(test) | — | exhaustive 0..=63 roundtrip + Table 7-1 spot checks (IRAP 16..=23, VCL 0..=21) — mutation-killing, hand-verified vs spec |
| `header_split_decodes_two_byte_geometry` | 1034-1065 | REAL(test) | S4 | geometry assertions hand-verified (0x43,0x0f → type 33, layer 33, tid+1 7); truncation/forbidden/temporal-id negatives; F-003 unreachable pattern |
| `annexb_splits_three_and_four_byte_prefixes` | 1068-1080 | REAL(test) | — | garbage prefix, 3- and 4-byte codes, trailing-zero trim, no-start-code → None |
| `emulation_removal_handles_overlapping_escapes` | 1083-1125 | REAL(test) | — | THE chained-escape case on real bytes (14→11 hand-verified) + simple forms + typed bad-suffix/dangling + whole-VPS roundtrip with level_idc survival |
| `length_prefixed_split_and_typed_failures` | 1128-1175 | REAL(test) | — | padding end, short tail geometry, zero-length, overrun, bad length_size — five distinct negatives |
| `hvcc_0100` (helper) | 1182-1202 | REAL(test) | — | faithful 23-byte header + three arrays with per-array numNalus (the doc'd fixed builder bug) |
| `hvcc_parse_field_exact_on_corpus_arrays` | 1205-1233 | REAL(test) | — | 20+ byte-exact fields incl. length_size 4 and array payloads; hand-checked vs 14496-15 layout |
| `hvcc_typed_error_paths` | 1236-1290 | REAL(test) | — | six negatives: 23-byte truncation geometry, version, reserved bits, missing array, zero-length NAL, overrun |
| `bit_reader_roundtrip_and_bounds` | 1293-1348 | REAL(test) | — | write/read roundtrip, width 0/33 rejections, ue prefix cap, rbsp tail violations, past-end reads typed |
| `fuzz_lite_no_panics_on_random_and_mutated_bytes` | 1351-1432 | REAL(test) | S4 | real panic-freedom fuzz (10k random × 3 heads + 5k mutations × 4 surfaces); F-005 dead gating booleans; asserts nothing beyond not-panicking (its stated purpose) |
| `corpus_0044_parameter_sets_and_slice_prefixes_parse` | 1440-1516 | SHALLOW-as-executed | S2 | F-001: silently skipped when corpus absent (it is, here); the test BODY is strong (id lattice VPS←SPS←PPS, PTL agreement, 64-slice prefix walk with poc-mask bounds, IRAP presence) — but none of it runs without corpus-v4 |
| (`Bits`/`corpus_path`/`splitmix64` harness) | 893-1005 | (part2's) | S4 | referenced: F-002 dual env names, F-004 ue overflow |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all except corpus_0044 | YES | byte-exact expected vectors, exhaustive enums, distinct typed-negative shapes — every mutation class I reasoned through (wrong bit offsets, wrong escape handling, wrong reserved-bit masks, unbounded loops→panics) trips at least one |
| `corpus_0044_parameter_sets_and_slice_prefixes_parse` | NOT IN THIS ENVIRONMENT | the file is absent → zero assertions execute → green (F-001); in a corpus-equipped environment: YES, the lattice/PTL/poc-mask asserts are strong |
| `fuzz_lite…` | YES (panic class only) | harness fails on panic; wrong-but-quiet parses survive — that is the stated scope |
Baseline citation: test-workspace.log.txt:632-641 (all nal::tests ok), :781 (fuzz_lite ok), :1088-1117 (hevc corpus binaries, 0.01s durations — the skip signature for hevc_parser_corpus).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Corpus differential (live ffprobe 7.1.5): 3/3 lanes PASS … 0044 … PASS" | docs/formats/video-hevc.md:182-190 | REAL-UNPROVEN (unreplicable from baseline) | the lanes pass only where corpus-v4 + ffprobe exist; both absent/undocumented in the baseline environment (F-001); the claim's provenance is the dev session, not the baseline |
| "cards 5.G1 parser 3/3 corpus lanes byte-verified" (evidence string) | docs/feature-matrix.toml:441 | REAL-UNPROVEN | same chain; the matrix cites a lane the shipped tree cannot run without an external, unshipped corpus |
| "(+FB) 10k splitmix64 random buffers plus 5k mutations … never panic" | nal.rs:1352-1356 (test doc) | REAL | test exists and passes in baseline (:781); scope is panic-freedom only |
| wave-6 hostile question | task prompt | N/A | hevc parser is not on any shipped path; H.264 answer lives in sw.rs.audit.md |

## 7. Dead / duplicated / unreachable code
- The whole `hevc` tree (parser + hw seam + decode wrapper, this chunk included) is production-dead: the only Decoder impl (`HevcDecoder`) is constructed solely by tests/hevc_hw_path.rs; the hw seam (hevc/hw.rs:63-67) imports the parser types (`super::parser::{nal, pps, slice, sps, vps}`) but no shipped path reaches either. CORRECTION vs my first draft of this row: an initial `rg -ln "hevc::parser"` sweep missed the `super::parser` import form and wrongly claimed hw.rs does not import parser::nal — it does (hw.rs:63); the unreachability conclusion is unchanged and now rests on the zero-caller proof for `HevcDecoder`.
- F-005's `parse_vps`/`parse_sps`/`parse_pps` booleans: computed, barely used — dead-in-effect.
- No duplication within the chunk (the `Bits` harness is deliberately shared pub(crate)).

## 8. Dependencies & flags
- Test module: std only (`std::env`, `std::fs`, `std::path`); no external deps; no unsafe; two `lumen-allow-literal` lint waivers at :992-993 (test-only corpus path/env — legitimate waivers, cited inline). No feature gate. Lint-clean per baseline clippy.

## 9. Open questions for the next agent
- W06-A04 (part2, owns `corpus_path` + `Bits` harness): F-002 and F-004 live in your range — adopt or refute from your side.
- Matrix-rows agent for the HEVC parser row (feature-matrix.toml:441): the evidence string's "3/3 corpus lanes byte-verified" needs an environment caveat or an in-tree fixture (F-001).
- REQUEST-ORCH-RUN (optional, would upgrade F-001's MEDIUM to HIGH/LOW): `LUMEN_HEVC_CORPUS=/nonexistent cargo test -p codecs --release hevc::parser::nal::tests::corpus_0044 -- --nocapture` in a corpus-equipped environment to observe the SKIP path vs a real run's duration.
