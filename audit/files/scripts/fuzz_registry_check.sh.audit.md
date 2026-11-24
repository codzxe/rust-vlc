# AUDIT: scripts/fuzz_registry_check.sh
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 79 / 1 gate flow (4-set walk + corpus gate) |
| File verdict | REAL (executed: CLEAN, 35 targets) |
| Reachable from shipped entry point? | YES as tooling — the doc 04 §8.5 registry gate (CI wiring per card 5.Q1); runs on the repo tree |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |

## 1. One-paragraph verdict
A real, working drift gate — I EXECUTED it: `fuzz-registry: CLEAN — 35 targets, diff empty, corpora present`, exit 0. It cross-checks four sets (Cargo.toml [[bin]] names, on-disk target files, README registry rows, and the in-crate harness fn each shim calls) plus the seed-corpus coverage rule, and any drift fails loudly. Two genuine weaknesses: a fully-empty registry (all targets deleted) passes CLEAN with a bogus "1 targets" count (the classic `printf '%s\n' "$empty" | wc -l` off-by-one), and the bin-name sed only recognizes `fuzz_*`-shaped names — a renegade non-conforming bin name escapes every check.

## 2. What actually works (verified)
— `bash scripts/fuzz_registry_check.sh` -> "fuzz-registry: CLEAN — 35 targets, diff empty, corpora present", exit 0 (ran).
— `bash -n` -> OK (ran).
— Spot-checks: fuzz/fuzz_targets/fuzz_codec_aac.rs exists; fuzz/corpus/fuzz_codec_aac/ has seeds (ls, ran) — the corpus gate has real inputs.
— Drift behavior verified by reasoning over the code paths: extra file on disk without a manifest bin -> `bins vs files` diff non-empty -> fail=1 (L52-60); README row missing -> second diff fails; shim not calling an in-crate harness -> harness_missing -> fail (L30-49); empty corpus dir -> fail (L65-71).
— W25 CI fix documented in-band: the old `rg ... 2>/dev/null` hid "command not found" and failed every probe; now plain `grep -r` (L41-45) — the comment records the failure class it replaced.

## 3. Findings

F-001 [S3]
Where: scripts/fuzz_registry_check.sh:73-79
Code:
```
count=$(printf '%s\n' "$bins" | wc -l)
if [ "$fail" -eq 0 ]; then
    say "fuzz-registry: CLEAN — $count targets, diff empty, corpora present"
```
What is wrong: `printf '%s\n' "$bins"` on an EMPTY bins list prints one empty line -> wc -l = 1. If the whole fuzz registry were deleted (bins empty, files empty, readme empty — consistent gutting), all set-diffs are empty, the corpus loop iterates zero times, the harness loop zero times -> exit 0 "CLEAN — 1 targets". The gate has no minimum-count floor, so total deletion reads as pass. Mutation reasoning: delete all [[bin]] rows + targets + README rows consistently -> CLEAN. Why it matters: "registry gutted" is a realistic drift class (a cleanup PR that removes the fuzz suite); the gate's purpose is to catch registry-vs-implementation drift and it certifies the empty registry. What correct looks like: `[ "$count" -ge 1 ]` or a hard expected floor. Confidence: HIGH — Verification: read + printf/wc behavior reasoned (not executed with a gutted tree — read-only repo).

F-002 [S4]
Where: scripts/fuzz_registry_check.sh:20
Code:
```
bins=$(sed -n 's/^name = "\(fuzz_[a-z0-9_]*\)"$/\1/p' fuzz/Cargo.toml | sort -u)
```
What is wrong: only `fuzz_*`-shaped bin names are collected; a [[bin]] named anything else (typo'd or non-conforming) is invisible to all four checks — it would drift silently (not flagged, not counted). The naming convention is thereby assumed, not enforced. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: scripts/fuzz_registry_check.sh:13-14 — `set -uo pipefail` + unguarded `cd "$(dirname "$0")/.."` (same class as soak-scenarios.sh F-001): a failed cd degrades to misleading later errors. Confidence: HIGH — read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| set walk (bins/files/readme) | 19-26 | REAL | S4 | F-002 shape assumption |
| harness-fn walk | 30-49 | REAL | - | shim-call regex + `grep -rq "fn $fn" crates/$crate/src` existence probe (weak-but-real; first path segment must be the crate) |
| set diffs | 51-61 | REAL | - | two pairwise diffs vs bins; drift -> fail |
| corpus gate | 63-71 | REAL | - | non-empty dir required per target |
| verdict/exit | 73-79 | REAL | S3 | F-001 empty-registry CLEAN bug |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the gate itself (executed) | YES for the drift classes it covers | single-target deletion -> three-way mismatch -> FAILED; empty corpus -> FAILED; F-001 gutting class -> NO (passes CLEAN) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Exit 0 = registry-vs-implementation diff EMPTY. Any drift fails." | L12 | REAL (for covered drift) | executed CLEAN + code paths; F-001 documents the uncovered drift class |
| "every target has a non-empty seed corpus ... CI failure" | L9-10 | REAL | corpus loop executed (35 corpora present, exit 0) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
bash, sed/diff/grep/ls (POSIX-portable set — the W25 fix deliberately dropped the rg dependency for CI portability); LUMEN_FUZZ_CORPUS env override for the corpus root.

## 9. Open questions for the next agent
1. Is this gate wired into ci.yml (the "CI failure" claim)? grep ci.yml for fuzz_registry_check — not in my file list.
2. F-001's minimum floor — cheap fix for the owner.
