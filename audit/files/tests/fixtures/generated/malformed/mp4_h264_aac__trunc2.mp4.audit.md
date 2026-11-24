# AUDIT: tests/fixtures/generated/malformed/mp4_h264_aac__trunc2.mp4
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| File verdict | REAL — load-bearing malformed derivative (typed-refusal + fuzz-survival tests, baseline-passing) |
| Reachable from shipped entry point? | NO — test fixture; consumed by crates/demux/tests/mp4_fixtures.rs:231 (malformed family list incl. `malformed_mp4_fixtures_only_ever_refuse_or_demux_typed` and `malformed_mp4_fixtures_survive_the_fuzz_harness`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3: 6,000 bytes, sha256 cf5ab91be3e02984997bc9b405ae1f6e866c0680e0caef66d1f6fdd44b712a41; `file -b` -> ISO Media MP4 v1; structure: ftyp 1, moov 1, mdat 1, avc1 2, mp4a 1; derivation VERIFIED: byte-prefix of the committed parent generated/valid/mp4_h264_aac.mp4 (13,703 B; `p[:len(b)]==b` -> True); baseline cargo-test-workspace.txt:2373-2374 (both consuming tests ok; suite 5/5 :2377-2379) |

Generated-vs-handmade: genuine class-B-style derivative — a clean 6,000-byte prefix cut of the committed parent (python-verified this run; header boxes intact, truncation inside the media payload — exactly the mid-file truncation the `__trunc2` name promises). Consumers run GREEN in the orchestrator baseline (:2373 typed-refusal-only, :2374 fuzz-harness survival).
F-001 [S3]: unregistered — no row in tests/fixtures/generated/MANIFEST.toml (28 registered rows cover only the avi_mjpeg_pcm / mp4_mjpeg_pcm families; the mp4_h264_aac family has none) and no expectation TOML (`ls tests/fixtures/expectations/malformed | grep -c h264` -> 0; the 22 on-disk TOMLs are all avi/mp4_mjpeg_pcm). CORPUS.md's class-B contract (derivation record + error-contract TOML) does not exist for this family. Where: MANIFEST.toml (absence) + expectations/malformed/ (absence). Confidence: HIGH — Verification: ran (grep/python).
