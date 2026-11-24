# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__flip1.avi
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Binary fixture (class-B seeded header bit-flip, k=1) |
| Size / sha256 | 425516 bytes / 4e3207ccc5b4d0ca504ffa4b4ae5f97ba204570b2abbd6c802dcac7dd59bec9f (verified locally) |
| File verdict | REAL (generated, hash-locked, multiply consumed) |
| Reachable from shipped entry point? | N/A — test asset |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256 (match), header sniff, rg consumers |

## 1. Verdict
Real class-B mutation fixture of the AVI parent. Header intact
(`RIFF...AVI LIST...hdrlavih`), one seeded bit-flip somewhere in the family
header extent (by construction, not visible in first 32 bytes). sha256 and
size match MANIFEST.toml:102-107 exactly.

## 2. What actually works (verified)
- python3 hashlib -> `4e3207ccc5b4d0ca...` == MANIFEST sha256; 425516 == `size_bytes = 425_516`.
- MANIFEST.toml:102-107 (verbatim):
```
path = "generated/malformed/avi_mjpeg_pcm__flip1.avi"
class = "B"
op = "flip1"
parent = "generated/valid/avi_mjpeg_pcm.avi"
sha256 = "4e3207ccc5b4d0ca504ffa4b4ae5f97ba204570b2abbd6c802dcac7dd59bec9f"
```
- Consumers (rg): crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:105 (`contained("generated/malformed/avi_mjpeg_pcm__flip1.avi", 0, 100)`); tests/integration/first-vertical/tests/recovery_matrix.rs:594; crates/media-probe/tests/fixtures_matrix.rs:343; expectations/malformed/avi_mjpeg_pcm__flip1.avi.toml exists; compat-matrix malformed_root walker (tests/compat/src/corpus.rs:106).

## 3. Findings
None. Generated, pinned, four distinct consumers.
