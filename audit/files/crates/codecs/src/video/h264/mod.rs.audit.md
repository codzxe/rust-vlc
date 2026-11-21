# AUDIT: crates/codecs/src/video/h264/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2026-11-21 |
| Lines / functions | 61 / 8 declaration/config items (5 `pub mod`, 1 feature-gated `pub mod`, 3 re-exports incl. 1 gated; no functions — pure module root) |
| File verdict | REAL |
| Reachable from shipped entry point? | MIXED BY CHILD — the file itself is the module root of the shipped H.264 lane. YES: `pub use sw::H264SwDecoder` (:60-61) → apps/cli/src/pipeline_host.rs:65,346 → EXECUTED in baseline (`mp4-smoke-open-session-420.txt`: h264 CB 4:2:0 MP4 plays end-to-end; `h264_sw_differential` 17/17 ok); `parser::nal` (via sw.rs:92-94 bridge) → same execution. NO: `pub use decode::H264Decoder` + `pub use hw::{H264HwBridge, H264HwDecoder, H264HwPath}` (:58-59) → zero production callers (W06-A07's hw/mod.rs audit: only test callers of `from_setting`); `parser::slice` → hw-tree types + tests only (my slice.rs#part2 audit); `fuzz` → test/fuzz-shim only (baseline :548-549 smoke tests ok). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | full-file read; `rg -n "cfg.*codec-h264-sw"` over codecs/src (exactly 2 sites — this file :55,:60; the gate is coherent); `rg -n "sw::\|H264SwDecoder"` over hw/mod.rs, decode.rs, fuzz.rs, parser/mod.rs (comment-only references — the feature-OFF build has no code-level sw references outside the gated module); `rg -rn "no-default-features\|codec-h264-sw"` over docs/web, scripts/, .github/ (no CI lane builds the OFF configuration; scripts/quality/09_features.sh exists as the cargo-hack verifier but no baseline row or workflow invokes it — UNVERIFIED); `rg -n 'id = "codec-video-h264-software"' -A 10` docs/feature-matrix.toml (:419-433: implemented on win/linux/mac — the mod doc's "blocked era closed" claim matches); baseline rows cited for the sw lane's execution. |

## 1. One-paragraph verdict
This 61-line module root is where the wave-6 hostile question is answered in code: `pub mod sw` behind `#[cfg(feature = "codec-h264-sw")]` (:55-56) — the feature that crates/codecs/Cargo.toml:53 lists in `default` and every release lane builds (`cargo build --release --locked -p lumen-cli -p desktop`, release.yml:193/474/627/763/911, no feature subset) — so the software H.264 decoder is compiled into and reachable from the shipped `lumen` CLI, proven by the corrected baseline smoke (4:2:0 MP4 plays) and the 17/17 ffmpeg-oracle differential. The module doc's claims check out against the tree and matrix: two decode paths exactly as described (hw typed-refusal without accelerator; sw default-on), the matrix row's `blocked` era genuinely closed (row status `implemented`, :427). Three findings, none blocking the lane: the feature-OFF build of this module is verified by NO baseline run and NO evidenced CI lane (only an unrouted local cargo-hack script), so the gate's compile-coherence rests on my grep, not on a run; the doc's "the fuzz_codec_h264 target's future guarantee" (:45) undersells what already runs (the target shim and its in-module smoke tests exist and pass in baseline); and the matrix row's evidence string still cites the phantom directory `h264/sw/**` — the standing feature-audit violation (baseline row 8, exit 1) that the INDEX CORRECTION v2 already traced to sw.rs, cross-file territory but this module's direct neighborhood.

