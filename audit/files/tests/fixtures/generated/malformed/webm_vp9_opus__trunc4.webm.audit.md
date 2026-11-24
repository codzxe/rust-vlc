# AUDIT: tests/fixtures/generated/malformed/webm_vp9_opus__trunc4.webm
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A08) |
| Date | 2026-06-14 |
| File verdict | REAL — load-bearing malformed derivative (typed-error survival test, baseline-passing) |
| Reachable from shipped entry point? | NO — test fixture; consumed by crates/demux/tests/mkv_fixtures.rs:363 (`malformed_derivatives_are_typed_or_recover_never_panics`, webm family in the same loop :362-366) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3: 27,251 bytes, sha256 51bbcb7fc53b0ef5d689c5b38261ab3a9e2f141b692eb46163fcfa50cd27e4e4, EBML magic 1a45dfa3; `file -b` -> WebM; content: doctype `webm` x1, `V_VP9` x1, `A_OPUS` x1 — name matches content; baseline cargo-test-workspace.txt:2358 `malformed_derivatives_are_typed_or_recover_never_panics ... ok` |

Generated-vs-handmade: derivative of the (unregistered) valid/webm_vp9_opus.webm parent (trunc4 = truncation near full length; the family trunc0..trunc4 exists on disk). The consumer loop feeds it to the REAL MkvDemuxer and asserts typed refusal (stage Demux, `demux.*` code) or bounded recovery with an event-storm cap (<1,000,000) and no panic (mkv_fixtures.rs:370-397).
The consuming test is GREEN in the orchestrator baseline (cargo-test-workspace.txt:2358; suite 9/9 :2366) — this fixture's bytes are exercised by a passing test every run.
F-001 [S3]: NOT registered in tests/fixtures/generated/MANIFEST.toml (28 registered families only — avi/mp4_mjpeg_pcm; the webm family has no rows) and NO expectation TOML exists (`ls tests/fixtures/expectations/malformed | grep -c webm` -> 0). So the promised class-B contract (derivation record + error contract) does not exist for this file — CORPUS.md's class-B description does not cover it; only the panic-survival loop consumes it. Where: tests/fixtures/generated/MANIFEST.toml (absence) + expectations/malformed/ (absence). Confidence: HIGH — Verification: ran (grep/ls/python).
