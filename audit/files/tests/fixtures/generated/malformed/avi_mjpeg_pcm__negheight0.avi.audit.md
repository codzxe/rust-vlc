# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__negheight0.avi
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 425,516 B / edae3802859061b9eda683c63ecd65ef45db0e87d96704eda1fdbb0be18beeaa |
| File verdict | REAL (data fixture; derivation independently re-executed) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

Assignment rows 2996-3020 (25 chunks -> one audit). What it is: AVI (RIFF/AVI )
class-B malformed corpus entry derived from tests/fixtures/generated/valid/
avi_mjpeg_pcm.avi by negating the video strf BITMAPINFOHEADER biHeight.

Derivation re-executed (python): exactly 4 differing bytes at offsets 180-183;
valid = f0 00 00 00 (240), malformed = 10 ff ff ff (= -240 LE). This matches the
committed expectation verbatim (tests/fixtures/expectations/malformed/
avi_mjpeg_pcm__negheight0.avi.toml:16-19):
`op = "negate-biheight"
 patched_offset = 180
 patched_value = 4_294_967_056
 note = "video strf biHeight patched to -240 (bottom-up DIB; ED-FIX-3 V1 — accepted with damage, not rejected)"`
sha256 matches expectation toml:11 and tests/fixtures/CORPUS.md row. Generator
tools/xtask/src/cmd/fixtures.rs:2685-2687 (`let patched = -(FRAME_HEIGHT as i32);`)
corroborates the negate op.

Generated-vs-handmade: generated (cargo xtask fixtures, seed lumen-2a3-v1).
Consumers (rg): crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:135
(`contained("generated/malformed/avi_mjpeg_pcm__negheight0.avi", 0, 59)`),
tests/integration/first-vertical/tests/recovery_matrix.rs (fixture row),
tests/fixtures/CORPUS.md, tests/fixtures/generated/MANIFEST.toml.
Matrix-evidence: docs/corpus/compat-matrix-20261110.raw-rows.jsonl:8 records
outcome status "decoded", 60 mjpeg + 60 pcm frames, demux_recovered=1 — consistent
with the "accepted with damage" contract in the expectation.

S4 (naming): the "0" in `negheight0` is a variant index; the mutation is biHeight
= -240 (bottom-up DIB), not height 0. Anyone reading the name alone would expect
biHeight == 0. No functional impact; the expectation + generator code are
authoritative.
