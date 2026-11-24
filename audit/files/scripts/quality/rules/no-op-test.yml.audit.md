# AUDIT: scripts/quality/rules/no-op-test.yml
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / functions | 27 / 1 ast-grep rule |
| File verdict | REAL-UNPROVEN (rule logic sound and internally consistent; not executable in this sandbox — ast-grep absent, `sg` here is util-linux) |
| Reachable from shipped entry point? | YES (CI evidence lane): 03_astgrep.sh:26 `sg_run scan --rule "$RULES/$id.yml"` scans crates/apps/tools/tests; invoked by 99_run_all.sh:31,44 as `J 03_astgrep.sh` (reporter) — never gates |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `sg --version` -> "sg from util-linux 2.41.5" (NOT ast-grep); `ast-grep --version` -> not found; `sg scan --rule ... ; echo rc=$?` -> "unrecognized option '--rule'", rc=1; grep 99_run_all.sh -> 03_astgrep is J-tier |

## 1. One-paragraph verdict
The rule itself is a competently written ast-grep heuristic: a
`function_item` preceded by a `#[test]`/`#[tokio::test]` attribute whose
body contains none of the six macro families (assert*, panic*, require*,
insta*, matches*, unwrap*) is flagged as a candidate fake test, with the
message itself telling the reader it is only a candidate ("confirm with
mutation testing (T12)"). Two things keep this from being enforcement:
`severity: warning`, and the fact that 03_astgrep.sh runs in the J
(reporter) tier — no ast-grep rule can ever fail a tier, so no-op-test
detection has zero gating effect anywhere. The false-positive classes are
real (unwrap/expect METHOD calls are not macro_invocations), but the rule
admits it is a candidate-generator, not a verdict.

## 2. What actually works (verified)
- Rule structure read line-by-line: `follows: attribute_item regex
  '#\[(tokio::)?test'` is the canonical ast-grep Rust-test idiom; `not: has:
  stopBy: end` searches the whole function body — consistent.
- The consumer wiring is real: 03_astgrep.sh:22-27 passes this file to
  `sg_run scan --rule`, parses JSON hits, records failures via not_run.
- Honest failure protocol verified empirically: on this sandbox `sg` is
  util-linux's set-group tool (`sg --version` -> "sg from util-linux
  2.41.5"); `sg scan --rule ...` exits 1 with "unrecognized option" —
  03_astgrep.sh's `rc -ne 0` check records not_run instead of inventing
  hits.

## 3. Findings
F-001 [S3] — The rule suite is structurally unable to fail anything
Where: scripts/quality/rules/no-op-test.yml:3 (`severity: warning`);
scripts/quality/99_run_all.sh:31,44 (`J 03_astgrep.sh`)
Code:
```
id: no-op-test
language: Rust
severity: warning
```
What is wrong: warning severity + J-reporter placement means a thousand
no-op-test hits change no exit code anywhere (audit.yml is self-declared
"evidence, not a green light", audit.yml:17-18). The repo's only structural
no-op-test detector has zero enforcement path. Not hidden — but the message
"confirm with mutation testing (T12)" delegates to 12_mutants.sh, which is
also `J` (99_run_all.sh:49: `J 12_mutants.sh list`) — in `list` mode, i.e.
it enumerates, it does not mutate, in these tiers.
Confidence: HIGH — Verification: read 99_run_all.sh tier tables.
F-002 [S4] — False-positive class: unwrap/expect method calls
Where: scripts/quality/rules/no-op-test.yml:16-27 (the six macro regexes)
What is wrong: a test whose only failure mechanism is `foo.unwrap()` or
`x.expect("...")` (method calls, `method_call_expr` nodes, not
`macro_invocation`) is flagged "It cannot fail" — which is false: unwrap
panics on Err/None. Same for `#[should_panic]` tests that assert nothing
themselves, and for tests delegating assertions to helpers. The rule's own
message says "candidate", so this is a known-coarseness issue.
Confidence: HIGH — Verification: read (ast-grep node-kind semantics).
F-003 [S4] — `pretty_assertions::assert_eq!`-style paths escape `^assert`
Where: rules/no-op-test.yml:17 (`regex: '^assert'`)
What is wrong: a fully-qualified macro invocation text starts with the path
(`pretty_assertions::`), so `^assert` misses it; only bare macro names are
covered. Minor FP class.
Confidence: MEDIUM — Verification: read + regex semantics.

## 4. Per-function table (rule file — one rule)
| Rule | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| no-op-test | 1-27 | REAL-UNPROVEN | — | logic sound; cannot execute here; FP classes F-002/F-003 |

## 5. Test honesty
N/A (rule definition). Its own honesty is in the message ("Candidate fake
test - confirm with mutation testing (T12)") — accurate self-description.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "It cannot fail. Candidate fake test" (L5-6) | rule message | SHALLOW | true for the six macro families only; unwrap/expect-method tests CAN fail but match the rule (F-002) |
| 13b "AST heuristics (no-op tests ...)" consumes this rule | 13_test_hygiene.sh:17 | REAL | `for id in no-op-test ...` reads `$OUT/astgrep/no-op-test.json` |

## 7. Dead / duplicated / unreachable code
None within the file. Environmental: on stock Debian/Ubuntu the `sg` name
collides with util-linux sg (verified in this sandbox), so the sg_run
`command -v sg` branch fires the WRONG binary and fails loudly; the npx
fallback is unreachable on such hosts. Filed here as context — the defect
lives in common.sh (another agent's file).

## 8. Dependencies & flags
ast-grep (via sg_run: ast-grep | sg | npx @ast-grep/cli@0.51.3); language:
Rust; consumed by 03_astgrep.sh/13_test_hygiene.sh 13b.

## 9. Open questions for the next agent
- Does any committed run output exist with `no-op-test` hits (CI artifacts
  only)? In-repo: no docs/audit/reports/astgrep/ — execution UNVERIFIED
  (reason: ast-grep not installed; npx install violates load discipline).
- todo-macros.yml and ignored-result.yml (my next files) share this
  enforcement-free status — same J-tier conclusion applies.
