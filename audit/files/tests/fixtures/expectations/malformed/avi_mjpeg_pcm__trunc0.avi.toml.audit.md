# AUDIT: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__trunc0.avi.toml
| Field | Value |
|---|---|
| Auditor | W16-A08 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Expectation manifest (expectation/1) for a class-B fixture |
| File verdict | REAL (digest-verified against its fixture this run; consumers green in baseline) |
| Reachable from shipped entry point? | N/A — test fixture metadata |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 sha256/size vs TOML; rg consumers; baseline citations |

## 1. Verdict
Same honest shape as its absidx1 sibling: fixture sha256 (ecd3e745…c628) and size (12
bytes) re-verified against the actual .avi this run — exact match, and the 12-byte cut
("cut after the RIFF size+form magic") is arithmetically consistent with the derivation's
cut_offset 12. Union error contract with the same v0 caveat as the sibling.

## 2. What actually works (verified)
- Digest verification (this run): fixture bytes -> sha256 ecd3e745fbf4… == TOML sha256 ==
  generated/MANIFEST.toml:58 row (trunc0, 12 bytes). Three-way agreement.
- Consumers green in baseline: codecs `tests/fuzz_survival.rs` names the fixture at :26 —
  binary ran green (cargo-test-workspace.txt:1115 Running tests/fuzz_survival.rs; suite
  green in the 4744-pass aggregate); media-probe `fixtures_matrix.rs` recorded_derivation
  reads this TOML directly (:300) — 8 passed (:3670); tools/mem-audit report + first-vertical
  recovery_matrix.rs:505 (binary UNRUN in baseline — package aborted at evidence.rs).
- cut_offset 12 == file size 12: the truncation claim is internally consistent.

## 3. Findings
F-001 [S4] — same union-contract deferral as the absidx1 TOML (identical [error] block,
"the specific flavor … is pinned by demuxer conformance (cards 2.B2/2.B3)"): this file
proves the bytes and the caps, not which union arm the 12-byte truncation triggers.
Confidence: HIGH — Verification: read. (Cross-ref: the demux side DOES pin typed behavior
— fixtures_class_b 11 passed baseline :2242 — but that evidence lives outside this TOML.)

## 4. What was checked and how
TOML read in full; sha256/size vs fixture (python3) and vs MANIFEST; consumer rg
(fuzz_survival.rs:26, fixtures_matrix.rs:300-304, recovery_matrix.rs:505,
mem-audit/report.rs:322); baseline lines for consumer binaries. No code executes from
this file.
