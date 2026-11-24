# AUDIT: tests/fixtures/generated/malformed/mkv_h264_aac__trunc4.mkv
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Size / sha256 | 26,097 bytes / 71ddc8d36a6dcbe446c6ad1942e9f00e7bccf1d78e9913b722ab98e8b0b68793 |
| Type | Matroska data (truncated) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 prefix check vs parent; rg consumers |

## 1. Verdict
Exact 26,097-byte prefix of parent `generated/valid/mkv_h264_aac.mkv` (26,105 B — cut 8 bytes before EOF; python3 `t4 == p[:26097]` → True). Parent is an ffmpeg-authored fixture (crates/demux/tests/mkv_fixtures.rs:4-13 header states "committed ffmpeg-authored fixtures ... ffmpeg 7.1.5"). Consumer: crates/demux/tests/mkv_fixtures.rs:354 in `malformed_derivatives_are_typed_or_recover_never_panics` (walks the demuxer: typed open refusal `demux.*` or bounded event walk with 1M event-storm cap). Baseline mkv_fixtures suite ran (cargo-test-workspace.txt:2353).
## 2. Findings
F-001 [S3]
Where: this file family vs tests/fixtures/CORPUS.md:14-17
Code (CORPUS.md): `Every file under `generated/` and `expectations/` is authored in-tree by the deterministic generator in `tools/xtask/src/cmd/fixtures.rs`, from public specifications only ... no external encoder output — ffmpeg was used only as a local cross-check oracle during development, never in the build.`
What is wrong: this file's parent is ffmpeg-authored external encoder output committed under `generated/`, and the derivative is outside the generator's manifest (NOT IN MANIFEST per python3 scan; drift gate verified 55/164 files — full_gate.txt:54). The licensing ledger claim is false for this family. Confidence: HIGH — Verification: read (mkv_fixtures.rs:4-13) + ran (python3 prefix + manifest scan).
## 3. What actually works (verified)
python3 prefix equality; consumer suite passes in baseline (cargo-test-workspace.txt:2353-2368 region).
