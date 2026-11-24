# AUDIT: crates/product-config/tests/fixtures/injected-enum-theme.toml
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines | 227 (full product.toml copy, one mutation) |
| File verdict | REAL |
| Reachable | Test fixture — consumed by TWO tests in crates/product-config/tests/semantic.rs: `injected_enum_theme_must_exist_in_themes_dir` (121-133) and `injected_enums_are_deferred_when_no_listing_is_supplied` (161-173); both pass in baseline |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## Verdict
Single-mutation fixture: `theme = "oled-dark"` (line 104) where the base product.toml ships `theme = "dark"` — exercising the schema's empty-`enum: []` injected-enum contract (product.schema.json:153-154: "enum injected at validation time from the themes/ directory listing"). Both consumption modes are tested honestly:
```
theme = "oled-dark"
```
- With a supplied listing `["dark", "light"]`: exactly one diagnostic for `defaults.theme`, rule mentions `themes/<id>.toml`, expected contains "dark, light", found `"oled-dark"` (semantic.rs:129-132).
- With `SemanticInputs::none()`: zero diagnostics for `defaults.theme` — the deferral contract ("the tool supplies listings", doc 07 §3.4 stage 3) is pinned as a deliberate, documented no-op rather than an accidental gap (semantic.rs:162-173).
This pairing is exactly right for the schema's `enum: []` design and would catch both a broken injected-enum check AND an over-eager one that rejects without a listing.

F-001 [S4] Same stale-base issue as the sibling fixtures (0.1.0-dev / version_code 1 / 480x320 base / no `[web.size_budgets]`): the fixture predates the current product.toml. Structural drift is caught loudly by the `semantic_diagnostics` structural gate (semantic.rs:13-19). Note also: the real `themes/` directory has no *.toml candidates at all (xtask-config-validator baseline: "themes/ has no *.toml candidates — enum injection skipped until the plane lands"), so in production the deferral path is the ONLY path that ever runs today — the rejecting path is exercised solely by this fixture's synthetic listing. Confidence: HIGH — Verification: read + diff + baseline xtask-config-validator.txt note.

## Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| semantic.rs:122 injected_enum_theme_must_exist_in_themes_dir | YES | pins rule text, listing rendering, found value |
| semantic.rs:162 injected_enums_are_deferred_when_no_listing_is_supplied | YES | pins the no-listing deferral (would fail if the check rejected unconditionally) |
