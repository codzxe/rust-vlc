# AUDIT: crates/product-config/tests/fixtures/injected-enum-locale.toml
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 226 / 0 (fixture data) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test data). Consumed by `tests/semantic.rs::injected_enum_locale_must_exist_in_locales_dir` (semantic.rs:132-142) with `SemanticInputs { locales: ["en-US"] }`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p product-config --test semantic` -> 17 passed, 0 failed (incl. this fixture's test); diff vs root product.toml; sed verification |

## 1. One-paragraph verdict
Correct fixture for the injected-enum rule: `defaults.locale = "xx-XX"` is deliberately VALID at every earlier stage (it passes the BCP-47 pattern — `xx` is a well-formed 2-letter primary subtag — and the schema cannot know the locales/ listing), so only the tool-injected semantic check can catch it. The test pins key `defaults.locale`, rule containing "locales/<tag>.toml", found `"xx-XX"`. This is exactly the "empty enum in schema, injected at validation time" design the product.schema.json audit noted. Only defect: the shared stale base (S4, see README audit).

## 2. What actually works (verified)
— `cargo test -p product-config --test semantic` -> 17 passed (fresh run).
— Stage isolation verified by reasoning + pattern check: `"xx-XX"` satisfies `^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$` (is_xx_upper ok), so structural passes and the semantic stage is the only rejector — the fixture tests precisely the rule it names, with the negative control (`injected_enums_are_deferred_when_no_listing_is_supplied`, semantic.rs:166-176) proving the check stays silent without a listing.
— Mutation (verbatim, L107-108):
```
locale = "xx-XX"
```

## 3. Findings
F-001 [S4] Stale base snapshot (display_version 0.1.0-dev / version_code 1 / min 480x320 / streaming-http=false / pre-ADR-001 comments / no [web]) — identical to F-001 in the assets-missing audit; see crates/product-config/tests/fixtures/README.md.audit.md for the full treatment. Confidence: HIGH — Verification: diff.

## 4-9. (short fixture audit)
Test honesty: `injected_enum_locale_must_exist_in_locales_dir` fails if the injected-enum rule breaks, if the diagnostic key/rule text changes, or if the found value stops being echoed. The deferred-when-no-listing negative control (semantic.rs:166-176) proves the fixture does not smuggle a structural rejection. No dead code, no deps. Open questions: none.
