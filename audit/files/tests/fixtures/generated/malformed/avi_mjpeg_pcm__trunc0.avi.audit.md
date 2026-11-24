# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__trunc0.avi
| Field | Value |
|---|---|
| Auditor | W16-A10 | Date | 2026-11-24 |
| Size / sha256 | 12 B / ecd3e745fbf4c017dabed8fdc166dc8722f2b6ead6b151a34ba1e7eb54a8c628 |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only input; consumed by crates/demux/tests/fixtures_class_b.rs (pinned typed-truncation test) + fuzz-survival lanes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. Verdict (data audit)
The minimal `trunc0` AVI: 12 bytes = `RIFF` + declared size 425516 + `AVI ` form, nothing else. It is MANIFEST-pinned (sha256+size verified to match on disk — one of the 28 covered files) and its pinned outcome is a typed truncation error after the magic (`trunc0_after_magic_is_typed_truncation`, fixtures_class_b.rs:151), which passes in the committed baseline.

## 2. What was checked and how
- batch.py: 12 B, sha256 ecd3e745…, `riff form=b'AVI '` decl_size=425516, no chunks walkable; PAIRS: pure 12-byte prefix of the valid parent (0 diff bytes).
- MANIFEST pin: python cross-check -> manifest sha256/size MATCH (this file IS one of the 28 pinned).
- Consumers: crates/demux/tests/fixtures_class_b.rs:151-158 (`trunc0_after_magic_is_typed_truncation` + `derivative("trunc0")`); also `derivative_payloads_are_parent_prefixes` asserts the prefix property.
- Execution: audit/runs/baseline/cargo-test-workspace.txt:2242-2255 — fixtures_class_b suite ok (incl. `noidx0_demuxes_fully_via_the_movi_walk`, the suite's 8 tests).
- Note: byte-identical twin sits in fuzz/corpus/fuzz_demux_avi/seed_avi_trunc0.avi (see that audit).

## 3. Findings
F-001 [S3] None for this file beyond the family-level MANIFEST coverage note: this one IS pinned. The expectation TOML for the family pins a union contract (see avi_mjpeg_pcm__noidx0.avi audit F-001). No file-specific defect found: content, pin, consumer, and baseline run all verified.
