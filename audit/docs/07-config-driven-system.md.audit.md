# CLAIM AUDIT: docs/07-config-driven-system.md (2 parts, L1-891)
| Field | Value |
|---|---|
| Auditor | W19-A08 |
| Claims extracted | 22 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 12 / 2 / 4 / 0 / 4 / 0 / 0 |

## 1. Blunt verdict
The spine of doc 07 is REAL: product-config, settings-schema (with the schema embedded
via `include_str!`), localization (`audit_catalogs`), and every xtask gate it names
(config-validator, schema-gen, lint-literals, check-identity, rebrand, check-architecture)
exist and execute — the orchestrator baseline shows them running and FAILING on real
violations, which is what a real gate does. The rot is at the edges: the doc's Status
line says "implemented" while §7's declarative-UI planes (layouts/, profiles/ themes/
*.toml) are absent from the tree, the 4 brand assets the spec mandates do not exist
(validator: 4 violations), the rebrand acceptance harness and lint allowlist are missing,
and only 7 of the 16 precedence-matrix tests exist. "Implemented (awaiting Lead review)"
is inflated to that degree.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Status: implemented (awaiting Lead review)" | L6 | SHALLOW | core crates+xtask real (below); §7 planes + brand assets + rebrand tests absent (evidence rows 14-18) |
| 2 | "This is the real file content for the Phase 1 commit" (product.toml §2) | L80 | REAL | product.toml exists at root, structure matches §2 (identity/window/brand/features/playback/limits/telemetry/urls/migrations all present, sed-verified) |
| 3 | "validated by product.schema.json ... via cargo xtask config-validator" | L80, L87 | REAL | product.schema.json at ROOT (not schemas/ as §3.1 says — path drift); tools/xtask/src/cmd/config_validator.rs exists; baseline xtask-config-validator.txt shows a full run |
| 4 | "Path existence ... checked by config-validator (§3.4)" | L165 | REAL | baseline validator output: `violation: brand.icon_ico: asset path assets/brand/icon.ico does not exist` (x4) |
| 5 | "telemetry ... shipped defaults are off" | L203-207 | REAL | product.toml [telemetry] enabled=false, crash_reports="off" (sed-verified) |
| 6 | "Feature exposure lists only implementations that exist" (rule F1) | L210-214 | REAL-UNPROVEN | feature ids in product.toml §[features] all name engine features; the validator's registry intersection (R4/R7 of the tool) is documented as OWED in feature-matrix.toml header L12-14 — cannot prove enforcement from here |
| 7 | "The merge ... Implemented in crates/product-config" (§4.4) | L497 | REAL | crates/product-config/src/merge.rs + model.rs exist; tests/merge.rs exercises t1/t4/t10/t12/t13/t15/t16 |
| 8 | "The matrix is exhaustive over layer interactions; each row is a named test" (T1-T16) | L509 | SHALLOW | only 7/16 named tests present (t1,t4,t10,t12,t13,t15,t16 in tests/merge.rs; rg). T2/T3/T5-T9/T11/T14 absent — the profile/admin/session/lock layers are unimplemented, so their tests cannot exist |
| 9 | "One settings schema ... generates the settings screens, --help and CLI flags" (§5.1) | L536 | REAL | schemas/settings.toml (121 [[setting]] entries) + schemas/generated/{settings.d.ts, settings-help.txt, settings-docs.md} exist — G1/G2/G3 artifacts present |
| 10 | "the schema itself, embedded via include_str! into crates/settings-schema" (G4) | L585 | REAL | crates/settings-schema/src/runtime.rs:18 `const EMBEDDED_SOURCE: &str = include_str!("../../../schemas/settings.toml");` |
| 11 | "The seed total is ≈ 45 entries" (Phase 1) | L613 | SHALLOW (stale) | actual `rg -c '^\[\[setting\]\]'` = 121 — already past the doc's own Phase 4 budget (~120); planning figure never updated |
| 12 | "layouts/ decides existence and order" + §7.2 file table (main/menus/ui-actions/settings-areas/dialogs.toml) | L714-722 | PAPER-ONLY | all five layouts/*.toml MISSING (ls); settings-area navigation lives in UI code instead |
| 13 | "keymaps/ ... keymaps/default.toml seeds bindings" (§7.3) | L751 | REAL | keymaps/default.toml exists (281 lines) — the only §7 plane that landed |
| 14 | "profiles/<id>.toml — a named bundle" (§7.4) | L753 | PAPER-ONLY | profiles/ absent; validator output: `defaults.profile: profiles/ does not exist yet — enum injection skipped (parked plane, doc 07 §9 C4)` |
| 15 | "themes/ ... token references only" (§7.5) | L768 | PAPER-ONLY | themes/ has only generated/ + tokens.json; validator: `themes/ has no *.toml candidates — enum injection skipped` |
| 16 | "cargo xtask rebrand" (§8.1) | L791 | REAL | tools/xtask/src/cmd/rebrand.rs + crates/product-config/tests/rebrand_host.rs exist |
| 17 | "tests/regression/rebrand/ ships a fixture product definition" (§8.3) | L813 | FALSE-as-implemented → PAPER-ONLY | tests/regression/rebrand MISSING; only rebrand_host.rs unit test exists |
| 18 | "allowlist file tools/xtask/data/literal-allowlist.toml" (§6.5) | L693 | PAPER-ONLY | file MISSING; rebrand-allowlist.toml exists but literal-allowlist does not |
| 19 | "cargo xtask lint-literals ... Scans string literals" (C1) | L690 | REAL | lint_literals.rs exists; baseline full_gate.txt:63-66 shows it running and FAILING with 1 real violation (apps/cli/src/host.rs:1799) |
| 20 | "cargo xtask check-identity" (C2) | L833 | REAL | check_identity.rs exists; baseline full_gate.txt:64-66 shows FAIL with the same host.rs:1799 violation |
| 21 | "cargo xtask l10n-audit (backed by crates/localization::audit_catalogs)" (C3) | L678 | REAL | crates/localization/src/lib.rs:108 `pub use audit::{CatalogIssue, audit_catalogs, audit_catalogs_with_refs};` |
| 22 | "VLC 3.0 exposes on the order of 2 000+ preference/CLI options [VERIFY]" | L617 | FALSE | measured from the repo's own baseline: dump 13 (--advanced --longhelp) = 1476 option records; dump 14 (-H full) = 1476. The "2 000+" figure is ~35% high; the doc flags it [VERIFY] and defers to the inventory, which now contradicts it |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "≈45 entries" seed (§5.4) | schemas/settings.toml has 121 | rg count = 121 — doc stale |
| "product.schema.json ... lives in schemas/ next to the file it validates" (§3.1) | it lives at repo root | ls: ./product.schema.json; schemas/ holds settings.toml + commands.schema.json |
| "2 000+ options" (§5.5) | repo baseline parse: 1476 | parse-options.py summary `d13: 1476 d14: 1476` |
| "Status: implemented" (header) | validator notes "parked plane", "no *.toml candidates" for profiles/themes | xtask-config-validator.txt verbatim notes |

## 4. Fabrication / inflation indicators
The doc is unusually honest ([PROPOSAL]/[VERIFY] markers everywhere, count-discrepancy
called out at §5.4, cross-doc mismatches flagged as open questions). The single inflation
is the Status line "implemented" covering planes that were never built. No fabricated
test claims: every test it promises that exists, exists by name.

## 5. Matrix rows
N/A (doc feeds config-validator C4 and feature-exposure rules F1/F2 — matrix coupling
is other agents' rows; the validator's feature-registry intersection is admitted-owed
in feature-matrix.toml header L12-14).

Commands: ls of artifacts (existence checks above); rg counts in schemas/settings.toml,
crates/product-config/tests/merge.rs; sed product.toml [features]/[telemetry]; baseline
citations audit/runs/baseline/{xtask-config-validator.txt,full_gate.txt}.
