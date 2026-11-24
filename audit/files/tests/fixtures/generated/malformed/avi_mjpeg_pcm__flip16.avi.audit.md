# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__flip16.avi
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 425516 bytes / n.a. (25 assigned chunks L1-19364) |
| File verdict | REAL (fixture data; sha256 matches 3 independent records) |
| Reachable from shipped entry point? | NO (test fixture, consumed by 6+ test binaries) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 batch (exit 0); sed -n expectation toml; rg consumers (exit 0) |

## 1. One-paragraph verdict
Real generated class-B malformed fixture: 16-bit seeded header bit-flip derivative of
generated/valid/avi_mjpeg_pcm.avi, extent 64 bytes (AVI header region only — smaller than the
MP4 family's 512, correctly family-specific). Byte head `52494646 247e0602 41564920 4c495374`
= `RIFF` size `0x027e24` + `AVI LIST` — matches AVI magic and the 425516 recorded size.

## 2. What actually works (verified)
- python3 hashlib: sha256=2e08b0219bcc8025e29a87e973a803b631c8a98e46cdfee6e66b1122d753823b,
  size=425516 == `size_bytes = 425_516`
  (tests/fixtures/expectations/malformed/avi_mjpeg_pcm__flip16.avi.toml:15-16).
- Same sha in tests/fixtures/generated/MANIFEST.toml (line 118 block) and tests/fixtures/CORPUS.md.
  MATCH x3.

## 3. Findings
None on this artifact. Inside MANIFEST's 28-file drift gate (protected).

## 4. Provenance / consumers / matrix evidence
- Generator: tools/xtask/src/cmd/fixtures.rs:2828 `flip_derivative(parent, "flip16", 16, AVI_FLIP_EXTENT)`.
  Expectation DERIVATION (verified, cat of toml lines 18-24):
  `op = "bit-flip"` / `flip_bits = 16` / `flip_extent = 64`.
- Consumers (rg, exit 0): apps/cli/tests/bench_decode.rs:143 (bench decode of a CORRUPT file —
  asserts non-zero exit?), apps/cli/tests/unsupported_path.rs:67,
  crates/media-probe/tests/fixtures_matrix.rs:367,547, crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:113,
  tests/integration/first-vertical/tests/recovery_matrix.rs:625.
- Matrix evidence: docs/corpus/compat-matrix-20261110.raw-rows.jsonl cites this fixture with a
  recorded outcome (rg "avi_mjpeg_pcm__flip16" matched the file).
- Baseline cross-ref: rc-smoke.txt headless-decode exercises the AVI/MJPEG/PCM first-vertical
  fixture family (valid parent); this malformed derivative's pinned outcome is in the jsonl row.

## 5. Open questions
- bench_decode.rs:143 uses a malformed fixture for a bench — check the test asserts failure,
  not a pass-through (assignee of apps/cli/tests/bench_decode.rs).
