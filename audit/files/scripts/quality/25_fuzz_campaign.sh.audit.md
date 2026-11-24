# AUDIT: scripts/quality/25_fuzz_campaign.sh
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines / functions | 59 / 1 script |
| File verdict | REAL-UNPROVEN (fail-open failure labeling) |
| Reachable from shipped entry point? | NO — dev quality campaign script; not wired into any gate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read (full); ls fuzz/ (35 targets verified earlier); rg fuzz/Cargo.toml bin names; rg .gitignore |

## 1. One-paragraph verdict
A real cargo-fuzz campaign wrapper over the repo's 35 committed fuzz
targets (enumerated from fuzz/fuzz_targets/*.rs, matching fuzz/Cargo.toml's
bin entries), bounded by -max_total_time and -rss_limit_mb, preserving
crash artifacts into the reports tree. Its one real defect is fail-open
result labeling: a run that exits nonzero with zero artifacts is printed as
"clean", and the script's final status ([ $crashes -eq 0 ]) passes even if
every single target failed to build or run — the campaign "succeeds"
having fuzzed nothing. Absent tooling (cargo-fuzz + nightly) makes it skip
honestly, like its siblings.

## 2. What actually works (verified)
— Target enumeration matches reality: 35 .rs files in fuzz/fuzz_targets/
(verified in the sweep-parsers audit) and fuzz/Cargo.toml carries the
matching `name = "fuzz_*"` bin entries (:63-90+).
— cargo-fuzz's default corpus dir fuzz/corpus/<target>/ exists with 35 dirs
(registry-gated by scripts/fuzz_registry_check.sh) — the campaign reuses
committed seeds automatically.
— `need cargo-fuzz` skip path is labeled with the exact install command and
the nightly requirement (:21-24) — honest skip.
— Crash artifacts are copied to $OUT/fuzz/campaign/artifacts-<t>/ —
crash inputs kept (matches the header's "permanent regression case" claim;
the sweep-parsers.md fuzz results corroborate target health).

## 3. Findings
F-001 [S3]
Where: scripts/quality/25_fuzz_campaign.sh:45-50 and 59
Code:
```
  if [ "$arts" -gt 0 ]; then
    crashes=$((crashes + 1))
    say "CRASH: $arts artifact(s) -> fuzz/artifacts/$t/ (kept in reports)"
  else
    say "clean (exit=$rc)"
  fi
```
```
[ "$crashes" -eq 0 ]
```
What is wrong: crash detection is artifact-count ONLY. A target that fails
to build (rc nonzero, zero artifacts) prints "clean (exit=NN)" and counts
as healthy; a run where ALL 35 targets fail to start still ends with the
script exiting 0 ("$crashes of 35 targets produced artifacts" + success).
The gate is fail-open: broken campaign == green campaign.
What correct looks like: treat rc != 0 without artifacts as a run FAILURE
(not clean), and fail the script if any target failed to execute.
Confidence: HIGH — Verification: read (logic); not executed (cargo-fuzz
absent, and the script writes into repo trees).
F-002 [S4]
Where: scripts/quality/25_fuzz_campaign.sh:30-31
Code:
```
SUMMARY="$OUT/fuzz/campaign/summary.txt"
: > "$SUMMARY"
```
What is wrong: no date/host stamp in the summary (common.sh provides
stamp(); unused) — campaign provenance rides on the filename only, the
same evidence-quality gap as the gate logs.
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: scripts/quality/25_fuzz_campaign.sh:16
Code:
```
  mapfile -t TARGETS < <(ls fuzz/fuzz_targets/*.rs 2>/dev/null | xargs -n1 basename 2>/dev/null | sed 's/\.rs$//')
```
What is wrong: parsing `ls` (space-in-filename fragility — theoretical
here); mapfile is bash-4+ (fine on CI); no cross-check that each enumerated
name has a [[bin]] entry — a stray .rs file without a Cargo.toml bin entry
would fail at run time and be labeled "clean" per F-001.
Confidence: MEDIUM — Verification: read.

## 4. Per-section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| arg/target setup | 13-17 | REAL | — | SECS default 60; explicit target list supported |
| guards | 21-28 | REAL | — | honest not_run on missing tool / empty targets |
| run loop | 33-52 | REAL-UNPROVEN | S3 | bounded runs; fail-open labeling (F-001) |
| summary + exit | 54-59 | REAL-UNPROVEN | S3 | exit status = "no artifacts" not "all ran" |

## 5. Test honesty
No tests. Mutation reasoning: killing the `cargo fuzz run` line entirely
(turning every target into build failure) would still exit 0 — the script
cannot detect its own incapacitation. That is the fail-open proof.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "keeps every crash artifact ... A target that crashes once is a permanent regression case" | :5-7 | REAL | cp -r of fuzz/artifacts/$t into reports (lines 40-44) |
| "reports which targets are actually alive" | :6 | SHALLOW | "alive" is reported as "clean" even for dead (non-running) targets — F-001 |

## 7. Dead / duplicated code
None.

## 8. Dependencies & flags
cargo-fuzz (+ nightly for -Zsanitizer), libFuzzer flags passed through
(-max_total_time, -rss_limit_mb=4096). Outputs into gitignored
docs/audit/reports/fuzz/campaign/ (CI uploads as workflow artifacts).

## 9. Open questions
Did this campaign script ever run to completion anywhere? No
fuzz/campaign summary is committed; sweep-parsers.md's §2 campaign was a
different (uninstrumented, direct) method — so the instrumented campaign
this script provides has no committed evidence of ever executing.
