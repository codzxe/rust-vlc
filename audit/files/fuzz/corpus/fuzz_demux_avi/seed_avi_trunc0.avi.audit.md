# AUDIT: fuzz/corpus/fuzz_demux_avi/seed_avi_trunc0.avi
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 12 B / ecd3e745fbf4c017dabed8fdc166dc8722f2b6ead6b151a34ba1e7eb54a8c628 |
| File verdict | REAL (as data); corpus copy itself never read |
| Reachable from shipped entry point? | NO — fuzz corpus seed; byte-identical twin in the tests fixture tree |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |

## 1. Verdict (corpus-seed audit)
The minimal `trunc0` AVI: 12 bytes = `RIFF` + a size field still claiming 425516 + `AVI ` form magic, nothing else — the demuxer must refuse at the header. It is sha256-identical to `tests/fixtures/generated/malformed/avi_mjpeg_pcm__trunc0.avi` (the tests-fixture twin), which IS consumed on stable by `trunc0_after_magic_is_typed_truncation` (crates/demux/tests/fixtures_class_b.rs:151) and the class-B fuzz-survival lanes. The corpus copy is read by nothing (campaign unwired).

## 2. What was checked and how
- batch.py: 12 B, sha256 ecd3e745…, RIFF `AVI ` declared size 425516 vs actual 12 — the lying-size truncation class.
- sha256 equality with tests/fixtures/generated/malformed/avi_mjpeg_pcm__trunc0.avi (verified).
- Twin execution: fixtures_class_b passes in baseline (audit/runs/baseline/cargo-test-workspace.txt:2242+ — `noidx0_demuxes_fully_via_the_movi_walk`-suite 8 tests ok, including the trunc0 pinned test).
- Registry gate (existence only) run by me: exit 0.

## 3. Findings
F-001 [S3] Corpus-lane gap (shared): no stable gate reads fuzz/corpus/fuzz_demux_avi/*. Verification: ran (sha256 + rg).
F-002 [S4] Byte-duplicate of a committed tests fixture (identical sha) — deliberate corpus mirror, but the two copies are pinned by different mechanisms (twin: MANIFEST-pinned; corpus: nothing), so they can drift apart silently.
