# AUDIT: scripts/quality/rules/assert-tautology.yml
| Field | Value |
|---|---|
| Auditor | W17-A05 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 14 / 1 ast-grep rule (6 alternatives) |
| File verdict | REAL-UNPROVEN (zero live targets; duplicates two deny-by-default clippy correctness lints) |
| Reachable from shipped entry point? | NO — CI audit lane only: 03_astgrep.sh:17-25; 99_run_all.sh:31 (static) + :44 (quality), J reporter lanes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | yaml.safe_load -> OK (6 patterns); census rg for all literal forms -> 0 code hits (only a comment); sweep worker.rs comment read; sed quote verification |

## 1. One-paragraph verdict
The entire rule duplicates compile-level enforcement that is already deny-by-default in clippy — `clippy::assertions_on_constants` (constant assertions, patterns 1/2/6) and `clippy::eq_op` (identical operands, patterns 3/4/5) — and it has ZERO live targets in the tree: the census finds no `assert!(true)`, no `debug_assert!(true)`, and the single textual `assert!(false)` is a comment that documents avoiding exactly this clippy lint. The authors demonstrably route around the compile-level lint, so the reporter-style duplicate is theatre with nothing to report. Two of its six patterns contradict its own message ("can never fail"): `assert!(false)` can never PASS, and `assert_ne!($A, $A)` never passes (except NaN).

## 2. What actually works (verified)
— verified by `python3 yaml.safe_load` -> OK, 6 patterns: `assert!(true)`, `assert!(false)`, `assert_eq!($A, $A)`, `assert_ne!($A, $A)`, `assert_eq!($A, $A, $$$)`, `debug_assert!(true)`.
— verified by census (rg over crates/apps/tools/tests): `assert!(true)` -> 0 hits; `assert!(false)` -> 1 hit, a COMMENT (tools/sweep/src/worker.rs:552 — see F-001); `debug_assert!(true)`/`(false)` -> 0 hits. The rule's literal-pattern target set in the tree is empty.
— verified wiring: 03_astgrep.sh:17-25 scans it over crates/apps/tools/tests (11 rules in the dir, this one among them).

## 3. Findings
F-001 [S3]
Where: scripts/quality/rules/assert-tautology.yml:9-14 vs tools/sweep/src/worker.rs:551-553
Code: `    - pattern: assert!(true)
    - pattern: assert!(false)` / worker.rs:551-553 `// Typed failure: no panic macro (F-5 panic-surface hygiene) and no
            // \`assert!(false)\` (clippy::assertions_on_constants). The test still
            // fails with the offending line in the message.`
What is wrong: the rule re-implements, weaker and unenforced, two deny-by-default clippy correctness lints: constant assertions (`clippy::assertions_on_constants` — patterns 1/2/6) and identical operands (`clippy::eq_op` — patterns 3/4/5). The in-tree comment proves the compile-level lint is alive and routed around by the authors — while this ast-grep copy sits in a never-gating J reporter with zero current targets. Same duplication class as panic-in-lib.yml (S3) but with an empty hit set to boot.
Confidence: HIGH — Verification: read + census.

F-002 [S4]
Where: scripts/quality/rules/assert-tautology.yml:5-6 vs :10, :12
Code: `  Assertion can never fail (constant or self-comparison). This is test
  theatre, not a test.` / `    - pattern: assert!(false)` / `    - pattern: assert_ne!($A, $A)`
What is wrong: the message is false for two of its own patterns: `assert!(false)` can never PASS (it is a constant failure, the opposite of a tautology), and `assert_ne!($A, $A)` fails for every value except NaN (x != x is false unless x is NaN). Only patterns 1/3/5/6 match the stated invariant, and even `assert_eq!($A, $A)` can fail when `$A` evaluates to NaN (NaN == NaN is false).
Confidence: HIGH — Verification: read (language semantics, no execution needed).

F-003 [S4]
Where: scripts/quality/rules/assert-tautology.yml:12-14
Code: `    - pattern: assert_ne!($A, $A)
    - pattern: assert_eq!($A, $A, $$$)
    - pattern: debug_assert!(true)`
What is wrong: asymmetric coverage — `assert_eq!` gets both the bare and with-format-args forms but `assert_ne!($A, $A, $$$)` is missing, and `debug_assert!(false)` is missing while `debug_assert!(true)` is present. Pattern 5's `($$$)` trailing match for the empty case has the same open question as todo-macros.yml (W17-A02) — if `$$$` requires a node, patterns 3 and 5 are disjoint only for the non-empty case.
Confidence: HIGH on the asymmetry (read); UNVERIFIED on `$$$` semantics (ast-grep absent).

F-004 [S4]
Where: scripts/quality/rules/assert-tautology.yml:3
Code: `severity: error`
What is wrong: decorative severity — nothing consumes it (03_astgrep.sh counts hits only); inconsistent across the sibling rules (error here, warning in panic-in-lib, warning in ignored-result).
Confidence: HIGH — Verification: read.

## 4. Test honesty
No tests; zero live targets (census); no committed astgrep json (outputs gitignored per W17-A06). Rule execution UNVERIFIED (ast-grep absent; `/usr/bin/sg` is util-linux per W17-A02).

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Assertion can never fail (constant or self-comparison)" | :5 | FALSE for patterns 2 and 4 (F-002) | semantics of assert!(false) / assert_ne!(x,x) |
| "This is test theatre, not a test." | :5-6 | Accurate as description of the target class; the rule itself is reporter theatre | J lane, never gates |

## 6. Dead / duplicated / unreachable code
Whole rule duplicates two deny-by-default clippy lints with an empty target set (F-001).

## 7. Dependencies & flags
ast-grep 0.51.3 (pinned via installer/sg_run). Consumed by 03_astgrep.sh in static+quality J tiers.

## 8. Open questions for the next agent
- `$$$` empty-args semantics (shared with todo-macros/panic-in-lib questions) — one ast-grep run settles all of them; UNVERIFIED here.
