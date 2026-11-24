# CLAIM AUDIT: docs/reviews/w24-h-fixes-findings.md
| Field | Value |
|---|---|
| Auditor | W19-S1 (final recovery; original credit W19-A02; doc authored by W24-R6) |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL 7 / REAL-UNPROVEN 1 / FALSE 0 / UNVERIFIED 2 | |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A 2026-11-11 fix-verification audit of the w24/h packaging lane claiming 10/10 fixes
verified incl. negative-path re-runs. Its fix artifacts are ALL present in today's merged
tree and parse clean under `bash -n` (my own run). Its evidence-log digests grep-match.
The parts of its battery that need cargo/CI (lint-literals, gate.sh execution, fake-codesign
negative harnesses) are not re-runnable here. Notable: its own P4-2 (unescaped basenames)
was later hardened — gen-wxs.sh:329 now contains the `a&b<c>"d".md` test fixture. No
fabrication found.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "resolve_identity() runs in the MAIN shell... 2 NO credentials... 'refusing to fall back'" | :23-29 | REAL | scripts/packaging/sign-macos.sh:139,189-197: rc contract + verbatim "refusing to fall back to the unsigned path" |
| 2 | "22 mappings, all in NAME: ${{ secrets.NAME }} identity form" | :82-88 | REAL (23 today) | rg -c "secrets\." .github/workflows/release.yml -> 23; visibility step exists :1022 "Signing-credentials visibility (W24-R1 P1-2 hardening...)" |
| 3 | "gen-wxs.sh now emits one <File> row per PACKAGING_DOCS entry" | :112-114 | REAL | scripts/release/gen-wxs.sh:132 `:?` required; :152-162 Doc_${n} rows |
| 4 | "all five table digests... grep-FOUND in linux-lane-20261110.log" | :145-149 | REAL | docs/packaging/evidence/linux-lane-20261110.log:91-93 -> 3b5eae753466..., a39dafa88039..., c0b86e2984b1... |
| 5 | "PACKAGING_DOCS required... duplicate basenames → loud refusal rc 1" | :114-119 | REAL | build-msi.sh:69 `:?...not set`; gen-wxs.sh duplicate handling present |
| 6 | "bash -n + sh -n on all 14 scripts... clean" | :211 | REAL | my run: bash -n on sign-macos.sh, build-msi.sh, gen-wxs.sh -> all OK (exit 0) |
| 7 | "P4-2: doc basenames unescaped... a&b.md yields malformed XML" | :251-256 | REAL + later fixed | gen-wxs.sh:329 test fixture `PACKAGING_DOCS="a&b<c>\"d\".md it's.md"` (w24/q hardening) |
| 8 | "README still says 'all 41 steps'... now 42" P4-3 | :257-260 | REAL at date; fixed | docs/packaging/README.md:209 now "bash -n over all 42 run-blocks" |
| 9 | "cargo run -p xtask --locked -- lint-literals → PASS" | :215 | UNVERIFIED | cargo runs forbidden (load discipline); W17 baseline records lint-literals section RED in full_gate context — different invocation, not comparable |
| 10 | "source cargo-env.sh && bash gate.sh → PASS (51 vars...)" | :214 | UNVERIFIED | not re-run (needs the shared cargo env; gate.sh would re-execute pack tooling) |

## 3. Contradictions
None. The doc explicitly separates local-proven from CI-only ([VERIFY at first live run]
labels) and its overclaim sweep found exactly one bounded worklog phrase.

## 4. Fabrication / inflation indicators
None. The doc re-ran the fixer's negative tests itself, reproduced the pre-fix leak, and
downgraded W24-R1's wrong mechanism narrative honestly ("W24-R1's literal mechanism
narrative... was wrong for die()-based failures").

## 5. Notes for next agents
- The `|| true` P1-1 reframe (W24-R1's mechanism narrative wrong; real bug was the
  scratch/keychain leak) is corroborated by the script's current structure.
- scripts/release/gen-wxs.sh (not scripts/packaging/) is where the .wxs generator lives —
  cite the correct path.
