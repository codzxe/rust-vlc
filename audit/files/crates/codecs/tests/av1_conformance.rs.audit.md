# AUDIT: crates/codecs/tests/av1_conformance.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | 567 / 8 fns + 1 test (15 matrix rows) |
| File verdict | REAL (executed: 15/15 rows PASS in this sandbox, live ffmpeg+libaom oracle, 74.93s) — but it is a generated differential matrix, NOT official conformance vectors |
| Reachable from shipped entry point? | N/A (integration test; runs via `cargo test -p codecs --test av1_conformance`; baseline cargo-test-workspace.txt:977-986 ran it, 71.72s ok; I re-ran the built binary directly) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 1 |
| Commands run | direct execution of target/debug/deps/av1_conformance-52a40cdcb6e3e22a --nocapture -> 15/15 PASS, exit 0; ffmpeg -encoders (libaom-av1 present); ls /tmp/lumen-av1-conf-cache (all 15 cached); sed/rg quote checks |

## 1. One-paragraph verdict
The hostile-focus question — "real conformance vectors or fixtures only?" — answers: NEITHER. These are not the official AOM test vectors (the module doc admits "the official AOM test vectors are not available offline"), and they are not committed fixtures — 15 IVF files are generated at test time by this sandbox's own `ffmpeg -c:v libaom-av1` from `testsrc2`, then decoded by the in-tree rav1d-backed `Av1Decoder` and compared pixel-MD5-exact against a dump produced by the SAME ffmpeg binary. That is a self-consistent differential/round-trip matrix, honestly labeled inside the file, but named "conformance" — and the feature-matrix row cites it as "conformance 15 rows pixel-MD5 PASS". What the matrix does, it does genuinely: I executed the built test binary and all 15 rows pass, including per-frame geometry, pts restoration, packet/frame counts vs ffprobe, and whole-stream pixel MD5 vs the live oracle; the two boundary rows prove typed refusals (av1.bitdepth ×8, av1.layout ×8) with zero fabricated frames. The real defects are the stale "removed after the run" doc claim (the cache survives), and a name-keyed shared-/tmp cache whose "an existing non-empty file IS this row's bytes" assumption silently turns spec edits into stale-file tautologies.

## 2. What actually works (verified)
- Executed `./target/debug/deps/av1_conformance-52a40cdcb6e3e22a --nocapture` (binary newer than source: 18:07 vs 17:35): `test result: ok. 1 passed ... finished in 74.93s`, 13 exact rows `PASS ... pixel-MD5 ...`, 2 refusal rows `PASS tenbit_420_320x240: ... typed-refusal av1.bitdepth x8` / `PASS yuv444_320x240: ... typed-refusal av1.layout x8`.
- The doc's LR claim is observable in my output: `cpu4_320x240 ... 9227345f5e1ed8f35722f9dfa491224a` and `restoration_off_cpu4_320x240 ... 9227345f5e1ed8f35722f9dfa491224a` — same pixel MD5, different bitstreams, exactly as documented at :44-48.
- Baseline corroborates: cargo-test-workspace.txt:983-986 `conformance_matrix_vs_ffmpeg_generated_set ... ok ... 71.72s`.
- This sandbox: ffmpeg 7.1.5 with libaom-av1 encoder (verified), /tmp/lumen-av1-conf-cache contains all 15 files.

