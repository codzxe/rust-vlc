# AUDIT: scripts/quality/rules/process-exit.yml
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines / functions | 10 / 1 ast-grep rule |
| File verdict | REAL-UNPROVEN (valid ast-grep rule shape; cannot execute here) |
| Reachable from shipped entry point? | NO — dev lint rule; loaded by scripts/quality/03_astgrep.sh:24 (`sg_run scan --rule ...`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read (full); attempted `sg scan --rule` — /usr/bin/sg is NEWGRP not ast-grep (demonstrated error); rg census of std::process::exit in tools/apps |

## 1. One-paragraph verdict
A structurally valid ast-grep rule (id/language/severity/message/rule.any
with two $$$-variadic call patterns) whose intent — flag process::exit in
library code — cannot be enforced by the rule itself: it has no
library-only constraint, so the scan that drives it (03_astgrep.sh scans
crates apps tools tests, no exclusions) reports every legitimate CLI
`std::process::exit` as a severity:error hit. Ten-plus binary entry points
in this repo would be flagged as errors by their own lint. The rule also
misses the import form `use std::process::exit; exit(..)`, which the repo
currently does not use.

## 2. What actually works (verified)
— YAML shape is the documented ast-grep rule format (rule.any with two
pattern alternatives; `$$$` multi-metavariable for argument lists).
— The two patterns are complementary: `std::process::exit($$$)` matches
the fully-qualified path; `process::exit($$$)` matches the
`use std::process;`-style prefix form.
— rg census: the repo's binary mains (tools/vta/{mpeg4,vp9,ruopus,dnx}/
src/main.rs, tools/sweep, tools/mem-audit, apps/control-server,
apps/desktop-tauri) use std::process::exit — the rule WILL hit them.

## 3. Findings
F-001 [S3]
Where: scripts/quality/rules/process-exit.yml:4-10 vs scripts/quality/03_astgrep.sh:18
Code:
```
message: >-
  std::process::exit inside a library: kills the host process, skips Drop and
  destructors, cannot be caught, untestable.
rule:
  any:
    - pattern: std::process::exit($$$)
    - pattern: process::exit($$$)
```
What is wrong: the message scopes the rule to LIBRARIES, but the rule
encodes no such constraint (no `inside`, no file-kind/glob constraints),
and the driving scan (03_astgrep.sh:18 `PATHS=(crates apps tools tests)`)
includes every binary main.rs. All 10+ legitimate CLI exits in this repo
(tools/vta/mpeg4/src/main.rs:284 `std::process::exit(2)` etc.) become
severity:error hits, drowning the library-code signal the rule exists
for. The rule is unfalsifiable as written: it fires where its own
justification does not apply.
What correct looks like: constrain to library targets (e.g. ast-grep
`inside`/`has` constraints or scanning lib-only file sets), or lower
severity for bin entries.
Confidence: HIGH — Verification: read + rg census (execution of the rule
itself UNVERIFIED — no ast-grep here).
F-002 [S4]
Where: scripts/quality/rules/process-exit.yml:9-10
Code:
```
    - pattern: std::process::exit($$$)
    - pattern: process::exit($$$)
```
What is wrong: neither pattern matches the direct-import form
(`use std::process::exit;` ... `exit(2)`) — coverage gap; zero current
instances in the repo (rg over crates/apps/tools/tests found none), so
latent only.
Confidence: HIGH — Verification: read + rg.
F-003 [S4]
Where: (context) scripts/quality/common.sh:47-57 sg_run()
What is wrong: sg_run's second fallback is `command -v sg` — on this
sandbox /usr/bin/sg is newgrp (demonstrated: `sg scan --rule ...` ->
"unrecognized option '--rule'"; `sg --help` shows the newgrp usage), so
03_astgrep.sh would misroute ast-grep calls into a group-login command on
any Linux box lacking ast-grep but having util-linux. This is why the
rule could not be executed for this audit.
Confidence: HIGH — Verification: ran /usr/bin/sg (wrong binary).

## 4. Per-function table (rule sections)
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| rule metadata | 1-6 | REAL | — | valid ast-grep fields |
| rule.any patterns | 7-10 | REAL-UNPROVEN | S3 | valid patterns; unscoped blast radius (F-001) |

## 5. Test honesty
The rule has no tests; a single synthetic Rust file with all three exit
forms would pin its behavior (library hit, CLI false positive, import
miss). Nothing like that exists in the repo.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "std::process::exit inside a library: kills the host process ..." | :4-6 | SHALLOW | the rationale is true for libraries; the rule cannot restrict itself to libraries (F-001) |
| The scan reports rule hits as structural anti-patterns | 03_astgrep.sh:24-31 | REAL-UNPROVEN | never executable in this sandbox; hits output is gitignored like all quality outputs |

## 7. Dead / duplicated / unreachable code
None in-file.

## 8. Dependencies & flags
ast-grep CLI (via sg_run); rule language: Rust; severity: error. Sibling
rules: allow-attr, assert-tautology, blocking-in-async, ignored-result,
no-op-test, panic-in-lib, sleep-in-test, swallowed-error, todo-macros,
unwrap-outside-tests (same directory, same driver).

## 9. Open questions
Did any ast-grep run of this rule ever distinguish bin from lib hits?
No committed astgrep output exists (docs/audit/reports is gitignored) —
the ten sibling rules share the same no-committed-evidence gap.
