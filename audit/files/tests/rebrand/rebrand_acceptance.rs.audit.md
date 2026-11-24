# AUDIT: tests/rebrand/rebrand_acceptance.rs
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 315 / 2 helpers + 7 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test). Wired: crates/product-config/tests/rebrand_host.rs:12-13 `#[path = "../../../tests/rebrand/rebrand_acceptance.rs"] mod rebrand_acceptance;` (verified). Baseline runs it: cargo-test-workspace.txt:5136 "Running tests/rebrand_host.rs". |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | cat rebrand_host.rs (wiring); baseline citation; gate-log read docs/gates/logs/rebrand-acceptance-4h2.log |

## 1. One-paragraph verdict
The runtime-plane half of the rebrand proof: the Nova fixture is loaded through the REAL product-config pipeline (committed product.schema.json parse + structural + typed), and every runtime surface (window title, installer identity, About, CLI resolution, logs) is asserted from the loaded config with negative no-Lumen-leak checks. The acceptance report test re-asserts each check in-file and pins the BLOCKED register at exactly 2 — a blocker cannot silently appear or persist. All 7 tests pass in the orchestrator baseline. One defect: the "docs" report row asserts a literal `true`.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:5139-5145: all 7 tests `... ok` (surface_about_facts_render_nova, rebrand_acceptance_report, surface_cli_identity_resolves_nova, surface_fixture_load_nova_through_the_real_pipeline, surface_installer_identity_derives_from_nova_config, surface_log_identity_resolves_nova, surface_window_title_resolves_nova).
- The committed gate log docs/gates/logs/rebrand-acceptance-4h2.log reproduces the exact report format emitted by the code (7 PASS, 2 BLOCKED, 0 FAIL) — the artifact and the code agree.
- BLOCKED register honesty cross-checked: apps/cli help.rs hardcoding claim is the stated CLI blocker (matches W16-A04 knowledge: baseline HEVC smoke shows CLI hardcode issues elsewhere; the [PROPOSAL] marker claim UNVERIFIED in apps/cli/help.rs — next agent can confirm).
## 3. Findings
F-001 [S3]
Where: rebrand_acceptance.rs:265-269 vs :218-221
Code:
```
        (
            "docs",
            true, // evidence lives in the xtask-hosted test below
            "build-time plane: tools/xtask tests/cli.rs asserts dest README/settings-docs/settings-help + tauri.conf.json carry Nova, zero Lumen".to_string(),
        ),
```
What is wrong: the module docstring (:220-221) claims "Every PASS line is backed by a real assertion above (and re-asserted here)" — the docs row's `ok` value is the literal `true`, not an assertion. The report will print `docs ... PASS` even if the xtask-side docs assertions regress; only that test failing elsewhere would catch it (it does exist — baseline :6788 nova_fixture_rebrand... ok — so CI catches it, but THIS file's claim is false and the report row is unbacked in-file). Why it matters: the acceptance report is a gate artifact; a PASS row without an assertion is a fabricated-green vector if the referenced test is ever removed. What correct looks like: read the dest settings-docs/README bytes (or invoke the xtask test) so the row is re-asserted, or mark the row as delegated evidence in the status vocabulary. Confidence: HIGH — Verification: read.
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| nova | 62-69 | REAL | — | include_str! of committed schema + fixture through real load_str; unreachable! renders full diagnostics |
| surface_fixture_load_nova_through_the_real_pipeline | 76-98 | REAL | — | 6 typed field equalities |
| surface_window_title_resolves_nova | 103-123 | REAL | — | playing + idle titles + no-Lumen leak loop |
| surface_installer_identity_derives_from_nova_config | 131-155 | REAL | — | product name + desktop bundle + 3 per-OS ids |
| surface_about_facts_render_nova | 161-175 | REAL | — | name/version/publisher/logo path |
| surface_cli_identity_resolves_nova | 180-193 | REAL | — | version banner + usage header (resolution plane; host wiring honestly BLOCKED) |
| surface_log_identity_resolves_nova | 198-211 | REAL | — | prefix "nova" + startup line with ids |
| rebrand_acceptance_report | 223-315 | REAL | S3 | re-asserts 6 of 7 checks; `docs` row is literal true (F-001); blocker register pinned at exactly 2 and count 7 |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 6 surface tests | YES | value equalities on real resolver outputs through the committed schema; leak loop on window title |
| rebrand_acceptance_report | YES (except docs row) | re-asserts each check inline; blocker-set equality fails on any register change; the docs row cannot fail in-file (F-001) |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every PASS line is backed by a real assertion above (and re-asserted here)" | rebrand_acceptance.rs:220-221 | FALSE | docs row `true` literal (:267) |
| "BLOCKED surfaces … asserted the blocked set matches the known blockers exactly" | :34-37 | REAL | assert_eq!(blocked.len(), 2) at :306-310 |
| "run with cargo test -p product-config --test rebrand_host" | :12-14 | REAL | baseline :5136 shows exactly that test binary running |
## 7. Dead / duplicated / unreachable code
None. The two constant blocks duplicate NOVA_* constants from rebrand_skeleton.rs (both files assert, not trust, the fixture — deliberate cross-checking duplication, not drift risk since both fail loudly on fixture change).
## 8. Dependencies & flags
product-config crate (typed model + runtime resolvers); committed product.schema.json via include_str!. No feature flags.
## 9. Open questions for the next agent
- Verify apps/cli help.rs still hardcodes its banner (the BLOCKED(wiring) reason). If it was rewired since, the pinned blocker register fails loudly — trust the pin, but confirm the reason text matches current reality.
