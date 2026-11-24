# AUDIT: scripts/quality/rules/todo-macros.yml
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / functions | 11 / 1 ast-grep rule (3 alternatives) |
| File verdict | REAL-UNPROVEN (pattern semantics sound for non-empty args; empty-invocation matching UNVERIFIED; severity `error` is decorative — nothing enforces it) |
| Reachable from shipped entry point? | YES (CI evidence lane only): 03_astgrep.sh runs it over crates/apps/tools/tests; 99_run_all.sh:31,44 places 03_astgrep.sh in the J (reporter) tier — never gates |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | read; `sg --version` -> util-linux (not ast-grep; execution impossible here); grep 99_run_all.sh tier tables; baseline loc-patterns citation |

## 1. One-paragraph verdict
A three-alternative ast-grep rule that flags `todo!`/`unimplemented!`/
`unreachable!` invocations. The pattern is the right shape and the message
is appropriately hostile ("a lie about reachability"). Two problems: it
declares `severity: error` while living in a reporter pipeline where no
severity is ever enforced — the "error" is decoration; and its patterns use
`($$$)` for arguments, whose matching of zero-argument invocations
(`todo!()`, the most common form of all three) I could not verify — if
`$$$` requires at least one node, the rule systematically misses the bare
calls. With 4126 `unreachable!` occurrences already recorded in the
baseline (audit/runs/baseline/loc-patterns.txt per the Wave-0 spec), this
rule's function is counting, not preventing.

## 2. What actually works (verified)
- The three patterns are syntactically valid ast-grep pattern syntax with
  anonymous multi-meta-variable `$$$` for the argument region.
- Consumer wiring: 03_astgrep.sh scans it over `crates apps tools tests`
  (plus `plugins` if present) — the rule reaches the whole tree.
- Message text is honest about both failure modes (runtime panic
  reachable from user input; lie about reachability).

## 3. Findings
F-001 [S3] — `severity: error` with no enforcement behind it
Where: scripts/quality/rules/todo-macros.yml:3; 99_run_all.sh:31,44
Code:
```
id: todo-macros
language: Rust
severity: error
```
What is wrong: the only tier that runs this rule (03_astgrep.sh) is a J
reporter — exit code recorded, tier never fails. ast-grep's scan --json
output consumed by 03_astgrep.sh:36 counts hits; severity never influences
anything downstream. An "error"-severity rule that cannot error anything is
a false sense of enforcement. Why it matters: a reader of rules/ sees
`severity: error` and assumes todo!() is blocked; it is not — 4126
unreachable! sit in the tree (Wave-0 baseline loc-patterns).
Confidence: HIGH — Verification: read 03_astgrep.sh + 99_run_all.sh.
F-002 [S4] — Empty-argument invocations may be missed by `($$$)`
Where: scripts/quality/rules/todo-macros.yml:9-11
Code:
```
    - pattern: todo!($$$)
    - pattern: unimplemented!($$$)
    - pattern: unreachable!($$$)
```
What is wrong: if ast-grep's multi-meta-variable requires at least one node
in the token tree, `todo!()` / `unreachable!()` — the dominant forms —
do not match. UNVERIFIED: no ast-grep binary in this sandbox (`ast-grep`
not found; `/usr/bin/sg` is util-linux 2.41.5, not ast-grep; npx install
banned by load discipline). What correct looks like: adding bare
`todo!()`-shaped patterns, or verifying $$$ matches empty sequences.
Confidence: LOW (flagged precisely because unverified) — Verification:
UNVERIFIED (ast-grep unavailable).

## 4. Per-function table (rule file)
| Rule | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| todo-macros | 1-11 | REAL-UNPROVEN | — | see F-001/F-002 |

## 5. Test honesty
N/A (rule definition). Message accuracy: "in shipped code" — scan scope is
crates/apps/tools/tests (incl. tests) — broader than "shipped"; test-only
todo! hits would be reported under a shipping-framing message. Minor
over-framing (S4-grade, folded into F-001's context).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "todo!/unimplemented!/unreachable! in shipped code" (L5) | rule message | SHALLOW | scan covers tests too; and nothing ships a failure on hits (J tier) |
| Wave-0 note "4126 unreachable! exist" | AGENT-SPEC §6.4 citing audit/runs/baseline/loc-patterns.txt | REAL | cited baseline; corroborates non-enforcement |

## 7. Dead / duplicated / unreachable code
None in file. Shared context: on stock Debian/Ubuntu the sg_run `sg`
fallback grabs util-linux `sg` and the whole astgrep lane degrades to
not_run (verified in this sandbox: `sg scan --rule ...` -> rc=1,
"unrecognized option '--rule'").

## 8. Dependencies & flags
ast-grep via sg_run; language Rust; consumed by 03_astgrep.sh (J tier).

## 9. Open questions for the next agent
- Empirically confirm whether `pattern: unreachable!($$$)` matches a bare
  `unreachable!()` under ast-grep 0.51.3 (pinned in common.sh:53) — this
  decides whether F-002 is a live false-negative engine or a nothing-burger.
- panic-in-lib.yml / swallowed-error.yml (other agents' files) share the
  enforcement-free status; the summary of this fact belongs to the
  astgrep-suite auditor.
