# AUDIT: tools/xtask/src/cmd/fixtures.rs
| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-23 |
| Lines / functions | 5341 / 94 production fns+methods (+44 tests, +13 test helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev tool by design; reachable via `cargo xtask fixtures` (dispatch: main.rs:112-131 -> cmd::lookup -> `fixtures::run`, cmd/mod.rs:123-127). Executed by auditor. Its committed artifacts feed test binaries + rc-smoke (baseline: headless-decode AVI/MJPEG/PCM PASS). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | `./target/debug/xtask fixtures --check` (exit 0); fresh build `CARGO_TARGET_DIR=audit/... cargo test -p xtask --test cli --no-run` then `--check` (exit 0) and `--dest` regen (55 files, flip1 sha == committed); drift/missing/usage exit codes 1/1/2 (executed on scratch copy under audit/); `ffmpeg`/`ffprobe` oracle re-runs (4/4 digests match pins; ffprobe 3-stream MT parent); `sha256sum` fixtures vs MANIFEST (match); python3 verification of SIN_QUARTER/DCT_A/MD5/fdct tables, xorshift64*+Fisher–Yates model (matches committed garbage/flip bytes); `bash scripts/oracle_check.sh` (exit 1 — see F-002). |

## 1. One-paragraph verdict
This is a genuine, dependency-free (std-only, tools/xtask/Cargo.toml) deterministic corpus generator: it hand-encodes baseline JFIF JPEG (fixed-point DCT, per-image Huffman), writes AVI 1.0 and MP4 (ISO 14496-12/-14) parents plus 22 malformed derivatives, and regenerates the committed `tests/fixtures` tree byte-for-byte — verified end-to-end: `--check` exits 0 with two independently built binaries (prebuilt target/debug/xtask and a fresh `CARGO_TARGET_DIR` build from current source), and the two first-vertical parents decode under ffmpeg 7.1.5 to exactly the externally pinned oracle digests. The generated fixtures are NOT cited as evidence anywhere in docs/feature-matrix.toml (0 matches) — no circular fixture-as-matrix-evidence; they instead back real consumers (codecs fixtures_parent.rs, playback-engine scenario_09/soak, media-probe fixtures_matrix, fuzz_survival, track_switch). The single hard-coded constant (FIRST_FRAME_HASH_Y) is honestly labeled "RECORDED VALUE" and its loop is closed decoder-side by a test that passes in the baseline. Defects are governance-level: the ledger/CORPUS.md this file emits claims to be the licensing ledger of a tree it only partially governs, and the external-oracle perimeter is currently red.

## 2. What actually works (verified)
- Determinism + drift gate: `./target/debug/xtask fixtures --check` -> `result: no drift — 55 file(s) verified` (exit 0); re-verified with a binary freshly compiled from current source into an isolated target dir (audit/runs/W01-A01.target) -> identical result; `--dest` regeneration under audit/ produced flip1 sha `4e3207cc…` == committed file.
- Exit-code contract: clean `--check` -> 0; corrupted/missing file in scratch tree -> `drift:`/`missing:` messages and TRUE_EXIT=1; `--badflag` -> exit 2 (matches frozen contract, main.rs:100-108).
- External oracle (the anti-circularity pin, W1-14): re-ran the exact pinned commands — `ffmpeg -i tests/fixtures/generated/valid/avi_mjpeg_pcm.avi -map 0:v -pix_fmt yuv420p -f rawvideo - | sha256sum` -> `00c4f875fa15b765a2d1ffb5eaedf5c1b7647e3b3dbeeb995d9020f0ec78d9c4` (== pin), audio -> `1d9f5019c9b5181f0910528dce360f46e74f9c4df8d9bc9a20c2c68fd218b95b` (== pin); MP4 twin identical 2/2. ffmpeg 7.1.5 = the pinned version.
- Multi-track parent externally probed: `ffprobe avi_mjpeg_2pcm.avi` -> mjpeg 320x240 10/1, pcm_s16le 48000 stereo, pcm_u8 44100 mono — exactly the generated `[tracks]`/`[switch_plan]` claim.
- Math verified against independent python3 references: SIN_QUARTER == round(sin(i·π/128)·32767) (65/65), DCT_A == Q16 basis (64/64), MD5 RFC-1321 + 55/56/64-byte edge vectors == hashlib, gradient-block fdct reference == float64 recompute. In-crate SHA-256 == coreutils sha256sum on committed parents (MANIFEST digests match).
- PRNG/derivatives reproduced by an independent Rust model (byte-copy of the algorithm executed under audit/): AVI flip1 = bit 470 (byte 58, `avih` dwStreams -> 16386 streams), flip4 = {57,148,470,504} (RIFF size lie + hdrl size lie), MP4 flip1 = bit 1230 (byte 153, video `trak` size 0x5c -> 0x40005c); garbage block first 16 bytes match committed file 16/16. All seeded flips land in semantic header fields (confirmed by consumer comments in crates/media-probe/tests/fixtures_matrix.rs:340,441).
- Unit tests: all 44 fixtures tests pass in the orchestrator baseline (audit/runs/baseline/cargo-test-workspace.txt:6568 "running 176 tests … 176 passed", incl. `full_tree_is_byte_stable_across_two_runs`, `bitflip_derivatives_flip_exactly_k_header_bits`); codecs-side loop `first_frame_descriptor_matches_the_expectation` passes (same log, line 1053).

## 3. Findings
F-001 [S3]
Where: tools/xtask/src/cmd/fixtures.rs:129-131, 3254-3305, 3596-3635
Code:
```
  (no flags)    regenerate every fixture under tests/fixtures/ and print
                the report (committed artifacts: refresh + commit on
                generator change, same workflow as schema-gen/codegen)
```
and
```
    s.push_str("- Curated third-party files (class D): none in v0 — any future row\n");
    s.push_str("  needs source URL + license + hash per corpus-plan §5.\n\n");
```
What is wrong: The emitted CORPUS.md titles itself "Test-corpus licensing ledger (tests/fixtures)" and asserts no class-D (curated third-party) files exist, but the tree it governs contains ~84 additional files this generator does not produce or list: 27 extra valid media fixtures (mkv_h264_aac.mkv, mp4_h264_aac.mp4, mp4_qt.mov, webm_vp9_opus.webm, wav_*, ogg_*, aiff/aifc, *.3gp, mp4_chapters/edts/frag/frag_sidx) and ~59 extra malformed derivatives using this file's `<stem>__<op>` naming. Verified: `ls tests/fixtures/generated/valid | wc -l` = 33 (generator owns 6), malformed = 81 (owns 22); `rg` finds no generator for those names in tools/ or scripts/. `verify_tree` (L3596-3635) only byte-compares the 55 owned files — a modified or swapped non-generator media fixture is invisible to `--check`, and the usage text "regenerate every fixture under tests/fixtures/" overstates the gate's scope by ~3x. Why it matters: the ledger is the audit artifact for license/provenance claims; a ledger that silently omits 3/4 of the tree (including real third-party-looking media) makes the CC0/clean-room claim unauditable for those files. What correct looks like: CORPUS.md should enumerate or explicitly scope-out every file under tests/fixtures, and `--check` should at minimum fail on unlisted media files in its perimeter (the way scripts/oracle_check.sh does). Confidence: HIGH — Verification: ran (ls counts, --check output, rg searches).

F-002 [S3]
Where: tools/xtask/src/cmd/fixtures.rs:3407-3437 (mt parent generation) + tests/fixtures/generated/valid/ (sidecar state)
Code:
```
    let mt_frames: &[Vec<u8>] = &frames[..MT_FRAME_COUNT];
    let avi_mt = build_avi(mt_frames, MT_FPS, &mt_audio)?;
```
What is wrong: The W2-FIX-MT multi-track parent `avi_mjpeg_2pcm.avi` — generated by this file — has no `.oracle.sha256` external-oracle pin (only avi_mjpeg_pcm.avi and mp4_mjpeg_pcm.mp4 do), and no wired ffmpeg cross-check (CORPUS.md records oracle commands "for provenance only"). Executed: `bash scripts/oracle_check.sh` exits 1 today — perimeter FAIL on 15 unpinned media files plus 6 unpinned .avi/.mp4 fixtures (mp4_h264_aac, mp4_chapters, mp4_edts, mp4_frag, mp4_frag_sidx, avi_mjpeg_2pcm). The writer↔decoder hash loop for the MT parent is closed only in-repo (track_switch tests parse `[switch_plan]`), so the "escape the echo chamber" cure (W1-14) does not cover this file's newest parent. Why it matters: the exact defect class (self-referential pins) that W1-14 was created to cure is re-entering through growth — this generator added a parent without extending the pin set. What correct looks like: every media fixture this generator emits gets a sidecar in the same commit (the sidecar header itself demands this: "Extend this set in the same commit as any fixture stream change"). Confidence: HIGH — Verification: ran (oracle_check.sh exit 1; ffprobe structure verified manually as partial mitigation). Note: the 15 non-generator files breaching the perimeter are primarily another owner's defect (see F-001); this finding scores the fixtures.rs-owned asymmetry.

F-003 [S4]
Where: tools/xtask/src/cmd/fixtures.rs:29-32 vs 656-665
Code:
```
//!   RGB→YCbCr, a 256-point LUT sine NCO with phase accumulator. No
//!   float op executes anywhere in the generation path, so no platform
//!   libm can drift a byte.
```
vs
```
        probe.extend_from_slice(&(f32::from(sample) / 32_768.0).to_le_bytes());
```
What is wrong: The determinism contract's claim "No float op executes anywhere in the generation path" is false as written: `audio_probe_md5_s16` executes f32 conversion and division while generating the expectation files (part of the generated tree). The conclusion still holds — i16→f32 is exact, division by 2^15 is exact, so the digest is bit-deterministic, and the conversion is byte-pinned by `audio_probe_conversion_pins_the_f32_serialization` — but the stated invariant is inaccurate. Why it matters: the module's whole pitch is a strictly-checkable determinism contract; an inspector trusting the comment would not look for float paths. What correct looks like: scope the claim to the media bytes, and state the exactness argument for the probe path (the function's own doc already does — the module header does not). Confidence: HIGH — Verification: read + ran (the pinning test passes in baseline).

F-004 [S4]
Where: tools/xtask/src/cmd/fixtures.rs:1293-1300
Code:
```
    let len = u16::try_from(payload.len() + 2)
        .unwrap_or(u16::MAX)
        .to_be_bytes();
```
What is wrong: If a marker segment's payload ever exceeded 65533 bytes the writer silently emits a wrong length field (u16::MAX) followed by the full payload — a structurally corrupt JPEG instead of an error. Unreachable with the fixed corpus (largest segment: DQT = 130+2 bytes) but this is the "silent corruption instead of typed error" pattern in a codebase whose error contract is otherwise strict (`Result<_, String>` everywhere). What correct looks like: return the Err like every sibling function does. Confidence: HIGH — Verification: read (unreachable branch; no corpus input reaches it).

F-005 [S4]
Where: tools/xtask/src/cmd/fixtures.rs:566-573
Code:
```
        step: u32::try_from(step).unwrap_or(u32::MAX),
```
What is wrong: For `freq >= rate` the NCO step saturates silently to u32::MAX (aliasing to ~0 Hz) instead of erroring. Corpus tones (440/660/880 Hz) are far below all rates, so unreachable today, but a future tone-plan edit would produce wrong audio with no error. Confidence: HIGH — Verification: read.

F-006 [S4]
Where: tools/xtask/src/cmd/fixtures.rs:3156-3163 (generated expectation content)
Code:
```
    s.push_str("# v0: the union contract — the specific flavor (typed-error |\n");
    s.push_str("# bounded-recovery | rejected) is pinned by demuxer conformance\n");
    s.push_str("expected = \"typed-error-or-bounded-recovery\"\n");
```
What is wrong: The generated `[error]` contract cannot by itself distinguish correct from incorrect demuxer behavior for semantically-light damage (e.g. AVI flip1 lands on an advisory `avih` byte; a demuxer that opens it cleanly is neither typed-error nor obviously "bounded-recovery"). The expectation files delegate the real pinning to downstream test code — which does exist and is specific (crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs pins per-fixture Refused/Contained/FatalFirstRead; crates/media-probe/tests/fixtures_matrix.rs documents each flip site) — so this is a doc/precision debt in the generated artifact, not a test hole. Confidence: HIGH — Verification: read + ran (flip sites computed and matched the consumers' documented offsets).

## 4. Per-function table (production code; test fns in §5)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_args | 157-208 | REAL | — | dup-flag, missing-value, mutual-exclusion checks; executed (exit 2 path) |
| workspace_root | 211-216 | REAL | — | compile-time CARGO_MANIFEST_DIR anchor; same as sibling cmds |
| run | 219-243 | REAL | — | executed: write/check/list modes, exit codes 0/1 verified |
| Sha256::{new,update,finish,compress} | 274-382 | REAL | — | FIPS 180-4; NIST vectors test; == coreutils on parents (executed) |
| sha256_hex | 385-389 | REAL | — | used by manifest/expectations/CORPUS |
| md5_hex | 398-467 | REAL | — | RFC 1321; vectors + 55/56/64 edge == python hashlib (executed) |
| hex_lower | 470-478 | REAL | — | trivial, correct |
| Xorshift64Star::{from_digest,next_u64,below} | 486-519 | REAL | — | stream reproduced independently; garbage bytes match 16/16 (executed) |
| sine_full_table | 546-562 | REAL | — | table verified exact vs float reference (executed) |
| SineOsc::new | 566-573 | REAL | S4 | F-005 silent saturation |
| SineOsc::sample | 576-583 | REAL | — | bounded interp, index+1 ≤ 256 by table size 257 |
| generate_pcm | 625-637 | REAL | — | 88200 instants, integer NCO; first-sample-zero asserted+verified |
| audio_probe_md5_s16 | 656-665 | REAL | S4 | F-003 float-op in "generation path"; exact IEEE ops, pinned by test |
| audio_probe_md5 | 669-671 | REAL | — | thin wrapper, used by parents + tests |
| generate_mt_pcm_s16 / generate_mt_pcm_u8 | 708-734 | REAL | — | u8 path clamps to [0,255] (range [28,228] by construction) |
| RgbImage::{new,fill_rect,pixel} | 753-786 | REAL | — | pixel is #[cfg(test)] only (fine) |
| draw_digit / draw_frame | 822-861 | REAL | — | geometry pinned by test; out-of-range rejected |
| fdct_quantize | 928-957 | REAL | — | i64 headroom verified (max acc ~8.4e12); matches float ref (executed) |
| magnitude_category / amplitude_bits | 960-977 | REAL | — | spec C.2/F.1.2.1 |
| build_huffman_table | 1007-1135 | REAL | — | deterministic tiebreak; Annex-K.3 16-bit cap; Kraft/prefix-free tested incl. Fibonacci-depth case |
| BitWriter::{new,push,finish} | 1144-1176 | REAL | — | 0xFF stuffing + 1-padding; round-trip tested |
| rgb_to_ycc420 | 1183-1222 | REAL | — | JFIF integer math; cb/cr ranges land in [0,255] (no clamp needed, verified by arithmetic) |
| count_block_symbols / emit_block | 1226-1290 | REAL | — | pass 1/2 walk identical paths (ZRL/EOB handling consistent) |
| jpeg_segment | 1293-1300 | REAL | S4 | F-004 silent u16::MAX fallback |
| encode_baseline_jpeg | 1313-1446 | REAL | — | full JFIF stream; decoded by ffmpeg (oracle, executed) |
| put_u16le/put_u32le/put_u16be/put_u32be | 1454-1459, 1896-1901 | REAL | — | endianness correct (AVI LE / MP4 BE), byte-verified by structure tests + ffprobe |
| avi_chunk/avi_begin_sized/avi_end_sized | 1493-1518 | REAL | — | WORD-alignment pad not counted in size (test-verified) |
| AviAudioEssence::{frame_bytes,tick_bytes,total_frames} | 1541-1556 | REAL | — | pure arithmetic |
| build_avi | 1563-1786 | REAL | — | validates rate|fps, tick_frames, byte counts; idx1 offsets movi-relative (test + ffprobe verified) |
| json_put_string/json_put_uint/fourcc_str | 1796-1827 | REAL | — | defensive escaping |
| avi_structure_json | 1831-1888 | REAL | — | offsets re-walked by test and matched |
| mp4_box/mp4_begin_box/mp4_end_box/mp4_fullbox_body | 1931-1962 | REAL | — | size patching correct (tests walk boxes cleanly) |
| mp4_dinf | 1966-1975 | REAL | — | dref/url self-contained flag |
| mp4_jpeg_sample_entry | 1978-2001 | REAL | — | 78-byte VisualSampleEntry; ffmpeg decodes it (executed) |
| mp4_sowt_sample_entry | 2005-2022 | REAL | — | 28-byte; samplerate 16.16 fits u32 (44100<<16) |
| mp4_stbl | 2029-2090 | REAL | — | returns stsz-count + stco patch sites; validated counts |
| mp4_trak | 2102-2194 | REAL | — | tkhd/mdhd/hdlr/minf; zero creation times (determinism contract upheld) |
| build_mp4 | 2201-2352 | REAL | — | strict frame/PCM count checks; size-neutral stco patch pass; u32 offset overflow guarded |
| mp4_structure_json | 2355-2417 | REAL | — | recorded offsets verified against fresh box walk by tests |
| parent_prng / garbage_block / flip_header_bits | 2446-2493 | REAL | — | seed = sha256(parent)[:8] BE; Fisher–Yates k-prefix; reproduced independently (executed) |
| derivative_path | 2505-2512 | REAL | — | naming per corpus-plan; `trim_start_matches` idempotent for these paths |
| truncate_derivative | 2515-2541 | REAL | — | bounds-checked cut; recorded in [derivation] |
| flip_derivative | 2544-2569 | REAL | — | fresh parent-seeded rng; extent/k recorded |
| garbage_derivative | 2572-2591 | REAL | — | 4096 seeded bytes; non-constant asserted; executed match |
| avi_no_idx1_derivative | 2596-2630 | REAL | — | drops idx1, patches RIFF size; avih still claims HASINDEX (intentional spec violation, tested) |
| mp4_stsz_lie_derivative | 2634-2663 | REAL | — | patches video stsz count to 2^31-1; exactly 4 bytes differ (tested) |
| avi_neg_height_derivative | 2670-2701 | REAL | — | biHeight -> -240 (ED-FIX-3 V1); patch site found by structure walk, not hard-coded |
| avi_abs_idx1_derivative | 2708-2751 | REAL | — | rewrites dwOffset to absolute (ED-FIX-3 V4); every diff confined to dwOffset fields (tested) |
| avi_strl_chunk_payload_offset | 2756-2784 | REAL | — | structure walk with bounds checks (no infinite loop: pos strictly increases) |
| avi_derivatives / mp4_derivatives | 2791-2890 | REAL | — | cut sites all derived from layout; mid-chunk indexes bounds-checked |
| toml_string / toml_uint | 2901-2934 | REAL | — | underscore grouping matches doc 10 style |
| expectation_path / expectation_header | 2939-2969 | REAL | — | header includes re-derived sha256 each run |
| duration_us / frame_period_us | 2972-2979 | REAL | — | 2_000_000 / 33_333 asserted in tests and consistent with seek math |
| parent_expectation | 2984-3051 | REAL | — | see F-006 for [error] looseness (derivatives only); parents carry digests consumed by codecs tests |
| multitrack_expectation | 3058-3140 | REAL | — | [switch_plan] parsed (not transcribed) by track_switch_fixture.rs (verified) |
| derivative_expectation | 3144-3165 | REAL | S4 | F-006 |
| manifest_toml | 3170-3199 | REAL | — | 28 rows; no self-referential ledger rows (tested); hashes == coreutils (executed) |
| corpus_purpose | 3203-3252 | REAL | — | per-op prose; matches actual ops |
| corpus_markdown | 3258-3360 | REAL | S3 | F-001 incomplete ledger claim |
| generate_all | 3378-3539 | REAL | — | pure function; fixed order; shared video essence across parents (tested: both containers byte-identical frames) |
| print_file_table | 3542-3568 | REAL | — | --list output; counts media fixtures |
| write_tree | 3571-3592 | REAL | — | create_dir_all + fs::write with typed IO errors |
| verify_tree | 3596-3635 | REAL | S3 | F-001: no orphan/extra-file detection (by-design scope, but USAGE text overstates) |

## 5. Test honesty (44 #[test] in file; all pass in baseline `cargo test` — 176 passed / 0 failed for xtask)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| sha256_matches_nist_vectors / sha256_is_update_split_invariant | YES | public vectors incl. 1M-‘a’; chunked==one-shot (padding bug caught) |
| md5_matches_rfc1321_vectors | YES | RFC suite + 55/56/64 straddle edges (independently confirmed vs python hashlib) |
| audio_probe_conversion_pins_the_f32_serialization | YES | pins exact LE bytes 0x3f000000/0xbf000000, not just self-comparison |
| audio_probe_spans_multiple_chunks_and_is_stable | YES | prefix-stable + cut-sensitivity (assert_ne!) |
| prng_* / sine_* / pcm_payload_* | YES | determinism, seed-sensitivity, bounds, channel-plan asymmetry (max_l>max_r) |
| drawn_frame_matches_the_planned_geometry / frames_differ… | YES | probes exact pixels incl. an OFF segment and inter-digit background |
| fdct_of_constant_block_is_dc_only / fdct_matches_float_reference… | YES | closed-form DC values + independently-computed gradient reference (±1 tolerance justified) |
| huffman_* (3) | YES | Kraft==2^16 exact, prefix-free pairwise, Fibonacci depth>16 exercises the Annex-K.3 cap; mini-decoder round-trip |
| encoded_jpeg_has_well_formed_structure / every_fixture_frame_encodes | YES | marker walk + 0xFF stuffing invariant over all 60 frames |
| avi_parent_is_a_self_consistent_riff_avi / mp4_parent_boxes_walk_cleanly / mp4_sample_tables_point_at_the_essence | YES | full independent re-walk of headers/tables; stco→mdat bytes compared to re-encoded frames and PCM |
| *_derivatives_* (7) | YES | re-derive cuts/patches from structure maps and byte-compare derivatives to parent prefixes/masks; k-prefix nesting and subset checks |
| full_tree_is_byte_stable_across_two_runs | YES (for determinism) | two generate_all() calls compared per-file; NOTE: does not compare against disk — the disk comparison is `--check` (executed by auditor, exit 0) |
| tree_shape_is_the_planned_55_files | YES | exact 55/6/22/25 counts + expectation↔media bijection |
| manifest_records_every_fixture… / corpus_ledger_declares_cc0… | YES (for consistency) | recomputes hashes from in-memory bytes and asserts presence — self-consistent by construction, but the hashes are independently confirmed == coreutils on disk (auditor-executed) |
| parent_expectations_match_the_doc10_v0_shape | PARTLY | asserts text contains values computed by the same functions (serializer consistency); the non-circular pins (FIRST_FRAME_HASH_Y vs decode, oracle sidecars) live in crates/codecs tests + oracle_check.sh, both verified separately |
| help/flags tests (3) | YES | arg parsing incl. all rejection paths |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "byte-stable across runs" (determinism contract) | module doc L24 | REAL | --check exit 0 (two binaries); two-run unit test in baseline |
| "No float op executes anywhere in the generation path" | L29-32 | FALSE (as written) | f32 ops at L662; exactness argument holds; see F-003 |
| "ffmpeg/ffprobe used only as local cross-check oracles … never in the build" | L48-49 | REAL | no ffmpeg invocation in xtask/Cargo/tests; oracle pins live as committed sidecars + opt-in script (executed) |
| FIRST_FRAME_HASH_Y "recorded from Lumen's first compliant decode … verified by crates/codecs/tests/fixtures_parent.rs" | L93-106 | REAL | test exists (fixtures_parent.rs:299) and passes (baseline line 1053); cross-checked against ffmpeg oracle digest (video hash pinned from ffmpeg's own decode — M15 cure confirmed live) |
| "Every byte is authored by this in-tree generator from public specifications … no VLC code or assets" (clean-room) | L41-47 | REAL-UNPROVEN | all tables verified as public math (DCT/JPEG Annex K/RFC 1321/FIPS 180-4); authoring-process provenance (no copied assets) is not machine-checkable from this repo — UNVERIFIED beyond the math tables |
| "[first_frame] stamp … t = 0" | L109-112 | REAL | FIRST_FRAME_STAMP_US=0; codecs test asserts Some(0) |
| 55-file layout (§ Layout produced) | L51-63 | REAL | tree_shape test + --check "55 file(s) verified" |
| "Curated third-party files (class D): none in v0" (in generated CORPUS.md) | L3304 | FALSE (as a statement about tests/fixtures today) | ~84 non-generator files in tree with no ledger row — see F-001 |
| Exit codes 0/1/2 | L69-74 | REAL | all three executed by auditor |
| Matrix evidence: (hostile-focus check) fixtures as evidence for feature-matrix rows | docs/feature-matrix.toml | N/A — NOT USED | `rg tests/fixtures docs/feature-matrix.toml` = 0 matches; fixtures are not matrix evidence; matrix's own failures (parity 0/252) are unrelated to this file |

## 7. Dead / duplicated / unreachable code
- No dead production functions found (all reachable from run()/generate_all(); `RgbImage::pixel` is #[cfg(test)] by design).
- Duplication: `audio_probe_md5` vs `audio_probe_md5_s16` (deliberate named anchor, one-line wrapper); the count/emit two-pass JPEG walk duplicates the zigzag run logic by necessity (same path asserted by tests). `mt` PCM generators duplicate `generate_pcm`'s shape with different constants — acceptable, constant-driven.
- Not unreachable, but scoped: `--dest` exists for scratch/CI regeneration (used by this audit for isolation).

## 8. Dependencies & flags
- tools/xtask/Cargo.toml: std-only, zero dependencies ("C5"), publish=false, edition 2024, workspace lints — verified; no feature flags anywhere in the crate (`declared_features: []` in the build fingerprint), so nothing here is feature-gated out of any lane.
- Not part of the release lane (`cargo build --release -p lumen-cli -p desktop` baseline) — dev tooling only; committed artifacts are consumed by dev-dependency test trees.

## 9. Open questions for the next agent
1. Who generates/owns the ~84 non-generator files under tests/fixtures (mkv/wav/ogg/webm/aiff/mov/h264-aac families and their `__op` derivatives)? No generator found in tools/ or scripts/ — provenance and licensing are unledgered (F-001). scripts/oracle_check.sh exits 1 on them (perimeter) — that script's audit owner should record the red state.
2. The `.oracle.sha256` sidecars are committed next to the fixtures but are neither generated nor drift-gated by this command — an orphan class in the tree (harmless today, hashes verified live).
3. Clean-room provenance of the generator's authoring process (no VLC/copyrighted bytes) is asserted in doc comments and CORPUS.md but only the math tables are independently verifiable; full verification needs the worklog/history trail (UNVERIFIED here).
4. corpus-plan §2's "flip-k positions are a k-prefix" property is relied on by media-probe fixtures_matrix.rs — confirmed holding for the committed corpus (computed), but nothing in the expectation files records the property (only the note string).
