# CLAIM AUDIT: docs/07-config-driven-system.md
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Claims extracted | 11 (>300 LOC doc, depth mode) |
| REAL 4 / REAL-UNPROVEN 1 / SHALLOW 1 / STUB 0 / PAPER-ONLY 4 / FALSE 1 / UNREACHABLE 0 | |
| Internal contradictions found | 2 (status header vs parked planes; option count vs own inventory) |

## 1. Blunt verdict
The philosophy core is real and unusually disciplined (L1-L4 litmus tests, anti-pattern
taxonomy, validator that actually fires). The "Status: implemented" header is theatre:
the validator's own output parks two of the doc's planes, four brand assets are missing,
and two of the four G-plane artifacts the doc names do not exist on disk. This doc is the
W17 "quality-gate layer is theatre" pattern in miniature: the machinery exists, the
headline outruns it.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status \| implemented (awaiting Lead review)" | :6 | SHALLOW | see F-001: 4 missing assets, parked planes, absent G1/G3 artifacts |
| 2 | "Validated by product.schema.json via `cargo xtask config-validator`; a violation is a build-breaking error" | :86-88 | REAL | baseline xtask-config-validator.txt: runs, 4 violations detected, "result: FAILURE (4 violation(s))" — the gate detects and fails |
| 3 | "the product-config crate's CI suite owns" pattern constraints | validator note | REAL-UNPROVEN | validator v0 disclaims regex constraints; CI suite not re-run |
| 4 | "`themes/` … until the plane lands (doc 07 §9 C4)" (themes plane) | validator note | PAPER-ONLY | themes/ holds tokens.json + generated/ only; "no *.toml candidates" per validator |
| 5 | "`profiles/` does not exist yet … (parked plane, doc 07 §9 C4)" | validator note | PAPER-ONLY | ls: profiles/ absent |
| 6 | "Ordering … `layouts/`" | §1.2 table | PAPER-ONLY | ls: layouts/ absent at root |
| 7 | "G1 Settings screens `ui/src/generated/settings-schema.ts`" | :582 | FALSE | path absent; ui/src/generated/ contains only commands.ts + README.md |
| 8 | "G3 Documentation docs/settings-reference.md and docs/cli-reference.md" | :584 | PAPER-ONLY | both files absent from docs/ |
| 9 | "G4 … `crates/settings-schema`; `config.set-setting` validates every write" | :585 | REAL | crates/settings-schema present (src/ + Cargo.toml) |
| 10 | "VLC 3.0 exposes on the order of **2 000+** preference/CLI options" | :617 | FALSE | measured inventory options-linux.json = 1476 (my python3 count; parse-options.py re-run) — not "on the order of 2000+"; doc carries [VERIFY] marker but never updated after its own Phase-0 output |
| 11 | product.toml exists as single source of truth | §2 | REAL | product.toml + product.schema.json present at root |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| Status: implemented | validator: "themes/ has no *.toml candidates … until the plane lands"; "profiles/ does not exist yet — parked plane" | Both planes parked; status inflated (baseline output is the measurable truth) |
| "2 000+ [VERIFY … Phase 0 inventory output]" | options-summary.md / options-linux.json: 1476 | 1476 is the measured count; doc never reconciled |

## 4. Findings
F-001 [S3] Where: docs/07-config-driven-system.md:6.
Code (verbatim): "| Status | implemented (awaiting Lead review) |"
What is wrong: at audit day the config system is missing 4 brand assets (validator
FAILURE: brand.icon_ico/icns/png_512/logo_svg paths absent), two named planes are parked
(themes *.toml, profiles/), layouts/ does not exist, and G1/G3 artifacts named by the doc
are absent. "implemented" describes the core, not the system. What correct looks like:
status vocabulary per §1.4 rule F2 applied to itself — "implemented (core; layouts/themes/
profiles planes parked; brand assets missing)". Confidence: HIGH — Verification: read +
baseline cited + ls.
F-002 [S4] "2 000+" option-count claim is false against the doc's own Phase-0 inventory
(1476 measured, [VERIFY] never closed). Confidence: HIGH — Verification: ran.
F-003 [S4] G1 names `ui/src/generated/settings-schema.ts`; the generated dir holds
commands.ts only — the settings-surface generator either never landed or moved without a
doc update. Confidence: HIGH — Verification: ls.

## 5. Fabrication / inflation indicators
The inflation is confined to the status line and the option-count figure; the §1.4
borderline table and validator semantics are honest and checkable. The doc's own
[VERIFY] markers show where honesty lapsed into staleness.

## 6. Prior-audit reconciliation
Dead W19-A08's audit (old tree audit/docs/07-config-driven-system.md.audit.md) findings
(planes PAPER-ONLY, 4 brand assets, 1476 vs "2000+", T-matrix) all re-verified; the
validator baseline line "tables: 16" is the "16" of its T-matrix note.
