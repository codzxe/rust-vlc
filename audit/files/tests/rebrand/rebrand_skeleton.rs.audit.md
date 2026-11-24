# AUDIT: tests/rebrand/rebrand_skeleton.rs
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 643 / 2 structs + 8 fns + 5 tests + 3 ignored placeholders |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test). Wired: tools/xtask/tests/cli.rs:104-105 `#[path = "../../../tests/rebrand/rebrand_skeleton.rs"] mod rebrand_skeleton;` — verified. Executes the real `CARGO_BIN_EXE_xtask rebrand` binary (subprocess, hermetic temp dest). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg wiring + referenced tests + fixture existence; baseline citation (cargo not installed here) |

## 1. One-paragraph verdict
A hostile, well-engineered acceptance test: it spawns the REAL xtask rebrand binary against the committed Nova fixture, asserts the rebranded product.toml/README/tauri.conf/settings-docs content, negative-checks that NO source-identity token survives, byte-compares the reverse-control no-op, and pins error paths (exit 2 + actionable stderr). Counts are DERIVED from source content instead of pinned literals (two historical pin breakages are documented in comments) — exactly the discipline the Devil Audit wants. All 5 tests pass in the orchestrator baseline; the 3 `#[ignore]`d placeholders have empty bodies but each names the real unit-tier test that replaces it, and I verified every named replacement EXISTS and passes.

## 2. What actually works (verified)
- Wiring verified: tools/xtask/tests/cli.rs:104 `#[path = "../../../tests/rebrand/rebrand_skeleton.rs"]` (rg).
- Fixture verified present: tests/rebrand/nova.toml + assets/{icon.ico,icon.icns,icon-512.png,logo.svg} (ls).
- Baseline (cargo-test-workspace.txt:6786-6792): `nova_fixture_rebrand_passes_skeleton_assertions ... ok`, `rebrand_malformed_config_is_an_error ... ok`, `rebrand_missing_config_file_is_an_error ... ok`, `rebrand_missing_identity_fields_is_an_error ... ok`, `reverse_control_real_product_toml_is_a_noop ... ok`; 3 placeholders `ignored` with reasons.
- The #[ignore] pointers are real: tests/rebrand/rebrand_acceptance.rs:104 `fn surface_window_title_resolves_nova`, :162 `surface_about_facts_render_nova`, :181 `surface_cli_identity_resolves_nova` (all `... ok` in baseline :5139-5145); ui/src/screens/settings/areas/updates-about.nova.test.tsx exists (ls).
## 3. Findings
F-001 [S4]
Where: rebrand_skeleton.rs:618-643
Code:
```
#[test]
#[ignore = "boot tier: the unit-tier proof lives in tests/rebrand/rebrand_acceptance.rs::surface_window_title_resolves_nova (card 4.H2); this placeholder keeps the full-desktop boot assertion for the nightly tier — doc 07 §8.3 step 4"]
fn window_title_renders_nova() {
    // FUTURE ASSERTION (doc 07 §8.3 step 4): unit-assert the window-title
    // template with the fixture config loaded — "{media_title} — Nova
    // Player" with a fake {media_title}, and the idle title "Nova Player"
```
What is wrong: three placeholder tests with comment-only bodies never execute (nightly tier not wired — no CI lane runs them; UNVERIFIED whether any nightly job exists). Honest labeling and verified replacement tests keep this at hygiene, not fakeness: the doc claim "steps 3-4 … parked as #[ignore]d placeholder tests" matches reality. Confidence: HIGH — Verification: read + baseline.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Outcome + run_rebrand | 74-108 | REAL | — | spawns real binary; cwd pinned to compiled-in worktree root (shared-target-dir fix documented :126-138) |
| TempDest | 110-124 | REAL | — | unique per pid+tag, cleaned on drop |
| repo_root | 126-143 | REAL | — | CARGO_MANIFEST_DIR/../.. canonicalize; unreachable! carries context |
| read | 145-150 | REAL | — | unreachable! with filename in message |
| report_lines | 152-158 | REAL | — | prefix strip for the touched/copied report |
| pascal_case | 160-172 | REAL | — | mirrors fixture derivation; verified identical semantics to doc 07 §3.3(b) assert at :370-374 |
| nova_fixture_rebrand_passes_skeleton_assertions | 176-495 | REAL | — | 16 positive identity rows + 4 stale-token negatives + schema-shape re-implementation (lengths, charsets, GUID segmentation, derived-id cross-rule) + derived version_code row + derived README rewrite count + exact touched-surface set + 4 copied assets + result totals line derived |
| reverse_control_real_product_toml_is_a_noop | 499-548 | REAL | — | byte-identity of 5 surfaces; honest [VERIFY] conditional on missing brand assets (missing-asset must be REPORTED) |
| rebrand_missing_config_file_is_an_error | 552-567 | REAL | — | exit 2 + "rebrand: " prefix + "cannot read config" |
| rebrand_malformed_config_is_an_error | 569-586 | REAL | — | real malformed TOML written; "malformed at line 1" (1-based line reported) |
| rebrand_missing_identity_fields_is_an_error | 588-607 | REAL | — | all fields reported together |
| temp_dest | 609-614 | REAL | — | manual-cleanup variant for error tests |
| 3 ignored placeholders | 618-643 | S4 | — | see F-001 |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| nova_fixture_rebrand_passes_skeleton_assertions | YES | negative stale-token asserts mean any missed rewrite fails; derived counts mean any silent drop fails; exact touched-set equality means any extra/missing surface fails (mutation reasoning; not executed here — cargo unavailable; baseline ok cited) |
| reverse_control_real_product_toml_is_a_noop | YES | byte-for-byte comparison of 5 dest files vs repo files — any unintended mutation fails |
| 3 error-path tests | YES | exit codes AND stderr substrings pinned |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the file is pulled into the xtask package's cli integration test via #[path]" | rebrand_skeleton.rs:9-10 | REAL | tools/xtask/tests/cli.rs:104-105 |
| "steps 3-4 … parked as #[ignore]d placeholder tests … honest doc comments" | rebrand_skeleton.rs:41-44 | REAL | 3 placeholders carry replacement pointers; all named replacements exist + pass (baseline :5139-5145) |
| "missing brand assets reported, not failing" | rebrand_skeleton.rs:536-540 | REAL-UNPROVEN | code path conditional on missing assets; baseline run passed with the branch taken (config-validator baseline says 4 missing brand assets — consistent) |
## 7. Dead / duplicated / unreachable code
None beyond F-001 placeholders. unreachable! uses are context-bearing guards in test helpers (5 sites).
## 8. Dependencies & flags
Depends on xtask rebrand cmd (tools/xtask/src/cmd/rebrand.rs — 15 unit tests pass in baseline :6688-6702) and product.schema.json shapes re-implemented std-only. No feature flags.
## 9. Open questions for the next agent
- Is any nightly/release lane ever going to run the 3 boot-tier placeholders (full-build rebrand + boot)? Currently no wired consumer — they are dead weight unless the nightly tier lands. UNVERIFIED.
