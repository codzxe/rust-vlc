# AUDIT: fuzz/corpus/fuzz_demux_ivf/seed_ivf_vp9.ivf
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 2546 B / c5d58b77bac47c26d4b5b93d8224f4338df6072bf5b4766c13570fe783eda2cc |
| File verdict | REAL (as data); the corpus copy itself is never read |
| Reachable from shipped entry point? | NO — fuzz corpus seed; byte-twin exercised by demux ivf fixtures suite |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (corpus-seed audit)
Valid IVF container: `DKIF` magic, version 0, header length 32, fourcc `VP90`, 64x48, rate 25 / scale 1, 25 frames declared (batch.py ivf parse). It is sha256-identical to the committed ffmpeg-authored fixture `tests/fixtures/generated/ivf/vp9_64x48_25fps.ivf`, which the demux ivf fixtures suite demuxes bit-exactly against ffmpeg's own truth (`-c copy -f md5` oracle, ivf_fixtures.rs:37).

## 2. What was checked and how
- batch.py: 2546 B, sha256 c5d58b77…, DKIF header fields parsed (ver/hdrlen/fourcc/width/height/rate/scale/frames).
- sha256 equality with tests/fixtures/generated/ivf/vp9_64x48_25fps.ivf (verified).
- Twin consumers: crates/demux/tests/ivf_fixtures.rs:106 (`fixture: "generated/ivf/vp9_64x48_25fps.ivf"`); ffmpeg-truth tables at :77-106.
- Execution: ivf fixtures suite passes in baseline (audit/runs/baseline/cargo-test-workspace.txt, demux section).
- Registry gate (existence only) run by me: exit 0.

## 3. Findings
F-001 [S3] Corpus-lane gap (shared): no stable gate reads fuzz/corpus/fuzz_demux_ivf/*; campaign unwired. Verification: ran (sha256 + rg).
