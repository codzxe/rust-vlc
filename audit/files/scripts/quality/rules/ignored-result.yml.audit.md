# AUDIT: scripts/quality/rules/ignored-result.yml
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / functions | 9 / 1 ast-grep rule |
| File verdict | REAL-UNPROVEN (single coarse pattern; type-blind by construction; enforcement-free placement) |
| Reachable from shipped entry point? | YES (CI evidence lane only): 03_astgrep.sh scans it over the tree; J-reporter tier (99_run_all.sh:31,44) — never gates |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | read; 99_run_all.sh tier grep (03_astgrep is J); `sg --version` -> util-linux 2.41.5 (ast-grep execution impossible here) |

## 1. One-paragraph verdict
One pattern, `let _ = $EXPR;`, flagged with the message "If the expression
is a Result, the error is swallowed." The pattern is real ast-grep syntax
and does catch the classic swallow, but it is type-blind: every `let _ =`
on a non-Result (the common "silence unused" idiom) is flagged under a
message about Result-swallowing, while sibling swallow idioms (`.ok();`
discards, `_ => {}` arms, `if let Ok(_) =`, bare statement-position calls)
are invisible to it. It is warning-severity in a reporter tier, so like its
siblings it cannot fail anything. The message's conditional ("If the
expression is a Result") is the rule's own admission that it cannot check
the one thing that matters.

## 2. What actually works (verified)
- Pattern syntax valid: `pattern: let _ = $EXPR;` — anonymous single
  metavariable, anchored as a let statement with semicolon.
- Consumer wiring real (03_astgrep.sh:22-27 rule loading + JSON hit parse).
- The rule's existence correlates with a real checklist item (spec §6.2
  "swallowed errors (let _ =, .ok(), unwrap_or_default, Err(_)=>{})") —
  this file covers exactly the first of those four idioms.

## 3. Findings
F-001 [S3] — Enforcement-free and type-blind: false positives on
non-Result drops
Where: scripts/quality/rules/ignored-result.yml:3-8
Code:
```
severity: warning
message: >-
  Value dropped with `let _ =`. If the expression is a Result, the error is
  swallowed and the caller can never observe the failure.
rule:
  pattern: let _ = $EXPR;
```
What is wrong: (a) pattern matches ANY `let _ = expr;` — mutex guards,
RAII ignore, unused-var silencing — all reported under a Result-swallowing
message; with the noise, the true Result swallows drown; (b) warning
severity + J-reporter tier = zero enforcement; (c) misses `.ok()`,
`.unwrap_or_default()`, `Err(_) => {}`, and `let _ =` without trailing
semicolon (in expression position). Why it matters: the four-idiom swallow
class from the repo's own checklist is 25% covered, unenforced. What
correct looks like: type-aware check (clippy's `let_underscore_must_use` /
`unused_must_use` lints already exist and ARE enforced by clippy -D
warnings in ci.yml — making this ast-grep rule largely redundant).
Confidence: HIGH (placement/severity) / MEDIUM (FP-rate claims are
reasoning, not measured).
F-002 [S4] — Redundant with enforced clippy lints
Where: same file; baseline audit/runs/baseline/cargo-clippy.txt (CLEAN)
What is wrong: clippy runs `-D warnings` across the workspace in the
actual merge gate (ci.yml) and is CLEAN — the `let _ =` Result-swallow
class clippy covers (unused_must_use family) is already gated; this
ast-grep duplicate adds only noise-surface in a non-gating lane.
Confidence: MEDIUM — Verification: clippy CLEAN is baseline-cited; which
specific let_ lints are enabled in the workspace lint table UNVERIFIED
here (Cargo.toml workspace lints is another agent's file).

## 4. Per-function table (rule file)
| Rule | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ignored-result | 1-9 | REAL-UNPROVEN | — | one pattern; F-001/F-002 |

## 5. Test honesty
N/A (rule definition).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "If the expression is a Result, the error is swallowed and the caller can never observe the failure" (L5-6) | rule message | SHALLOW | true conditional; pattern cannot verify the condition — self-admitted type-blindness |
| 13_test_hygiene or 03 consumers treat it as a Result-swallow detector | 03_astgrep.sh usage | SHALLOW | counts pattern hits only; no Result filtering |

## 7. Dead / duplicated / unreachable code
None in file. Shared environment note: ast-grep absent in sandbox;
util-linux `sg` shadow (see no-op-test.yml audit §7) — execution
UNVERIFIED (ast-grep binary unavailable; npx install banned).

## 8. Dependencies & flags
ast-grep via sg_run; language Rust. Sibling rules covering adjacent
idioms: swallowed-error.yml (another agent's file).

## 9. Open questions for the next agent
- swallowed-error.yml (next agent): if it also targets `.ok()`/`Err(_) => {}`,
  the overlap/redundancy with clippy's enforced lints should be summarized
  once for the whole rules/ suite.
- Would the FP noise (non-Result `let _ =`) actually dominate? Requires an
  ast-grep run — flag as UNVERIFIED measurement for whoever has the binary.
