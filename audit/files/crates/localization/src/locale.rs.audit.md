# AUDIT: crates/localization/src/locale.rs
| Field | Value |
|---|---|
| Auditor | W11-A09 |
| Date | 2026-11-23 |
| Lines / functions | 222 / 4 methods + 2 trait impls (+3 tests, +1 doc-test) |
| File verdict | REAL (crate itself is UNREACHABLE from shipped entry points — see fallback.rs audit F-001; same island status applies) |
| Reachable from shipped entry point? | NO — no package depends on the `localization` crate (manifest scan, see fallback.rs audit). Reached only by the crate's own tests (baseline: locale tests + 2 doc-tests pass, cargo-test-workspace.txt:2902-2904 area). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | baseline citation (3 locale::tests + 2 doc-tests pass); sed quote verification; read of catalog.rs REFERENCE_LOCALE constant |

## 1. One-paragraph verdict
A small, careful well-formedness checker: 2–3-letter ASCII language, hyphen-separated 1–8-char alphanumeric subtags, no empty subtags, `en_US` explicitly caught and re-reported through the rule whose message tells the user to use hyphens (good error design), canonical casing (language lowercase, 4-letter scripts Titlecase, numeric regions preserved, variants lowercase). The rejection table tests exact error variants, and the canonicalization table covers 8 shapes. The one real defect is in the casing rule itself: every 2–3-letter alphabetic subtag is unconditionally uppercased as a "region", but in BCP-47 that position can hold an extlang (`zh-yue`) or an extension singleton + subtag (`-u-ca-`), which must stay lowercase — the canonicalizer mangles them. The header honestly defers full registry validation, but this is not a missing validation, it is a wrong transformation.

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt (localization package, exit 0): `locale::tests::{parses_and_canonicalizes, rejects_malformed_tags, language_primary_parent}` pass; `Doc-tests localization: 2 passed` (the struct-level doc example at lines 38-47 executes).
— The `en_US` interception (lines 77-84) reports the underscore error with the hyphen hint: tested case at lines 190-195.

## 3. Findings
F-001 [S3]
Where: crates/localization/src/locale.rs:109-117
Code:
```
            } else if (subtag.len() == 2 || subtag.len() == 3)
                && subtag.chars().all(|c| c.is_ascii_alphabetic())
            {
                // Region subtag: uppercase (US, BRA, 419 stays as-is below).
                canonical.push_str(&subtag.to_ascii_uppercase());
            } else {
                // Variants, numeric regions (419), extensions: lowercase.
                canonical.push_str(&subtag.to_ascii_lowercase());
            }
```
What is wrong: every 2–3-letter alphabetic subtag is uppercased as a region. Per BCP-47 (RFC 5646 §2.1/B): regions are 2-letter alpha or 3-DIGIT — a 3-letter alphabetic subtag after the language is an extlang and stays lowercase (`zh-yue`); extension singletons (`u`, `x`, 1 letter) and their 2–4-char subtags also stay lowercase (`en-US-u-ca-buddhist`). This canonicalizer produces `zh-YUE` and `en-US-U-CA-buddhist`. The module doc (lines 6-7) states the same wrong rule ("2–3-letter regions uppercase"), so the code faithfully implements a mis-specified doc — the doc is the origin of the bug. Why it matters: catalogs are looked up by canonical tag as file name (doc 07 §6.3 "file name = BCP-47 tag"); a `zh-yue.toml` catalog can never be matched by a `zh-YUE` runtime tag — the user silently falls to the `zh` parent and loses Cantonese-specific translations. Also the even-length comment claims "BRA" as a region example — 3-letter regions do not exist in BCP-47 (only 3-digit, e.g. 419, which the else-branch handles). What correct looks like: uppercase only 2-letter alpha subtags (and only in region position); leave 3-letter alpha lowercase. Confidence: HIGH — Verification: read (RFC knowledge; no runtime consumer exists to demonstrate — crate is an island).
F-002 [S4]
Where: crates/localization/src/locale.rs:129-137
Code:
```
        let primary = self.tag.split('-').next().unwrap_or_default().to_string();
        Locale { tag: primary }
```
What is wrong: `unwrap_or_default()` on the impossible-empty case would silently construct a `Locale { tag: "" }` — an invalid invariant — instead of panicking or being unreachable!(). The comment calls the case impossible; if it ever happened, the invalid tag would flow into the fallback chain as a phantom preference instead of failing loudly. Defensive-code nit. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Locale::reference | 60-64 | REAL | - | mirrors catalog::REFERENCE_LOCALE ("en-US") — single source documented |
| Locale::parse | 67-120 | REAL | S3 | well-formedness solid; casing rule wrong for extlang/extension subtags (F-001) |
| Locale::as_str | 123-125 | REAL | - | |
| Locale::language_primary | 129-137 | REAL | S4 | correct parent; silent-default on "impossible" case (F-002) |
| Display / AsRef | 140-150 | REAL | - | trivial |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_and_canonicalizes (157) | YES | 8-case table incl. EN-us, zh-hans-cn, es-419, en-us-posix |
| rejects_malformed_tags (175) | YES | 7-case table asserting the exact error VARIANT per case |
| language_primary_parent (212) | YES | 4 cases incl. zh-Hans-CN → zh |
| doc-test (38-47) | YES | executes parse + both error paths |
Gap: no test input contains an extlang or extension subtag — the F-001 mis-casing is untested territory (zh-yue would "pass" as zh-YUE).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "canonicalized to standard casing (language lowercase, 4-letter scripts Titlecase, 2–3-letter regions uppercase)" | locale.rs:6-8 | FALSE in part | 3-letter alphabetic subtags are extlangs (lowercase per RFC 5646), not regions; code implements the doc's wrong rule (F-001) |
| "Full registry validation ... is deferred to the Phase-4 i18n engine decision [VERIFY]" | locale.rs:8-10 | REAL | honest scoping; only shape validation present |

## 7. Dead / duplicated / unreachable code
- Whole crate island status (see fallback.rs audit): no dependents.

## 8. Dependencies & flags
std + thiserror only. No features.

## 9. Open questions for the next agent
- The bcp47 PATTERN in product.schema.json (`^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$`) matches the same shape — consistent; the schema's hand-written matcher `is_bcp47` (patterns.rs:34, 81+) presumably reimplements this logic a second time — cross-check its casing handling in the product-config audit (it validates but does not canonicalize, so it may be unaffected).
