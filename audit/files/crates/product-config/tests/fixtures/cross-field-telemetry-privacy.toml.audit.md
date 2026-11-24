# AUDIT: crates/product-config/tests/fixtures/cross-field-telemetry-privacy.toml
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 225 / n/a (test fixture) |
| File verdict | REAL (stale copy) |
| Reachable from shipped entry point? | NO — test input only; consumed by tests/semantic.rs:67-74 `cross_field_telemetry_requires_privacy_url`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `diff product.toml <fixture>`; rg marker lines + [urls] section; baseline cargo-test-workspace.txt (semantic suite 17 passed, exit 0). |

## 1. One-paragraph verdict
Real negative fixture exercising the privacy cross-field rule (doc 07 §3.3 / MP §12): `telemetry.enabled = true` (line 121) with `urls.privacy` deleted from the `[urls]` table — the exact combination semantic.rs:274-282 rejects. Two-part mutation correctly documented in the README and asserted exactly by the test. Same stale-copy drift as the rest of the fixture set.

## 2. What actually works (verified)
— verified by baseline semantic-suite run (17 passed, exit 0): one diagnostic at `telemetry.enabled`, rule contains "urls.privacy", hint contains "privacy policy".
— both mutation halves confirmed by sed/rg: `enabled = true` at :121; `[urls]` (lines 207-213) carries help/support/update_feed and NO privacy key (root product.toml has `privacy = "https://lumen-player.example.org/privacy"`).

## 3. Findings
F-001 [S4]
Where: cross-field-telemetry-privacy.toml:27,29,66-67,154 vs repo-root product.toml
Code:
```
display_version = "0.1.0-dev"
```
```
min_width = 480               # schema: min_width < width
```
```
streaming-http = false          # Phase 6
```
What is wrong: Same undocumented drift as the rest of the set (README:5-6 "exactly one documented mutation" — false): version 0.1.0-dev/1 vs 0.1.3/4, min sizes 480/320 vs 940/600 (w28), streaming-http=false vs true (w31.3), pre-ADR-001 comments, `[web.size_budgets]` missing (7.K1). Today harmless; latent spurious-failure source. Confidence: HIGH — Verification: ran (diff).
Mutations verified: `enabled = true` at :121 + privacy URL removed — matches README:54.

## 4. Per-function table
N/A (fixture).
## 5. Test honesty
`cross_field_telemetry_requires_privacy_url` (semantic.rs:67-74): YES — asserts key path, rule text and hint; would fail if the cross-field rule were removed.
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "exactly one documented mutation" | fixtures/README.md:5-6 | FALSE (drift) | diff vs product.toml |
| Mutation `telemetry.enabled = true`, `urls.privacy` removed | fixtures/README.md:54 | REAL | fixture:121 + missing privacy key + rejecting test |
| telemetry opt-in requires privacy URL (MP §12) | doc 07 §3.3 | REAL | semantic.rs:274-282 + this fixture's rejection |
## 7. Dead / duplicated / unreachable code
See fixture-set note in enum-crash-reports.toml.audit.md F-001.
## 8. Dependencies & flags
None (TOML test data).
## 9. Open questions for the next agent
Should crash_reports != "off" also require the privacy URL? semantic.rs:274 gates ONLY on `telemetry.enabled`; a product with enabled=false + crash_reports="send" ships crash data with no privacy policy — no rule covers that today (observed while auditing this fixture; semantic.rs is another agent's file).
