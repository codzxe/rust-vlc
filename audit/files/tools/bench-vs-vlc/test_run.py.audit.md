# AUDIT: tools/bench-vs-vlc/test_run.py
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Lines / tests | 544 / 30 (7 classes) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev-side unittest harness for tools/bench-vs-vlc/run.py (card 3.E3 acceptance); run manually / CI |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `python3 tools/bench-vs-vlc/test_run.py` -> Ran 30 tests in 0.707s, OK (exit 0) |

## 1. One-paragraph verdict
Mutation-resistant test suite for the bench-vs-vlc oracle harness. Executed by me:
30/30 pass in 0.707 s (dead agent's 30/30 @0.697 s reproduced). The suite pins the
honesty properties the harness promises: mock/real separation (absent VLC exits 2 with a
refusal that never mentions the mock), version-drift aborts (exit 3 via both the library and
CLI surfaces), PENDING reports carry zero oracle digests, [MOCK] labels on every mock row,
and single-byte PCM divergence is found at the exact offset.

## 2. What actually works (verified)
- Full run: 30 tests OK, exit 0 (includes the skipUnless(ffprobe) CLI-mode test — ffprobe
  present in this sandbox, so no skips occurred).

## 3. Findings
None. Zero-finding justification: every class below was read line-by-line; assertions are
behavioral (offsets, exit codes, stderr contents, evidence-json fields), not tautological;
no test asserts a property of the code under test by re-calling the code (no
mock-of-the-thing-tested — the mock oracle is the harness's own documented mode, exercised
through the real subprocess seam, and the suite separately proves the real oracle refuses
rather than degrading).

## 4. Per-class / per-test table
| Class / test | Lines | Verdict | Notes |
|---|---|---|---|
| TestClassification.test_comparable_families | 59-67 | REAL | pcm s16/u8/f32le -> comparable |
| TestClassification.test_non_comparable_undumpable_pcm | 69-75 | REAL | s24/s32/f64 rejected with reason |
| TestClassification.test_video_codec_does_not_gate_audio | 77-86 | REAL | mpeg4+pcm comparable (corpus-verified comment) |
| TestClassification.test_non_comparable_video_without_audio | 88-93 | REAL | |
| TestClassification.test_non_comparable_no_audio | 95-100 | REAL | |
| TestClassification.test_non_comparable_lossy | 102-107 | REAL | "legitimately differ" reason |
| TestClassification.test_non_comparable_other_container_lossless_audio | 109-116 | REAL | wav audio-only -> non-comparable |
| TestClassification.test_non_comparable_be_wire | 118-123 | REAL | no pinned afile format |
| TestClassification.test_excluded_corrupt | 125-131 | REAL | dirty/dirty-lie-ext/garbage -> excluded |
| TestSyntheticPcmComparison (5 tests) | 137-171 | REAL | identical MATCH; single-byte XOR @777 found; truncation offset; empty match; md5_file vs hashlib |
| TestMockOracle.test_version_pin_accepts_pinned | 182-185 | REAL | |
| TestMockOracle.test_version_drift_aborts | 187-198 | REAL | env-var drift -> VersionDrift with both versions in message |
| TestMockOracle.test_dump_is_deterministic_and_synthetic | 200-212 | REAL | two dumps equal, != input (synthetic) |
| TestMockOracle.test_dump_echo_hook_copies_ground_truth | 214-233 | REAL | echo hook + md5 cross-check |
| TestMockOracle.test_dump_command_shape | 235-245 | REAL | exact 13-token command incl. --no-audiofile-wav |
| TestRealOracleRefusal.test_resolve_raises_when_absent | 252-259 | REAL | empty PATH -> OracleUnavailable |
| TestRealOracleRefusal.test_main_vlc_mode_exits_2_when_absent | 261-282 | REAL | subprocess end-to-end; stderr has "no vlc/cvlc on PATH", no "mock" — the separation proof |
| TestReportLabels.test_pending_report | 309-327 | REAL | PENDING-VLC banner, to-run command, zero oracle digests |
| TestReportLabels.test_mock_report_labels | 329-357 | REAL | MATCH [MOCK] + evidence rows carry oracle_md5 |
| TestReportLabels.test_mock_mismatch_path | 359-373 | REAL | MISMATCH [MOCK] |
| TestVlcVersionParsing (2 tests) | 376-399 | REAL | fake binary drift/correct — shell fakes are honest fakes of the ORACLE, not of code under test |
| TestRefusedRows.test_lumen_refused_row | 407-447 | REAL | LUMEN-REFUSED [MOCK], never MATCH; run notes render |
| TestRefusedRows.test_manifest_provenance_line | 449-475 | REAL | manifest seed/count provenance string |
| TestRefusedRows.test_files_mode_rel_and_class_cli | 477-513 | REAL (ran) | end-to-end main(); explicit/dirty classes; --note |
| TestRefusedRows.test_vlc_mode_version_drift_exits_3 | 515-540 | REAL | exit 3 via CLI surface |

## 5. Test honesty
Every test would fail if the corresponding harness behavior broke (offset math, exit-code
contract, label contract, separation contract). The only skipped-conditional test
(ffprobe) ran in this environment. No #[ignore]-equivalents, no snapshot-of-self assertions.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Covers … mock/real separation; version drift aborts; PENDING reports carry PENDING-VLC and zero oracle digests" | :2-15 docstring | REAL | corresponding tests executed OK |

## 7. Dead / duplicated / unreachable code
None. setUp loads run.py fresh per test (30 importlib loads — cheap, ~0.7 s total).

## 8. Dependencies & flags
stdlib only (hashlib/importlib/json/os/shutil/subprocess/sys/tempfile/unittest/pathlib).
Tests mutate os.environ with save/restore in finally blocks; filesystem writes confined to
tempfile.TemporaryDirectory. No leaks.

## 9. Open questions for the next agent
None for this file; run.py has its own audit (W19-A10, old tree).