## 2. What actually works (verified)
- The feature gate is coherent and minimal: exactly two `cfg` sites in the whole crate (mod.rs:55 module, :60 re-export); no code-level `sw::` reference outside the gated module (hw/mod.rs:476,:585 and decode.rs are comment-only) — verified by rg, so the OFF configuration is at least grep-clean.
- The shipped lane, end to end: `lumen open sample-420.mp4` → pipeline_host.rs:346 → `H264SwDecoder` (this module's re-export) → plays (mp4-smoke-open-session-420.txt, INDEX.md CORRECTION v2); `tests/h264_sw_differential.rs` 17/17 ok (test-workspace.log.txt:1063-1087).
- Doc-claim verification against the matrix: `codec-video-h264-software` row status `{windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started"}` (feature-matrix.toml:427) — the mod doc's "The `blocked` era … is closed" (:22-23) matches; the hw row's notes (:416) carry the matching 2026-11-15 closure note.
- The two-path honesty architecture described at :8-23 is exactly what the tree contains (hw/mod.rs's typed `h264.hw.required` refusal path host-tested in baseline: h264_hw_path suite ok, :1029-1046 area).

## 3. Findings

### F-001 [S3]
Where: docs/feature-matrix.toml:428 (cross-file — the evidence string of the row this module's doc closes)
Code:
```
evidence = "crates/codecs/src/video/h264/sw/** (codec-h264-sw, rust_h264 =0.6.1 Lead-admitted 2026-11-15; admission record docs/dependencies/records/rust_h264-0.6.1.md; differential crates/codecs/tests/h264_sw_differential.rs: 6 libx264 lanes x 2 entry forms (Annex-B ES + MP4/AVCC) bit-exact vs ffmpeg 7.1.5 whole-dump MD5 + hostile battery); engine e2e apps/cli mp4_h264_family_lane (mp4_h264_aac.mp4 plays to Ended, exit 0); Linux-evidenced"
```
What is wrong: the evidence cites `crates/codecs/src/video/h264/sw/**` — a DIRECTORY that does not exist; the decoder is the single file `h264/sw.rs`. This is the exact evidence-string defect that fails `cargo xtask feature-audit` (baseline row 8: exit 1, "row 23 evidence cites missing crates/codecs/src/video/h264/sw") and that INDEX.md CORRECTION v2 already diagnosed ("the path points at a directory; the code is in sw.rs"). The row's STATUS (implemented) is correct and run-proven; the STRING still trips the repo's own gate.
Why it matters: the project's feature-audit gate has been failing on this string since the baseline; every gate run pays for it, and the string teaches tooling consumers a path that 404s. It is the last stale artifact of the wave-0 "no sw decoder" confusion.
What correct looks like: `evidence = "crates/codecs/src/video/h264/sw.rs (…)"` — one character-class change; then baseline row 8's row-23 violation clears.
Confidence: HIGH — Verification: read (matrix row sed-verified; `ls crates/codecs/src/video/h264/` shows sw.rs, no sw/ directory; baseline xtask-feature-audit.log.txt row 23 cited via INDEX.md).
(NOTE: the row belongs to the matrix-rows agent's assignment — flagged here because this module's doc :15-23 is the code-side twin of the claim; count it once, from whichever file lands first.)

### F-002 [S4]
Where: crates/codecs/src/video/h264/mod.rs:55-56 and :60-61
Code:
```
#[cfg(feature = "codec-h264-sw")]
pub mod sw;
```
What is wrong: nothing IN the file — but the feature-OFF configuration (`--no-default-features` or subsets without `codec-h264-sw`, the documented wasm tree-shake consumer per codecs/Cargo.toml:23-25) is built by NO baseline run (rows 1-26 are defaults or --all-features) and by no workflow I can find (`rg` over .github/ for the verifier script: no hit). The intended verifier, `scripts/quality/09_features.sh` (cargo hack --each-feature / --feature-powerset / --no-default-features), exists but nothing in evidence invokes it.
Why it matters: a compile break in the OFF path (e.g., a future unconditional `sw::` reference) would be invisible to every gate that runs, while the Cargo.toml comment advertises subsets as supported measurement builds.
What correct looks like: route 09_features.sh into a CI lane (or one baseline row), or drop the subset-support claim from the Cargo.toml comment.
Confidence: HIGH on the absence of evidence (baseline INDEX read; workflow grep); UNVERIFIED whether audit.yml's tier scripts include it (the `scripts/quality/` runner tier structure is outside this file's scope — flagged for the scripts/CI agent).
Confidence: HIGH — Verification: read + rg sweeps (see Commands run).

### F-003 [S4]
Where: crates/codecs/src/video/h264/mod.rs:43-46
Code:
```
//! Every entry point is total: any byte string yields either a parsed
//! structure or a typed [`parser::H264Error`] — no panics, no
//! unbounded work (the `fuzz_codec_h264` target's future guarantee,
//! doc 04 §8.5).
```
What is wrong: "the fuzz_codec_h264 target's future guarantee" undersells the present: the target shim exists (fuzz/fuzz_targets/fuzz_codec_h264.rs, registry-verified by baseline row 18) and its in-module smoke tests run and pass in baseline (`video::h264::fuzz::tests::empty_and_marker_inputs_survive`, `hostile_wide_inputs_survive` — test-workspace.log.txt:548-549). What is actually "future" is only the libFuzzer campaign (no nightly wiring, same as the mjpeg lane).
Why it matters: mild doc drift in the conservative direction (claims less than is true); a reader assessing fuzz coverage for the h264 parser would miss that smoke-level fuzzing already runs in the default test suite.
What correct looks like: "— no panics, no unbounded work (smoke-tested by `fuzz`'s in-module tests; the libFuzzer campaign's guarantee is pending CI wiring)".
Confidence: HIGH — Verification: read + baseline log lines :548-549.

## 4. Per-function table (declaration/config items — no functions in this file)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc (all sections) | 1-49 | REAL | S4 | claims verified against tree+matrix (see §6); F-003 "future guarantee" undersell; the 2026-11-15 software-track update is present and accurate |
| `pub mod decode;` | 51 | REAL | — | hw-only `Decoder` adapter; unreachable from shipped paths (W06-A07 chain) |
| `pub mod fuzz;` | 51-52 | REAL | — | smoke tests run in baseline (:548-549) |
| `pub mod hw;` | 53 | REAL | — | adapter seam; unreachable (vaapi/mod.rs.audit.md F-001) |
| `pub mod parser;` | 54 | REAL | — | `parser::nal` reachable via sw bridge; `parser::slice`/`sps`/`pps` surfaces hw+test only |
| `#[cfg(feature = "codec-h264-sw")] pub mod sw;` | 55-56 | REAL | S4 | THE hostile-question gate; feature in `default` (Cargo.toml:53) → compiled in every release lane; F-002 OFF-path unverified by any run |
| `pub use decode::H264Decoder;` | 58 | REAL | — | unreachable export (test-only consumers) |
| `pub use hw::{H264HwBridge, H264HwDecoder, H264HwPath};` | 59 | REAL | — | unreachable exports |
| `#[cfg(feature = "codec-h264-sw")] pub use sw::H264SwDecoder;` | 60-61 | REAL | — | the reachable one — consumed by apps/cli/pipeline_host.rs:65,346; executed in baseline smoke + differential |

## 5. Test honesty
No tests in this file. Its children's test status (baseline-verified): h264_sw_differential 17/17 (:1063-1087), h264_parser_corpus 2/2 (:1055-1061), h264_hw_path suite ok, h264::fuzz::tests 2/2 (:548-549), h264 parser unit suites ok. The module root adds no test surface of its own — appropriate.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "sw::H264SwDecoder — the software path (codec-h264-sw, default-on)" | mod.rs:15-16 | REAL | Cargo.toml:53 default list; release.yml:193 et al. build with defaults; smoke-420 plays |
| "The `blocked` era of the `codec-video-h264-software` matrix row is closed" | mod.rs:22-23 | REAL | feature-matrix.toml:427 status implemented (win/linux/mac) |
| matrix evidence "crates/codecs/src/video/h264/sw/**" | feature-matrix.toml:428 | FALSE (path shape) | `sw/` directory does not exist — the code is `sw.rs`; the standing feature-audit row-23 violation (F-001) |
| "Every entry point is total … no panics, no unbounded work" | mod.rs:43-46 | REAL-UNPROVEN at fuzz scale | smoke tests pass (:548-549) + hostile battery in the differential suite; the libFuzzer campaign does not run (F-003's precision) |
| "With no accelerator negotiated it fails configure with the typed `h264.hw.required`" | mod.rs:12-14 | REAL | h264_hw_path integration tests pass in baseline (typed-refusal lanes) |
| WAVE-6 HOSTILE QUESTION — "is H.264 decode reachable from the shipped lumen CLI / lumen-tauri build?" | task prompt | **YES (CLI: executed; tauri: graph-proven)** | codec-h264-sw ∈ default (codecs/Cargo.toml:53) → apps/cli/Cargo.toml:51 defaults → release.yml:193/474/627/763/911 `-p lumen-cli -p desktop` (no subset); tauri closure: apps/desktop-tauri/Cargo.lock:397-406 (codecs deps incl. rust_h264) + :2888; EXECUTED: mp4-smoke-open-session-420.txt (Buffering → Playing → Stopped → Idle, EXIT=0) + h264_sw_differential 17/17 vs ffmpeg 7.1.5 MD5. The gate lives at THIS file's :55-56/:60-61. |

## 7. Dead / duplicated / unreachable code
- Unreachable exports: `H264Decoder`, `H264HwBridge`, `H264HwDecoder`, `H264HwPath` (:58-59) — no production consumers (hw lane, W06-A07's finding; cross-confirmed by my vaapi/mod.rs caller sweep).
- No duplication; the module root is minimal and accurate.

## 8. Dependencies & flags
- The single feature touching this file: `codec-h264-sw = ["dep:rust_h264"]` (Cargo.toml:73), default-on (:53), pin `rust_h264 = "=0.6.1"` (:174). Everything else under h264/ is unconditional. No unsafe at this level. Lint-clean per baseline clippy (#5/#6).

## 9. Open questions for the next agent
- Matrix-rows agent: F-001's evidence-string fix (`sw/**` → `sw.rs`) clears baseline row 8's row-23 violation — one-line change, coordinate so it is counted once.
- scripts/CI agent: does any tier of `scripts/quality/`'s runner (the n99 run_all the audit.yml workflow invokes) execute 09_features.sh? If yes, F-002's "no CI lane" is wrong and should be downgraded to "not in baseline evidence" — UNVERIFIED from here.
