# CLAIM AUDIT: docs/reviews/w24-g-fixes-findings.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 5 / 1 / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Every code-shape claim in this fix-review resolves byte-for-byte in the
current tree: derived (not pinned) seed counts, the renamed/stronger CLI
help test, the shape-guard test, the 5 regenerated schema.ts constants, the
CARGO_MANIFEST_DIR re-rooting, and the matrix's OF-5 honesty retraction
(which itself documents a prior false "MF landed" claim that the gate
caught and fixed). The test-run counts are cited runs on a branch — not
re-runnable here (no git, no per-lane worktrees).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Pins are DERIVED … schema.len() vs seed [[setting]] row count … No literals" | :25-31 | REAL | crates/settings-schema/src/lib.rs:155-185 — `schema.len()`, `schema.areas.len()` present; the 27/14 pins absent |
| 2 | "areas_block_lists_the_six_areas renamed to areas_block_lists_every_screen_the_seed_declares" | :33-38 | REAL | apps/cli/src/help.rs:312 carries the new name; old name 0 hits |
| 3 | "seed_shape_is_what_every_committed_artifact_self_reports (integration.rs:150-214)" | :42-44 | REAL | crates/settings-schema/tests/integration.rs:151 exactly |
| 4 | "5 SETTING_EXTENSIONS_CRASH_* constants" in regenerated schema.ts | :61-63 | REAL | `rg -c SETTING_EXTENSIONS_CRASH ui/src/screens/settings/generated/schema.ts` → 5 |
| 5 | repo_root() derives from env!("CARGO_MANIFEST_DIR") | :78-83 | REAL | tests/rebrand/rebrand_skeleton.rs:126-133 verbatim |
| 6 | matrix OF-5: "Media Foundation … NOT IMPLEMENTED — no MF lane exists" | :102-108 | REAL | feature-matrix.toml:416 note contains the retraction + provenance "gate OF-5 honesty fix … the Media Foundation claim was false" |

## 3. Contradictions
None. Note the sequence this doc evidences: the matrix ONCE carried a
false claim (MF landed/compile-verified incl. windows-msvc); gate OF-5
caught it; matrix:416 now records both the correction and the fabrication
history. The fix pipeline works.

## 4. Fabrication / inflation indicators
None in this doc — it records its own infra failures (shared-target clobber
hit twice) and a tautological-assert nit (R5-3) against itself.

Findings: F-001 [S4] test-run counts (43/8/125/12) and the w24/g branch
diff are not re-verifiable from the sandbox (no git) — UNVERIFIED (reason:
branch state not inspectable; baseline workspace tests green is
consistent).
Commands: sed/rg verifications listed above (exit 0 each).
