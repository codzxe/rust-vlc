# AUDIT: tests/compat/src/oracle.rs
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Date | 2026-11-23 session |
| Lines / functions | 766 / 13 (+10 tests) |
| File verdict | REAL-UNPROVEN (real comparison logic; unit tests never run in the committed baseline — standalone workspace) |
| Reachable from shipped entry point? | NO — test-infra bin (`compat-matrix`, standalone workspace, invoked manually / by parked compat-matrix.yml) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 0 |
| Commands run | full file read; rg callers (main.rs:148 compare-pcm CLI; report.rs/config.rs refs); baseline greps |

## 1. One-paragraph verdict
The oracle side of the VLC 3.0.23 differential: honest, discriminated verdict types (BitExactPrefix/TolerancePass/Fail/NotBitExact/LumenRefused/OracleFailed/NoComparison), a version-asserted live VLC (absent binary → labeled committed-record; mismatched version → hard refusal), the MD5-prefix method for integer PCM, and an f32 RMS/peak tolerance path. The code does what its doc comments claim. The defects: (a) the f32 comparison window tolerates Lumen-side decode truncation; (b) the committed-record f32 path ECHOES the record's own measured deviations rather than comparing anything fresh; (c) nothing in the committed baseline executes these 10 unit tests (tests/compat is a standalone workspace, outside the root `cargo test` per-package baseline — verified by rg: no `### PACKAGE compat-matrix` in cargo-test-workspace.txt).

## 2. What actually works (verified)
- `prefix_verdict` (L260-273): truncates Lumen payload to vlc_bytes and compares MD5; `vlc_bytes > lumen.len()` → NotBitExact. Verified by reading + its 4 unit tests are exact (mutation-reasoning: a wrong truncation or hash fails them).
- `assert_version` (L176-208): absent binary → `OracleState::Absent` (soft, recorded); version mismatch → `Err` hard refusal. No silent downgrade.
- `committed_record_applies` (L466-471): record applied only when its `lumen_fresh_md5` equals the fresh decode MD5 — a real determinism check.
- NOT EXECUTED anywhere in audit/runs/baseline/ (standalone workspace; compat-matrix.yml is dispatch-only and CI is billing-blocked per its header L10-35). The committed evidence for compat is the 2026-11-10 local run (docs/corpus/compat-matrix-20261110.md), not a baseline test run.

## 3. Findings
F-001 [S2]
Where: tests/compat/src/oracle.rs:282-284, 307-323
Code (verbatim):
```
    // Head-aligned window: compare min(len) bytes as f32 LE samples.
    let comparable = lumen.len().min(vlc.len());
    let samples = comparable / 4;
```
What is wrong: a Lumen decode that truncates the audio (lumen.len() ≪ vlc.len()) is compared only over the head; RMS/peak over the surviving head can pass while the entire tail is missing — `TolerancePass` with no shortfall flag. The integer path rejects a short Lumen decode (`vlc_bytes > lumen.len()` → NotBitExact at L261-263), the f32 path does not. Why it matters: a truncated f32 decode (a realistic decode bug — flush/drain error) is graded PASS against the oracle. What correct looks like: record `lumen_shortfall_bytes` and fail (or at least flag) when lumen.len() < vlc.len() beyond a documented unflushed-tail bound. Confidence: HIGH — Verification: read.
F-002 [S3]
Where: tests/compat/src/oracle.rs:378-396
Code (verbatim):
```
        match (parts.rms_dev, parts.peak_dev) {
            (Some(rms), Some(peak)) => {
                ...
                let verdict = if rms <= tolerance.rms_max && peak <= tolerance.peak_max {
                    OracleVerdict::TolerancePass
```
What is wrong: for committed-record f32 rows the "comparison" re-evaluates the RECORD'S OWN recorded deviations against thresholds — circular evidence: the record said it passed; this path re-asserts the record. The only fresh fact checked is `lumen_fresh_md5` equality (caller-gated). On non-Linux CI lanes (no pinned VLC), all f32 "parity" rows reduce to self-consistency. Honest labeling (`VlcSource::CommittedRecord`) mitigates. Confidence: HIGH — Verification: read.
F-003 [S3]
Where: this file's 10 unit tests vs audit/runs/baseline/cargo-test-workspace.txt
What is wrong: zero baseline evidence — tests/compat is a standalone workspace (tests/compat/Cargo.toml `[workspace]` empty-table header), the root per-package baseline has no compat-matrix package (rg `### PACKAGE compat-matrix` → no match). The compat-matrix.yml job is dispatch-only and CI never executed (workflow header L27-31 "PARKED-BY-DESIGN ... MANUAL DISPATCH ONLY"; the Linux dev run 2026-11-10 is recorded but not reproducible from the baseline). UNVERIFIED: whether these 10 tests pass today (cargo absent). Confidence: HIGH — Verification: ran (rg).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| assert_version | 176-208 | REAL | — | version pin; absent vs mismatch correctly split |
| LiveOracle::dump | 214-255 | REAL | — | bounded VLC afile dump; exit!=0 → Failed row |
| prefix_verdict | 260-273 | REAL | — | MD5-prefix method; unbounded tail accepted (see F-001 sibling) |
| f32_verdict | 276-325 | REAL | S2 | head-window tolerates truncation (F-001) |
| compare_row_live | 330-361 | REAL | — | live path, real bytes both sides |
| compare_row_committed | 371-415 | REAL | S3 | echo of record deviations (F-002) |
| committed_rows | 418-437 | REAL | — | JSON record loader, typed errors |
| committed_record_applies | 466-471 | REAL | — | fresh-vs-record MD5 gate |
| committed_parts | 474-483 | REAL | — | read API |
| compare_pcm_cli | 486-545 | REAL | — | one-off CLI; errors typed |
| tests (10) | 547-766 | REAL | — | exact f32 powers-of-two; mutation-sensitive |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| prefix_verdict_bit_exact_with_tail | YES | exact enum equality incl. tail=40 |
| prefix_verdict_whole_payload | YES | tail=0 exact |
| prefix_verdict_rejects_mismatched_bytes | YES | corrupted byte → NotBitExact |
| prefix_verdict_rejects_oracle_longer | YES | 101>100 → NotBitExact |
| f32_verdict_identical_passes | YES | samples=min(4,3)=3 |
| f32_verdict_small_deviation_passes | YES | exact 2^-17 offsets |
| f32_verdict_out_of_policy_fails | YES | 2^-7 → ToleranceFail |
| committed_record_applicability | YES | md5 mismatch / None |
| compare_row_committed_integer_prefix | YES | tail=72 exact |
| compare_row_committed_f32_echoes_the_record | YES | pass/fail/NoComparison three-way |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bit-exact prefix method (VLC unflushed tail)" | file L5-9 | REAL | prefix_verdict L260-273 |
| "committed record cited, never inferred" | file L17-21 | REAL | VlcSource::None L71-77 + applies gate |
| compat evidence "0/252 verified every platform" | audit/runs/baseline/xtask-parity-report.txt | — | not this file's claim; cross-ref |

## 7. Dead / duplicated code
None observed; `BTreeMapOrHashMap` alias (L440) is a trivial rename — S4-level naming oddity only.

## 8. Dependencies & flags
serde/serde_json only (per tests/compat/Cargo.toml deps section); in-repo MD5 via bench-vs-vlc path dep. No feature gates.

## 9. Open questions for the next agent
- Does the compat report (docs/corpus/compat-matrix-20261110.md) count NoComparison rows in any "pass" bucket? (report.rs is another agent's file.)
