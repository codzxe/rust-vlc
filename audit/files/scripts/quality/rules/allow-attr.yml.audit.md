# AUDIT: scripts/quality/rules/allow-attr.yml
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 9 / 1 ast-grep rule (attribute_item kind + regex) |
| File verdict | REAL (executed via pinned ast-grep 0.51.3: 112 true hits) |
| Reachable from shipped entry point? | YES as tooling — default rule set of scripts/quality/03_astgrep.sh (J-tier reporter) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## 1. One-paragraph verdict
A minimal, working census rule: matches every `#[allow(` attribute and counts it as debt (severity: info). I EXECUTED it with the suite's pinned tool (`npx -p @ast-grep/cli@0.51.3 ast-grep scan --rule ... --json crates apps tools`): **112 hits**, top debtors crates/demux/tests/common/mod.rs (20), codecs h264 videotoolkit ffi (12), codecs stage_bench (10). The rule does what it says; the debt exists and is measured. One scope note: the baseline's "121 #[allow] in tree" (audit/runs/baseline/loc-patterns.txt) vs my 112 in crates/apps/tools — a scope/counting-method difference (this rule scans the 03_astgrep PATHS; the baseline's number's exact scope is unrecorded here), not a rule defect.

## 2. What actually works (verified)
— EXECUTED (stdout-only): pinned ast-grep 0.51.3 + this rule -> 112 hits with per-file counts (ran; output in run log).
— Rule shape correct: `kind: attribute_item` + `regex: '#\[allow\('` — matches the attribute node's full text, anchored on the opening paren; `#[allow (…)]` with a space would be missed by the regex — see F-001.

## 3. Findings

F-001 [S4]
Where: scripts/quality/rules/allow-attr.yml:9
Code:
```
  regex: '#\[allow\('
```
What is wrong: the regex requires the literal `#[allow(` — an attribute written `#[ allow(...) ]` or `#[allow (...) ]` (legal Rust spellings) escapes the census. Edge-shape undercount only; today's tree hits 112 via the canonical spelling. Confidence: HIGH (read; canonical spelling dominates).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| rule | 7-10 | REAL | S4 | executed: 112 hits |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| executed scan | YES | 112 true positives; a broken regex yields 0 hits — the 112 proves the matcher fires |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every #[allow] is ... debt" (counted) | rule message | REAL | 112 counted in the current tree (ran); baseline's separate count 121 (loc-patterns) — scope-difference noted |
| baseline: "121 #[allow] exist in tree" | AGENT-SPEC §6.4 note | REAL (as a count) | my scoped recount 112 (crates/apps/tools via this rule); delta = scope/method, UNVERIFIED which files the baseline's extra 9 live in |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
ast-grep 0.51.3 pinned (executed exactly).

## 9. Open questions for the next agent
1. The 20 `#[allow]` in crates/demux/tests/common/mod.rs — what rules are being suppressed in test-shared code? (Debt census exists; triage is elsewhere.)