## 3. Findings
F-001 [S3]
Where: av1_conformance.rs:1-5 (module doc), test name :542, docs/feature-matrix.toml:469
Code:
```
//! AV1 conformance matrix (card 5.E2): the official AOM test vectors
//! are not available offline, so the honest conformance set is the
//! **systematic ffmpeg-generation matrix** the card prescribes —
```
What is wrong: the word "conformance" overstates. The official AOM corpus (which dav1d/libaom are validated against, exercising film grain, superres, overlay, complex reference structures, etc.) is absent; the coverage is what testsrc2 at cpu-used 4/8 produces (the doc itself concedes `-cpu-used 0..3` rows were dropped for encode cost and tool mix is cpu-used-limited). The feature-matrix evidence string "conformance 15 rows pixel-MD5 PASS" inherits the inflation. Why it matters: the repo's headline claim for AV1 decode correctness rests on a self-oracle round trip; a decoder bug shared with the encoder's tool subset would pass. What correct looks like: name it a differential matrix everywhere (the test fn's `_vs_ffmpeg_generated_set` suffix already does — the short name and matrix evidence string do not). Confidence: HIGH — Verification: read + executed (the run IS the round-trip proof).
Mitigation: in-file honesty is extensive (scope notes :31-48 mirrored to docs/formats/video-av1.md); the always-on committed-fixture surface is av1_differential.rs.

F-002 [S3]
Where: av1_conformance.rs:26-27 vs :559-562
Code:
```
//! Fixtures are generated into a fresh temp dir and removed after
//! the run; **when ffmpeg is absent every row SKIPs with a message**
```
vs
```
    // The cache dir SURVIVES the run (row-keyed, content-stable —
    // see run_row): repeat gates skip the ~10 min of libaom encodes
    // and re-verify decode + the live-ffmpeg oracle on the same
    // bytes. Delete the dir to force regeneration.
```
What is wrong: the module doc says removed-after-run; the code deliberately keeps a persistent cache under /tmp/lumen-av1-conf-cache. One of the two statements was true at some point; today the first is FALSE. Internal doc contradiction. Confidence: HIGH — Verification: read (both quotes sed-verified) + `ls /tmp/lumen-av1-conf-cache` shows the surviving cache.

F-003 [S3]
Where: av1_conformance.rs:444-453
Code:
```
    // Content-stable cache: `encode_args` is a pure function of the
    // row (deterministic testsrc2 + libaom-av1 CRF, `-y`), so an
    // existing non-empty file IS this row's bytes. The oracle stays
    // live either way: ffprobe facts + the ffmpeg dump MD5 are
    // recomputed from the file on EVERY run — the cache only skips
    // the (slow, deterministic) encode, never the comparison.
    let cached = path.is_file()
        && std::fs::metadata(&path)
            .map(|m| m.len() > 0)
            .unwrap_or(false);
```
What is wrong: the cache is keyed by ROW NAME only, in a world-writable shared /tmp dir. "An existing non-empty file IS this row's bytes" holds only if (a) row specs never change after first generation and (b) nobody plants a file. If a row's duration/g/tiles/cpu-used is edited, the stale file is silently used and every assertion (ffprobe facts, packet count, pixel MD5) is computed against the STALE bytes — self-consistently passing while testing the old spec (a tautology by staleness). A planted or crashed-encode partial file >0 bytes likewise poisons the row (the `.ivf` write is not atomic). The baseline directory itself contains `cargo-test-workspace.attempt1-diskpoisoned.txt` — this workspace has already seen one disk-poisoned attempt. What correct looks like: key the cache on a hash of `encode_args(row)` (the function is already pure — hash it) or a stamp file; write via temp+rename. Confidence: HIGH — Verification: read; staleness reasoning is structural (no version/key check exists in run_row).

F-004 [S4]
Where: av1_conformance.rs:541-550
Code:
```
fn conformance_matrix_vs_ffmpeg_generated_set() {
    if !ffmpeg_available() {
        eprintln!(
            "av1 conformance matrix: ffmpeg absent — all 15 rows SKIP \
```
What is wrong: with ffmpeg absent the test returns Ok having asserted nothing, and reports "1 passed". Honest (message + module doc point to av1_differential.rs as the always-on surface), but a pass-counting skip: environments without ffmpeg silently lose this entire coverage layer while CI stays green. What correct looks like: `#[ignore]` + explicit env-flag opt-in, or emit a machine-readable marker. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ffmpeg_available | 270-277 | REAL | — | same probe shape as vp9_differential |
| encode_args | 282-320 | REAL | — | pure fn of Row; executed (generation ran in 0.1-0.2s cached / 10 min fresh) |
| ffprobe_facts | 324-356 | REAL | — | csv shape assert (parts.len()==6) guards field-order drift; parse panics are test failures |
| ivf_packets | 371-384 | REAL | — | landed demuxer, not a reimplementation |
| decode_all | 387-410 | REAL | — | configure→decode→flush; error codes collected |
| frame_i420 | 412-423 | REAL | — | asserts CPU storage |
| ffmpeg_dump_md5 | 428-438 | REAL | — | live oracle per run (verified: recomputed even on cached encodes) |
| run_row | 441-539 | REAL | — | F-003 cache; strong assertions per row |
| conformance_matrix_vs_ffmpeg_generated_set | 541-567 | REAL | S4 | executed 15/15 PASS; F-004 skip path |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| conformance_matrix_vs_ffmpeg_generated_set (ffmpeg present) | YES | pixel MD5 vs independent dump of the same file; frame counts vs ffprobe; refusal codes pinned; executed here 15/15 |
| same test (ffmpeg absent) | NO | returns before any assertion; counts as passed (F-004) |
| cached-row runs (stale cache) | only for pixel-level breakage | geometry/count/MD5 all re-derived from the STALE file if a row spec changed — spec-regression blind (F-003) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "15 deterministic libaom-av1 files covering the axes that matter" | av1_conformance.rs:4-13 | REAL | run output: 13 exact + 2 refusal = 15 rows; axes present in ROWS |
| "the oracle is the same binary that authored the bytes … no pinned constants needed" | :18-21 | REAL | ffmpeg_dump_md5 recomputed per run; my run matched |
| "-enable-restoration 1 is a no-op … both rows print the same pixel MD5" | :39-48 | REAL | my run: cpu4 and restoration_off both 9227345f5e1ed8f35722f9dfa491224a |
| "Fixtures are generated into a fresh temp dir and removed after the run" | :26-27 | FALSE | code keeps /tmp/lumen-av1-conf-cache (:559-562); dir exists in sandbox (F-002) |
| "conformance 15 rows pixel-MD5 PASS, 8-bit bit-exact x3, 10-bit typed x5" | docs/feature-matrix.toml:469 evidence | REAL (run) / inflated (name) | executed; but "conformance" is a self-oracle differential matrix, not AOM vectors (F-001) |

## 7. Dead / duplicated / unreachable code
None. All helpers are consumed by the single test. Note the near-duplication of helper shapes (ffmpeg_available/decode_all/frame_i420/Sink) with vp9_differential.rs and av1_differential.rs — copy-paste test scaffolding, acceptable but notable.

## 8. Dependencies & flags
External: ffmpeg + ffprobe with libaom-av1 (present here: 7.1.5 Debian). Test-only; no cargo features. Uses common::md5_hex. Env dependence: shared mutable /tmp/lumen-av1-conf-cache (F-003).

## 9. Open questions for the next agent
- av1_differential.rs auditor: confirm the committed-fixture always-on surface actually covers bit-exactness when ffmpeg is absent (claimed at :28-29).
- docs/formats/video-av1.md auditor: whether the record mirrors the "removed after the run" falsehood or the surviving-cache truth.
- Whether official AOM vectors could be vendored (license: they are not freely redistributable in general — that is the stated reason for the generated matrix; verify that claim independently).
