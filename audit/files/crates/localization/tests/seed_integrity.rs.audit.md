# AUDIT: crates/localization/tests/seed_integrity.rs
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines / functions | 60 / 1 helper + 3 tests |
| File verdict | REAL (but its enforcement scope is 1/7th of what doc 07 §6.4 rule 4 documents) |
| Reachable from shipped entry point? | N/A (test file). The crate + the `locales/` data it audits are linked by no shipped binary — see fallback.rs audit F-001 and audit/files/locales/en-US.toml.audit.md (W11-A08). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | baseline citation cargo-test-workspace.txt:2958-2965 (3/3 pass); rg of the 4 pinned seed keys (consumers: catalog + this test only); rg settings.* in locales/en-US.toml (0) vs ui/src/i18n/messages.ts (1070); sed quote verification; cross-read of audit/files/locales/en-US.toml.audit.md |

## 1. One-paragraph verdict
Three honest negative-control tests over the real repo state: the shipped `locales/` tree passes all five catalog-audit rules, the real product.toml's msgid refs (pinned at exactly 2) resolve in the reference catalog, and four representative seed keys exist. All pass in the baseline. The header is unusually candid that this suite substitutes for the non-existent `cargo xtask l10n-audit` runner (card 1.E2). The hostile findings are about provenance and scope: (a) "this suite is the enforcement" covers only ONE of the seven msgid sources doc 07 §6.4 rule 4 names — product.toml — because the reference catalog deliberately contains no settings.*/player.*/menu.* keys, so the 121 live settings msgids in schemas/settings.toml (printed by `lumen --help`, rendered by the UI) are joined against a DIFFERENT catalog (ui/src/i18n/messages.ts) by a DIFFERENT checker (ui/scripts/l10n-check.mjs) with zero mechanical link to this one; (b) the seed-presence test is a presence-only change-detector that cannot see that its own seeds' documented provenance has rotted (gate.rs label drift and the ci.yml stub-job fossil, per W11-A08's audit of locales/en-US.toml).

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:2958-2965: `phase1_seed_keys_are_present`, `shipped_locales_tree_passes_every_catalog_audit_rule`, `product_toml_msgids_resolve_in_the_reference_catalog` — 3/3 ok.
— Mutation reasoning: delete a seed key from locales/en-US.toml -> test 3 fails naming the key; point product.toml's `description_key` at a nonexistent msgid -> test 2 fails (refs join via audit_catalogs_with_refs); corrupt a placeholder shape in en-US.toml -> test 1 fails (rule 3). These tests bite within their scope.

## 3. Findings
F-001 [S3]
Where: crates/localization/tests/seed_integrity.rs:7-10 (scope claim) and 32-42 (the join)
Code:
```
//! The violation fixtures proving that the rules *fire* live in
//! `tests/audit_proof_matrix.rs`; this file is the negative control over
//! the repository itself. CI wiring (`cargo xtask l10n-audit`, doc 07
//! §6.4) is card 1.E2 — until then this suite is the enforcement.
```
What is wrong: "the negative control over the repository itself" + "this suite is the enforcement" describes doc 07 §6.4's contract, but rule 4 as documented joins SEVEN source families ("product.toml, schemas/settings.toml, layouts/*, keymaps/*, profiles/*, themes/* and doc 05's menu/UI-action tables") against the reference catalog. This suite joins exactly one: product.toml, 2 refs (seed_integrity.rs:37-39). The other joins are impossible with the current seed: locales/en-US.toml contains zero settings.*/player.*/menu.* keys by its own Phase-1 scope note ("NO player.*/settings.*/menu.* UI keys are seeded"), while schemas/settings.toml carries 121 live description msgids and ui/src/i18n/messages.ts carries 1070 `settings.*` keys — the real join for those lives in the TS plane (ui/scripts/l10n-check.mjs), unsynced with this one (W11-A08 established the zero-overlap split). So the enforcement that "is" this suite covers 2 msgids of the product's actual localized surface; a settings msgid that exists in schemas/settings.toml but vanishes from messages.ts is caught by the OTHER checker, and nothing catches the two planes drifting apart. Confidence: HIGH — Verification: read + rg (settings.* counts: locales/en-US.toml 0, messages.ts 1070).
F-002 [S4]
Where: crates/localization/tests/seed_integrity.rs:45-58
Code:
```
    for raw in [
        "product.description",
        "window.title.idle",
        "xtask.gate.status.pass",
        "ci.job.stub-suffix",
    ] {
        let key = MessageKey::new(raw)?;
        assert!(catalog.contains_key(&key), "missing seed key {raw}");
    }
```
What is wrong: presence-only change-detector for 4 cherry-picked keys. It validates existence, not provenance: it cannot detect that `ci.job.stub-suffix`'s documented source (.github/workflows/ci.yml stub jobs) no longer exists (W11-A08 F-002: `rg 'STUB' .github/` exit 1), nor that the `xtask.gate.result.*` templates were normalized rather than "harvested verbatim" (W11-A08 F-003), nor that 14/16 keys have no consumer. The test pins the seed's existence while the seed's justification rots silently. Confidence: HIGH — Verification: read + cross-cited audit.
F-003 [S4]
Where: crates/localization/tests/seed_integrity.rs:26-29
Code:
```
    let issues = audit_catalogs(&repo("locales"));
    assert!(issues.is_empty(), "issues: {issues:?}");
```
What is wrong: today `locales/` contains only the reference catalog, so the cross-catalog rules (1: every catalog ⊇ reference key set; 2: orphan keys; 3: placeholder equality vs reference) compare en-US with itself — vacuous until a second catalog lands (ls locales/: en-US.toml only). The intra-file rules still bite. The test name ("every catalog audit rule") promises more than one catalog can exercise. Confidence: HIGH — Verification: read + ls.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| repo | 19-23 | REAL | - | MANIFEST_DIR/../.. |
| shipped_locales_tree_passes_every_catalog_audit_rule | 26-29 | REAL | S4 | honest; vacuous cross-catalog half today (F-003) |
| product_toml_msgids_resolve_in_the_reference_catalog | 32-42 | REAL | - | pins ref count 2 (tripwire for new msgid keys); `unwrap_or_default` on None fails loudly at the len assert |
| phase1_seed_keys_are_present | 45-58 | REAL | S4 | presence-only (F-002); also asserts catalog locale == REFERENCE_LOCALE (47) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| shipped_locales_tree... (26) | YES (intra-file rules) | grammar/placeholder/plural rules run over the real file |
| product_toml_msgids... (32) | YES | real product.toml refs joined against real catalog; count pinned |
| phase1_seed_keys_are_present (45) | YES (change-detector) | 4 keys pinned; detects seed shrinkage, not staleness (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this file is the negative control over the repository itself" | seed_integrity.rs:8-9 | SHALLOW | controls 2 msgid refs of the documented 7-source rule-4 join (F-001) |
| "CI wiring (`cargo xtask l10n-audit`...) is card 1.E2 — until then this suite is the enforcement" | seed_integrity.rs:9-10 | REAL | candid: the runner does not exist (also filed as fallback.rs F-002); enforcement = this suite + ui/scripts/l10n-check.mjs (undocumented half) |
| doc 07 §6.4 rule 4 sources list | docs/07:679-681 | SHALLOW | only product.toml joined Rust-side (F-001) |

## 7. Dead / duplicated / unreachable code
- The suite audits a data plane no shipped binary reads (crate island — see fallback.rs audit). 14/16 catalog keys unconsumed (W11-A08 §7).

## 8. Dependencies & flags
localization::{audit, catalog, key, refs}; std. No features.

## 9. Open questions for the next agent
- When card 1.E2 (l10n-audit xtask wiring) lands, will it scan the TS catalog too, or keep the two planes formally separate? Today `npm run l10n:check` (baseline full_gate "ui" sections) is the only gate over the 1460-key plane.
- `extract_msgid_refs_from_file` recognizes `description_key`/`title_idle_key` (refs.rs:33) — if product.toml grows a third msgid-bearing key, both this test AND the extractor's hard-coded key list need edits (two-place pin).
