# AUDIT: crates/codecs/tests/mjpeg_field_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 347 / 3 tests + 2 helpers + 17 Truth rows |
| File verdict | REAL for the 4 committed fixture lanes (verified present, pinned external MD5s, baseline ran them); corpus half is vacuous in this sandbox |
| Reachable from shipped entry point? | NO (test binary; exercises the shipped mjpeg decode path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | Read of full file; `stat tests/fixtures/mjpeg_h3/*.avi` (all 4 present, sizes match manifest.tsv:2377-2380); `ls /tmp/my-project/corpus-v4/media` (absent); `rg` for corpus_dir/md5_hex; baseline citation audit/runs/baseline/cargo-test-workspace.txt:1216-1221 |

## 1. One-paragraph verdict
The 17-lane MJPEG differential suite is honestly structured: 4 committed fixtures (odd-size 4:2:0 86x62, odd-size 4:2:2 84x46, interlaced-content tff/bff 160x112 — all verified present at the repo root `tests/fixtures/mjpeg_h3/` with byte sizes matching the manifest) decode through the real `decode_image` with frame-count, dimension, format, non-concealment, and whole-output-MD5 assertions against ffmpeg-7.1.5-measured truths — and those lanes genuinely ran and passed in the baseline. The corpus half (13 lanes incl. the 4:4:4 and 1280x720 files that carry the strongest IDCT evidence) early-returns when `/tmp/my-project/corpus-v4/media` is absent — it is absent here, so the baseline "ok" for `corpus_lanes_match_pinned_truth` proves nothing in this environment (same pattern as the SEI corpus tests, at least announced loudly). A secondary hazard: the `LUMEN_H3_MEASURE` env knob disables every assertion while the test still passes, so one environment variable silently converts the whole suite into a no-op.

## 2. What actually works (verified)
— `stat` -> tests/fixtures/mjpeg_h3/{mjpeg_420_odd.avi 17718, mjpeg_422_odd.avi 18108, mjpeg_interlaced_bff.avi 30266, mjpeg_interlaced_tff.avi 30150} — matches manifest.tsv:2377-2380 exactly; `file` -> "RIFF ... AVI, 86 x 62 ... Motion JPEG" (matches Truth w/h).
— Baseline: `fixture_lanes_match_pinned_truth ... ok` + `interlaced_content_fixtures_stay_polarity_free ... ok` (audit/runs/baseline/cargo-test-workspace.txt:1220-1221) — these lanes have no skip gate on committed fixtures (missing fixture would `unreachable!` loudly, line 287).
— `md5_hex` is a real RFC 1321 implementation with its own pinned-vector test (common/mod.rs:786-789) — not a stub hasher.

## 3. Findings
F-001 [S3]
Where: mjpeg_field_differential.rs:299-303
Code: (verbatim)
```
fn corpus_lanes_match_pinned_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!("corpus(mjpeg-h3): corpus dir absent — SKIP (loud)");
        return;
    };
```
What is wrong: 13 of the 17 pinned-truth lanes (including the 4:4:4 lane 0033 and the 720p lane 0092 that the module docs lean on for the ±1-LSB/±3-LSB divergence classes) pass vacuously when the external corpus dir is missing. `/tmp/my-project/corpus-v4/media` does not exist in this sandbox (`ls` -> No such file or directory), and cargo swallows eprintln of passing tests, so the baseline line 1219 `corpus_lanes_match_pinned_truth ... ok` is indistinguishable from a real pass and asserts nothing here. Whether the baseline environment had the corpus is UNVERIFIABLE from the committed log (no env recorded). The skip is at least announced and documented ("skip loudly") — unlike a silent one — but the recorded green is still unearned in a corpus-less environment. What correct looks like: gate on the corpus and report `ignored` (harness-visible), or commit 1-2 of the corpus lanes as fixtures.
Confidence: HIGH on mechanism; UNVERIFIED whether baseline had the corpus.
F-002 [S4]
Where: mjpeg_field_differential.rs:226-227 (+ 246-260, 290-294, 310-314)
Code: (verbatim)
```
    let measure = std::env::var_os("LUMEN_H3_MEASURE").is_some();
    let frames = match extract_avi_mjpeg_frames(bytes) {
```
What is wrong: with `LUMEN_H3_MEASURE` set, every assertion in `run_file` (concealment, polarity, w/h/format) and in both lane tests (`frames`, `md5`) is skipped and the test still passes — a single env var converts the entire suite into a vacuous "ok". The knob is a deliberate measurement mode (dumps `/tmp/h3/ours/*` frames + MD5s), but nothing records whether it was set during a given run, so a future baseline could silently lose all evidential value. What correct looks like: assert-at-least-one-lane unconditionally, or make measure-mode a separate #[ignore]d test.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Truth/TRUTHS/CORPUS_TRUTHS consts | 55-213 | REAL | — | 17 rows with w/h/frames/format/md5; fixture rows verified against disk |
| corpus_dir | 217-222 | REAL | — | LUMEN_CORPUS override, is_dir gate |
| run_file | 225-280 | REAL | S4 | decode + asserts (measure-gated, F-002); unreachable! on damage = loud |
| fixture_lanes_match_pinned_truth | 282-296 | REAL | — | 4 committed lanes, no skip gate — real evidence |
| corpus_lanes_match_pinned_truth | 298-316 | SHALLOW | S3 | vacuous without corpus (F-001) |
| interlaced_content_fixtures_stay_polarity_free | 325-346 | REAL | — | polarity None + no concealment on committed fixtures |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixture_lanes_match_pinned_truth | YES | pinned MD5 + frame count + geometry on committed bytes; a decode regression changes the digest (this is the external truth backing idct.rs) |
| interlaced_content_fixtures_stay_polarity_free | YES | polarity/concealment assertions on committed bytes |
| corpus_lanes_match_pinned_truth | NOT VERIFIABLE here | corpus-gated early-return pass (F-001); in a corpus-present environment it would be YES (same pinned-MD5 structure) |
| any test under LUMEN_H3_MEASURE=1 | NO | all assertions compiled out of the path (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the YUV plane MD5 of our decoder's output compared against ffmpeg ... dumps" | mjpeg_field_differential.rs:2-4 | REAL for the 4 fixture lanes (MD5s pinned, baseline ran); REAL-UNPROVEN for the 13 corpus lanes in this environment | F-001 |
| "I420 lanes: every plane of every file within ±1 LSB; two files bit-exact (0015, 0125)" | mjpeg_field_differential.rs:26-29 | REAL-UNPROVEN | those lanes are corpus-gated; my independent IDCT check (idct.rs audit) found ±2 worst-case on synthetic random blocks, consistent with the claimed rounding-boundary class but not a substitute for the file lanes |
| "Corpus rows skip loudly when the corpus dir is absent ... the fixture rows always run" | mjpeg_field_differential.rs:36-37 | REAL | verified: exactly that structure (F-001/F-002 quantify the residual risk) |

## 7. Dead / duplicated / unreachable code
None. All 17 truth rows are referenced by a running test (corpus rows conditionally).

## 8. Dependencies & flags
codecs::video::mjpeg::{DecodedFormat, decode_image} (the shipped decoder), tests/common (AVI frame extraction, md5_hex, fixture_bytes rooted at `<repo>/tests/fixtures` via CARGO_MANIFEST_DIR/../../tests/fixtures — verified). No features. Env knobs: LUMEN_CORPUS (corpus dir override), LUMEN_H3_MEASURE (assertion killer — F-002).

## 9. Open questions for the next agent
1. Whoever audits `tests/common/mod.rs`: `extract_avi_mjpeg_frames` is the evidence pipe for every lane — its 00dc-chunk walk should get its own audit.
2. The corpus-gate pattern (early-return pass) is repo-wide (sei.rs, slice.rs, this file, adpcm/vp9 corpus suites) — an orchestrator-level count of how many baseline "ok" lines are corpus-gated would quantify the evidential hole.
