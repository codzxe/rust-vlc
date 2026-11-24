# AUDIT: scripts/quality/rules/panic-in-lib.yml
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 24 / 1 ast-grep rule (4 alternatives, 2 exclusion branches) |
| File verdict | REAL-UNPROVEN (wiring real; rule execution impossible here — ast-grep absent) |
| Reachable from shipped entry point? | NO — CI audit lane only: 03_astgrep.sh:17-25 scans it over `crates apps tools tests`; 99_run_all.sh:31 (static) + :44 (quality), both `J` reporter lanes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `python3` yaml.safe_load -> OK (keys/patterns/severity dumped); `command -v ast-grep` -> ABSENT (sg = util-linux, per W17-A02's todo-macros verification); rg over lib sources for assert/panic gating; sed quote verification |

## 1. One-paragraph verdict
Structurally sound ast-grep rule whose panic! half duplicates the workspace's compile-level `clippy::panic = "deny"` (root Cargo.toml:62) in a never-gating advisory reporter, and whose assert-family half — the only assert-in-library policy in the repo — is enforced by nothing at all: the workspace lint block has no assert lint, and this rule lives in a J reporter whose `severity: warning` is never read. The message even names `unreachable`, which this rule does not pattern-match.

## 2. What actually works (verified)
— verified by `python3 -c "yaml.safe_load(...)"` -> OK: keys id/language/message/rule/severity; 4 patterns `panic!($$$)`, `assert!($$$)`, `assert_eq!($$$)`, `assert_ne!($$$)`; severity `warning`.
— verified consumer wiring: 03_astgrep.sh:17-25 `sg_run scan --rule "$f" --json crates apps tools tests` -> counts hits (`say "rule=$id hits=$n"`), not_run on scan failure; rule dir has 11 yml files, this one among them.
— verified tree layout compatibility: every sampled assert/panic in lib files sits inside `#[cfg(test)] mod tests` (e.g. crates/video-engine/src/adjust.rs:196-197, crates/transcode/src/graph.rs:382, crates/packet-queue/src/limits.rs:334) — the rule's `mod_item identifier ^tests?$` exclusion matches the repo's actual test-module naming.

## 3. Findings
F-001 [S3]
Where: scripts/quality/rules/panic-in-lib.yml:4-6, 9-12 vs root Cargo.toml:59-66
Code: `  Explicit panic!/assert!/unreachable in library code: reachable from
  user-controlled input. Prefer Result and checked access.` / `    - pattern: panic!($$$)
    - pattern: assert!($$$)
    - pattern: assert_eq!($$$)
    - pattern: assert_ne!($$$)`
What is wrong: two coverage mismatches. (a) The message names `unreachable` but no pattern matches it (unreachable! is todo-macros.yml's territory) — the rule's self-description is wrong. (b) The assert family is this repo's ONLY assert-in-lib policy (the workspace lints deny panic/todo/unimplemented/unwrap/expect — no assert lint), and it exists solely as an advisory J-reporter that cannot fail CI (static tier: 99_run_all.sh:31 `J 03_astgrep.sh`; severity never consumed). Meanwhile the panic! half duplicates the compile-level deny. Net: the unique coverage is theatre, the duplicate coverage is weaker than what already exists.
Confidence: HIGH — Verification: read (lint block + tier wiring quoted).

F-002 [S4]
Where: scripts/quality/rules/panic-in-lib.yml:9
Code: `    - pattern: panic!($$$)`
What is wrong: zero-argument invocation matching (`panic!()`) with `($$$)` is UNVERIFIED — same open question W17-A02 filed for todo-macros.yml; if `$$$` needs at least one node, bare `panic!()` is systematically missed.
Confidence: LOW (cannot execute) — Verification: UNVERIFIED (ast-grep absent in sandbox).

F-003 [S4]
Where: scripts/quality/rules/panic-in-lib.yml:3
Code: `severity: warning`
What is wrong: decorative — 03_astgrep.sh never reads the severity field (it counts hits and writes json); nothing downstream distinguishes warning from error. Inconsistent with sibling todo-macros.yml (`severity: error`) — neither has meaning.
Confidence: HIGH — Verification: read (consumer script read in full).

F-004 [S4]
Where: scripts/quality/rules/panic-in-lib.yml:13-24
Code: `    inside:
      stopBy: end
      any:
        - kind: mod_item
          has:
            kind: identifier
            regex: '^tests?$'
        - kind: function_item
          follows:
            kind: attribute_item
            regex: 'test'`
What is wrong: the exclusion set covers `mod tests` and `#[test]`-attributed functions only: helper functions in tests/*.rs (scanned — PATHS includes `tests`) without a `#[test]` attribute, and `#[cfg(test)] mod <not-named-tests>` bodies, are flagged as "library code" — false positives in the reporter. Current-tree hit count unknown (UNVERIFIED — no ast-grep).
Confidence: MEDIUM — Verification: read.

## 4. Test honesty
No tests. Cannot execute (ast-grep not installed; `/usr/bin/sg` is util-linux, not ast-grep — W17-A02's verification). YAML syntax verified parseable.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Explicit panic!/assert!/unreachable in library code" | :4-5 | FALSE (unreachable not matched) | patterns list has 4 macros, no unreachable (F-001) |
| "reachable from user-controlled input. Prefer Result and checked access." | :5-6 | PAPER-ONLY (advice in a reporter that cannot fail anything) | J lane + severity unused |

## 6. Dead / duplicated / unreachable code
The panic! alternative duplicates clippy::panic deny (weaker). No dead keys.

## 7. Dependencies & flags
ast-grep 0.51.3 (pinned via installer/sg_run npx fallback). Consumed by 03_astgrep.sh (static+quality J tiers).

## 8. Open questions for the next agent
- Does `($$$)` match zero-arg `panic!()`? Needs a real ast-grep run (UNVERIFIED).
- Actual hit count of this rule on the current tree — zero committed astgrep json in-tree (W17-A06: outputs gitignored) so the rule has no recorded result at all (UNVERIFIED).
