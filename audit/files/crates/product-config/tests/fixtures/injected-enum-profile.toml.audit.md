# AUDIT: crates/product-config/tests/fixtures/injected-enum-profile.toml
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines | 203 (full product.toml fork) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/semantic.rs:150 (`injected_enum_profile_must_exist_in_profiles_dir`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | diff product.toml vs fixture (mutation: `profile = "cinema"` at :109-110); sed test body verification |

## 1. Verdict
Honest fixture. One intentional mutation on a product.toml fork:
`profile = "cinema"` (product.toml:113 has `profile = "default"`). The test
(semantic.rs:148-160) supplies a profiles listing of exactly `["default"]`,
asserts exactly one `defaults.profile` diagnostic whose rule mentions
`profiles/<id>.toml` and `found` is `"cinema"`, and the helper first proves
the fixture passes structural validation (semantic.rs:12-22). Mutation
reasoning: if `injected_enums` stopped checking, `one()` gets 0 hits →
fails. Passed in baseline. Companion fixtures injected-enum-theme/locale
cover the other two injected enums plus the deferred-when-no-listing case
(semantic.rs:162-176) — the negative control family is complete.

## 2. Findings
F-001 [S4]
What is wrong: same stale-fork drift as its siblings (display_version
"0.1.0-dev", min sizes 480/320, no [web.size_budgets]). Benign for the same
reasons (values remain valid and non-interfering).
Confidence: HIGH — Verification: diff.
