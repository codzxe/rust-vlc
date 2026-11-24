# AUDIT: tests/fixtures/generated/valid/mp4_mjpeg_pcm.mp4
| Field | Value |
|---|---|
| Auditor | W16-A10 |
| Date | 2026-11-24 |
| Lines / functions | binary; 424452 bytes; N/A (data fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only input (correct for a fixture); consumed by 20+ test files + tools/perf/baseline.py |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. One-paragraph verdict
Genuine, machine-generated Class-A parent fixture: a valid MP4 (`ftyp isom`) with a 2172-byte `moov` (2 tracks: mjpeg 320x240 @30fps video + pcm_s16le 44.1k stereo audio, per expectation TOML) followed by a 422252-byte `mdat` carrying 60 deterministic JPEG frames + PCM. Pinned by `MANIFEST.toml` (sha256 + size both verified to match on disk), consumed by a real acceptance suite that decodes all 60 frames and compares digests against an ffmpeg-derived reference; that suite passes 9/9 in the committed baseline.

## 2. What actually works (verified)
- `python3 audit/scratch/W16-A10/batch.py` -> size=424452, sha256=d0459524e3be5e90174c2a861724c3bb06fc12fbcbb52c7feea5f117aaa26da6; box walk: ftyp(28) moov(2172, 2 trak) mdat(422252); structure matches `mp4_mjpeg_pcm.mp4.structure.json` (`trak_offsets":[144,1327]`).
- MANIFEST pin check (python) -> manifest sha256 and size (424452) MATCH the on-disk file.
- Consumer chain: `crates/codecs/tests/fixtures_parent.rs:29` `const MP4: &str = "generated/valid/mp4_mjpeg_pcm.mp4";` -> `common::fixture_bytes` -> decode through real `MjpegDecoder` -> digest compare vs `tests/data/mjpeg_decode_reference.txt`.
- Execution evidence (orchestrator baseline): `audit/runs/baseline/cargo-test-workspace.txt:1044-1058` — `Running tests/fixtures_parent.rs ... 9 passed`, including `decoded_mp4_outputs_match_reference_digests ... ok`.
- External-oracle pin sibling `mp4_mjpeg_pcm.mp4.oracle.sha256` (ffmpeg's own decode, 39 entries) exists for this fixture.

## 3. Findings
F-001 [S3]
Where: tests/fixtures/generated/MANIFEST.toml:4-6 (whole-file scope; affects this fixture's protection envelope)
Code: (verbatim)
```
# Regenerate: cargo xtask fixtures. Drift gate: cargo xtask fixtures --check.
schema = "manifest/1"
generator = "cargo xtask fixtures (tools/xtask/src/cmd/fixtures.rs)"
```
What is wrong: the drift gate pins only 28 of ~117 files under tests/fixtures/generated/ (only the avi/mp4 mjpeg_pcm family). This fixture IS covered (verified), but its wav/aiff/ogg/mkv/mov sibling fixtures are NOT — the manifest does not protect them, so silent drift there is undetectable. Why it matters: the manifest is the only committed integrity pin for generated fixtures; 76% of the tree is unpinned. What correct looks like: manifest covers every generated file (or the gate is honest about partial coverage). Confidence: HIGH — Verification: ran (python manifest cross-check, 6/12 of my assigned generated files NOT IN MANIFEST).
Confidence: HIGH — Verification: ran.

## 4. Per-function table
N/A (binary data file).

## 5. Test honesty
Fixture is an input, not a test. The consuming suite is honest: it would fail if frames decoded differently (digest compare against ffmpeg-derived reference, baseline 9/9 ok).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "60 deterministic JPEG frames" | crates/codecs/tests/fixtures_parent.rs:4 | REAL | FRAME_COUNT=60; per-frame sha256 list in expectations/valid/mp4_mjpeg_pcm.mp4.toml `[essence_v0] video_sample_count = 60` |
| MANIFEST drift gate protects fixtures | MANIFEST.toml:3 | PARTIAL (28/117 files) | python check: 6 of my 12 generated files absent from manifest |

## 7. Dead / duplicated / unreachable code
No duplication for this file (fuzz corpus does not contain a copy of this fixture; the byte-identical 12-byte avi trunc0 appears in both trees — see that file's audit).

## 8. Dependencies & flags
None (data). Referenced from tests via path string; consumed by crates/{demux,codecs,media-probe,transcode,playback-engine,audio-output}/tests/*, tests/integration/first-vertical/tests/recovery_matrix.rs, tools/perf/baseline.py.

## 9. Open questions for the next agent
Does `cargo xtask fixtures --check` actually run in CI (which workflow)? The xtask feature-audit baseline already FAILS (2 missing evidence paths) — confirm whether the fixtures drift gate is wired into full_gate.sh.
