# AUDIT: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__trunc4.mp4.toml
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A08) |
| Date | 2026-06-14 |
| Lines / functions | 28 / data file (schema/derivation/error keys) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A data — consumed by 4 test binaries + 2 gate artifacts (below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 sha256/size/prefix-compare of the binary; rg consumers; baseline + phase7 log greps |

## 1. One-paragraph verdict
Fully accurate expectation record, verified end-to-end: the pinned sha256
(9ebf78e4…2e36) and size (424,451) reproduce byte-exactly under python3; the
derivation ("cut the final byte", cut_offset 424_451) is TRUE — the binary is an
exact 1-byte-short prefix of the 424,452-B parent. Consumers exist at four
layers and all have green records: xtask sha-check (:6625), media-probe verdict
(:3672-83), codecs harness survival (:1120), and the full-stack recovery row
(open accepted → fatal demux.mp4.truncated, phase7 gate log :2480).

## 2. What actually works (verified)
— python3: binary 424,451 B, sha256 9ebf78e460a062979c1d3c9c0a03107605af403920df881d468643f553802e36
== TOML sha256; d == parent[:424451] TRUE (parent 424,452 B — exactly the final
byte cut).
— Consumers, all green:
  * tools/xtask fixtures.rs:5311-5340 derivative_expectations_carry_the_error_
    contract (asserts TOML sha256 line == sha256_hex(file.bytes), op =
    "truncate", [error] contract keys) — baseline :6625.
  * crates/media-probe/tests/fixtures_matrix.rs:426-436 — trunc2/3/4 loop: Hit
    mp4/95/BOTH_HINTS (moov fully inside the window) — baseline :3672-3683
    (8 passed; test mp4_class_b_derivatives_are_classed_per_their_nature).
  * crates/codecs/tests/fuzz_survival.rs file list — baseline :1120
    (every_fixture_file_survives_the_harness).
  * tests/integration/first-vertical/tests/recovery_matrix.rs:746-759
    matrix_mp4_trunc4_opens_then_lands_in_error_typed — Outcome::
    ErrorAfterOpenNoFlush with message_part "demux.mp4.truncated" — green in the
    committed dated gate log docs/gates/logs/phase7-exit-test.log:2480 (2026-11-10,
    tree c339ee8).
— Registrations: tests/fixtures/generated/MANIFEST.toml:182 (sha+size pin,
same digest); tests/fixtures/CORPUS.md:96 (row with the SAME sha 9ebf78e4… and
size 424451 — cross-artifact digest agreement verified).
— Generated (cargo xtask fixtures, seed_root lumen-2a3-v1), not handmade.

## 3. Findings
F-001 [S4]
Where: this TOML [error] (:20-27) vs recovery_matrix.rs:747-758; and the baseline lane gap (see stszlie0 audit F-001)
Code: `expected = "typed-error-or-bounded-recovery"` (vs the specific pinned outcome `Outcome::ErrorAfterOpenNoFlush { message_part: "demux.mp4.truncated" }`)
What is wrong: (a) the union contract is deliberately vague — the TOML's own
comment (:21-23) defers the specific flavor to demuxer conformance, which
recovery_matrix pins; acceptable by design, so S4 not S3. (b) the tier-1
full-stack consumer has no green record in the ORCHESTRATOR baseline (first-vertical
aborted at evidence.rs before reaching recovery_matrix, baseline :2773-2795) — the
pass lives only in the dated phase-7 gate log (:2480).
What correct looks like: baseline re-run with --no-fail-fast; optionally a
TOML v1 that records the realized flavor.
Confidence: HIGH — Verification: read (both logs grepped; :2480 verbatim above).

## 4. Matrix evidence
Class-B row; CORPUS.md:96 documents it with the matching digest; the probe
layer's verdict (95/both hints) and the full-stack outcome (fatal truncated)
are both pinned green. No feature-matrix row cites this TOML directly.

## 5. Open questions for the next agent
- None for this file. The trunc4 family member is the best-covered fixture of
  the batch (4 green consumer layers + 2 registration artifacts).
