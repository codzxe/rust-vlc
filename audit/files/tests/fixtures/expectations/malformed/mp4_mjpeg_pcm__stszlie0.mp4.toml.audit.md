# AUDIT: tests/fixtures/expectations/malformed/mp4_mjpeg_pcm__stszlie0.mp4.toml
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A05) |
| Date | 2026-06-14 |
| Lines / functions | 29 / data file (schema/derivation/error keys) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A data — consumed by 4 committed test binaries + xtask gate tests (below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 hashlib/sha256 + byte-diff vs parent (executed); rg stszlie0 across tree; baseline greps |

## 1. One-paragraph verdict
Accurate, self-verifying expectation record. Every identity claim reproduces under
execution: sha256 of the binary matches byte-for-byte (c7d9ece0…7185, 424_452 B),
and the derivation is exactly 4 patched bytes — parent stsz sample_count 0x3c (60)
at offset 827 → 0x7fffffff (2^31−1), inside the moov `stsz` box at 811 (header
verified `\x00\x00\x01\x04stsz`). Two xtask unit tests already pin this on CI-green
(baseline :6643/:6625), and I reproduced both independently. The [error] union
contract has layered consumers; only the full-stack tier-1 consumer lacks any green
run record.

## 2. What actually works (verified)
— python3: child sha256 == TOML sha256 exactly; size 424452 == 424_452; diff vs
parent = offsets [827,828,829,830] only; parent bytes 823-835 `000000000000003c` →
child `000000007fffffff`. Box walk: ftyp@0 (28), moov@28 (2172), mdat@2200 (422252).
— Consumers (rg stszlie0): tools/xtask/src/cmd/fixtures.rs:2649-2652 (generator op
"stszlie0"), :5150-5168 `mp4_stsz_lie_derivative_patches_only_the_count` (asserts
exactly-4-diff + be32==0x7fffffff at stsz_video_count_offset) — GREEN baseline
cargo-test-workspace.txt:6643; `derivative_expectations_carry_the_error_contract`
(:5311-5340, asserts sha256 line == sha256_hex(file.bytes) + op =
"stsz-count-lie" + rss_cap_mb/wall_cap_ms) — GREEN baseline :6625;
crates/media-probe/tests/fixtures_matrix.rs:483-493 (reads this TOML's op, asserts
probe verdict Hit mp4/95/both hints — "the box the signature never opens") — GREEN
baseline :3672-3683 (8 passed); crates/codecs/tests/fuzz_survival.rs:35
(every_fixture_file_survives_the_harness) — GREEN baseline :1120;
crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs:145 (mp4 rows
pinned Refused probe/unsupported, D-018) — GREEN baseline :4398;
tests/fixtures/generated/MANIFEST.toml:221-227 pins sha+size (same digest).
— Generated (cargo xtask fixtures, card 2.A3; seed_root lumen-2a3-v1), not handmade.

## 3. Findings
F-001 [S4 — corrected post-write]
Where: audit/runs/baseline/cargo-test-workspace.txt:2773-2795 (vs) tests/integration/first-vertical/tests/recovery_matrix.rs:819-833
Code: `fn matrix_mp4_stszlie0_clean_ended_with_damage() { ... Outcome::CleanEnded { damage: Damage::AtLeastOne, frames: 120, ...`
What is wrong: the full-stack consumer of this TOML's [error] union contract has NO
green record in the orchestrator baseline — the first-vertical package aborted at
tests/evidence.rs (baseline :2773-2795, exit 101) before the recovery_matrix binary
ran (alphabetically after evidence; no --no-fail-fast). It DID run green in the
committed dated gate log docs/gates/logs/phase7-exit-test.log:2479 (`test
matrix_mp4_stszlie0_clean_ended_with_damage ... ok`, 23 passed/1 ignored, :2483;
log header: 2026-11-10, tree = origin/main c339ee8, --no-fail-fast inferred), which
I found only after writing the first draft at S3 — severity corrected down.
Why it matters: the orchestrator's own baseline understates tier-1 coverage; the
recorded pass is a dated artifact, not a fresh baseline row.
What correct looks like: baseline re-run with --no-fail-fast so recovery_matrix
lands in the canonical run alongside evidence's known failures.
Confidence: HIGH — Verification: read (rg over both logs; phase7-exit-test.log:2479
verbatim above).

## 4. Matrix evidence
Class-B row for the malformed corpus; docs/corpus/compat-matrix-20261110.raw-rows.jsonl
and tests/fixtures/CORPUS.md cite it. Executed today: sha verified by me; executed
green at 3 layers in baseline (probe / codecs-harness / playback refusal) + full-stack
tier-1 green in the dated phase-7 gate log (:2479) — fresh-baseline run absent (F-001).

## 5. Open questions for the next agent
- phase7-exit-test.log's header admits the --no-fail-fast flag is *inferred*, not
  recorded — treat the dated-log lane as good but secondary evidence.
