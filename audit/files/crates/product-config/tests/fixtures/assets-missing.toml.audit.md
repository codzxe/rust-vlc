# AUDIT: crates/product-config/tests/fixtures/assets-missing.toml
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 226 / 0 (fixture data) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test data). Consumed by `tests/semantic.rs::missing_brand_asset_is_rejected` (semantic.rs:260-270) via `common::load_fixture` + committed product.schema.json. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p product-config --test semantic` -> 17 passed, 0 failed (fresh run, includes this fixture's test); diff vs root product.toml -> exactly one mutation on top of a stale base; sed verification of the mutation line |

## 1. One-paragraph verdict
Correct single-mutation fixture: the only intended difference from its base is `logo_svg` pointing at a nonexistent file, and its test asserts the exact expected diagnostic (key `brand.logo_svg`, rule contains "must exist", found contains "missing.svg") with the `base_dir` SemanticInputs injection. The one defect is inherited staleness: the fixture's base is an old product.toml snapshot, not the current one the README tells maintainers to copy.

## 2. What actually works (verified)
— `cargo test -p product-config --test semantic` -> 17 passed (run fresh in this session; includes `missing_brand_asset_is_rejected`).
— Mutation isolation: diff vs current product.toml shows the intended mutation plus only base-snapshot drift (see F-001); every OTHER field is schema-valid so the semantic asset check fires on `logo_svg` alone.

## 3. Findings
F-001 [S4]
Where: crates/product-config/tests/fixtures/assets-missing.toml:27-29, 66-67 (base drift; mutation at 97)
Code: (verbatim, L97)
```
logo_svg = "assets/brand/missing.svg"
```
What is wrong: the fixture base is an old product.toml snapshot (`display_version = "0.1.0-dev"`, `version_code = 1`, `min_width = 480`, `streaming-http = false`, pre-ADR-001 comments, no `[web.size_budgets]`), while the current root product.toml has `0.1.3` / `4` / `940`/`600` / `true` / post-ADR-001 prose / a web section. The README's regeneration instruction ("re-applying its mutation to a fresh copy of product.toml") would produce a materially different file. Harmless to test outcomes (fixtures validate against the schema, not against product.toml), but the fixtures silently document superseded product decisions (e.g. the stale comment claiming "software-only" is the ONLY legal hw_decode_policy, contradicted by the post-ADR-001 enum fixture in the same directory). What correct looks like: regenerate all fixtures from the current product.toml. Confidence: HIGH — Verification: diff + read. (Full treatment in the README.md audit.)

## 4-9. (short fixture audit)
Per-unit table: single data file; sections identity/window/brand/defaults/features/playback/limits/urls/migrations present and valid except the mutation. Test honesty: `missing_brand_asset_is_rejected` would fail if the asset-existence rule broke, if the fixture's other assets went missing, or if the key path changed — real assertions on rule substring + found value. Dependencies: none. Dead code: none. Open questions: none — see README.md audit for the shared staleness fix.
