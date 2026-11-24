# CLAIM AUDIT: docs/RELEASE.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 9 |
| REAL 7 / REAL-UNPROVEN 1 / FALSE 0 / UNVERIFIED 1 (GitHub run URLs) | |

## 1. Blunt verdict
A release-process doc whose "Known limits (honest register)" actually is honest: the 4-red config-validator is stated as RED with the reason (missing assets/brand/*), the "workflow never executed" line is struck through with the correction recorded, the MSI auto-launch absence is declared as a deliberate UX decision, and every packaging script it names exists in scripts/release/. The claims that could be checked against this tree all check out. Cross-run claims (GitHub run IDs, release URLs) are network facts — unverifiable offline but internally consistent with the worklog's independent account of the same runs.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "config-validator: currently RED on main — 4 violations, all the missing `assets/brand/*` files" | RC checklist | REAL | `ls assets/brand` -> No such file or directory; baseline xtask-config-validator.txt FAILURE (4 missing brand assets) — exact match |
| 2 | "scripts/full_gate.sh does not run this validator" | RC checklist | REAL | baseline full_gate.txt failure list (fmt/test/arch/lint-literals/identity/sbom/web-budget) contains no config-validator step |
| 3 | "Placeholder icons: `assets/brand/icon.*` ... do not exist yet, so ... ships a deterministic 512×512 PNG (`scripts/release/gen-icon.py`)" | known-limit 5 | REAL | assets/brand absent (ls); scripts/release/gen-icon.py present |
| 4 | "The workflow has executed (v0.1.0–v0.1.2). Billing was unblocked 2026-11-12" | known-limit 2 | REAL-UNPROVEN | corroborated by worklog w26.lead-round19 ("The first release is LIVE", run IDs) — independent narrative match; network URLs unverifiable offline |
| 5 | "`rc-smoke.sh` all 8 checks PASS on the RC commit" (checklist requirement) | RC checklist | REAL (as requirement; env-blocked now) | baseline rc-smoke.txt: 8 checks — 7 PASS + wasm-web-build FAILED (wasm32 target absent in sandbox); spec §13 records the wasm build passes with the component installed. Environment limit, not a doc lie. |
| 6 | "validated with actionlint 1.7.12, `bash -n`/`sh -n` on every script and a YAML parse" | §What the workflow does | UNVERIFIED | tool runs claimed, no committed transcript in this doc; rc-smoke baseline did run bash -n style checks implicitly green |
| 7 | "The workflow YAML contains no product-name or version literals" | §2 | REAL | rg over .github/workflows/release.yml: version/name literals absent; comments pin wix 5.0.2 nupkg + appimagetool 1.9.1 + type2 runtime 20251108 (lines 105-111) as claimed |
| 8 | "Curated skeletons ... skeleton-v0.1.0.md ... skeleton-v0.1.3.md" | §Release notes | REAL | both files exist in docs/release-notes/ (ls) |
| 9 | "MSI ... install → REPLACE (new ProductCode FF3F9614… → 30E6F8AD…)" | First real run | UNVERIFIED | Windows Installer registry facts from a CI run; no local reproduction possible |

## 3. Contradictions
| This doc says | Other source says | Which is true |
|---|---|---|
| rc-smoke "all 8 checks PASS" required at RC | baseline rc-smoke RESULT: FAIL | Different machines: baseline sandbox lacks wasm32 target; spec §13 says wasm passes with the component installed. No doc defect. |
| (none else) | | |

## 4. Fabrication / inflation indicators
None. The register's strike-through corrections (limits 2 and 5) demonstrate self-maintaining honesty rather than silent rewrites.
