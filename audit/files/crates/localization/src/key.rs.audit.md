# AUDIT: crates/localization/src/key.rs
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-21 |
| Lines / functions | 250 / 17 (incl. 5 in-file tests + 1 doc-test) |
| File verdict | UNREACHABLE (correct, well-tested newtype; the crate it lives in has zero workspace dependents) |
| Reachable from shipped entry point? | NO — proof below |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg -n "localization" --glob '**/Cargo.toml'` (no dependent anywhere); `rg -n "use localization::\|localization::MessageKey" --glob '!audit/**'` (only the crate's own src/tests); `rg -n "MessageKey::new\|MessageKey \{" crates/localization/src` (construction sites); read ui/scripts/l10n-check.mjs, ui/src/screens/settings/areas/language/locales.ts, crates/localization/src/lib.rs, src/catalog.rs (grep), locales/en-US.toml header; `sed -n` on all quoted ranges |

## 1. One-paragraph verdict
`key.rs` is the cleanest kind of code in this audit: a small validated newtype whose constructor, accessors, and error type all do exactly what they claim, with mutation-sensitive tests pinned to exact error values. The problem is what surrounds it. The `localization` crate has **zero dependents** — no `Cargo.toml` in the repo declares it (rg over every manifest; workspace members are `crates/*, apps/*, tools/xtask, tests/integration/first-vertical, tools/mem-audit, tools/sweep`) — so the "deny by construction" guarantee in the module doc guards an API that neither `lumen` CLI nor `lumen-tauri` nor any other crate ever links. The strings users actually see flow through a separate TypeScript stack (`ui/src/i18n/messages.ts` + `ui/scripts/l10n-check.mjs`, baseline #24), which reimplements the same doc-07 §6.1 grammar independently; `locales/en-US.toml` — whose header says its grammar is "enforced by `localization::MessageKey`" — is parsed only by this crate's own CI tests (baseline #3). The declared "stable address of every user-visible string" is a mechanism without a consumer.

## 2. What actually works (verified)
— verified by `sed -n` full trace + baseline #3 (`audit/runs/baseline/test-workspace.log.txt`, 1681 passed includes this crate's suite): `MessageKey::new` correctly rejects empty, 1-segment, 6-segment, uppercase, underscore, empty-segment (`a..b`, `.a.b`, `a.b.`), space, and non-ASCII input with typed errors; `accepts_valid_keys`/`rejects_invalid_keys_with_typed_errors` assert exact `KeyError` values per case.
— verified by construction-site grep: `MessageKey` has a single constructor (`new`); the `raw` field is private; no `MessageKey { raw: .. }` literal exists outside it, so the `namespace()` invariant (non-empty first segment) cannot be violated.

## 3. Findings

### F-001 [S2]
Where: crates/localization/src/key.rs:4-7
Code:
```
//! A key is 2–5 dotted segments, each `[a-z0-9-]+`. Because
//! [`MessageKey`] is a validated newtype and the translation API accepts
//! nothing else, an invalid or raw string can never reach a catalog lookup
//! (doc 07 §6.5 "deny by construction").
```
What is wrong: The guarantee is true inside a crate nothing ships. `rg -n "localization" --glob '**/Cargo.toml'` matches only the crate's own manifest and a comment in `crates/playlists/Cargo.toml`; no app, no engine crate, no tool depends on `localization`. The UI's real l10n pipeline is TypeScript: `ui/scripts/l10n-check.mjs` states it is a "Zero-dependency audit of the UI message catalog (ui/src/i18n/messages.ts)" and reimplements §6.1 ("keys follow doc 07 §6.1: 2–5 dotted kebab-case segments") in JS. `locales/en-US.toml:20` claims its grammar is "enforced by `localization::MessageKey`" — enforced only by `crates/localization/tests/seed_integrity.rs` in CI; no runtime path reads it. The UI language picker even hardcodes the locale list (`AVAILABLE_LOCALES: readonly string[] = ["en-US"]`, ui/src/screens/settings/areas/language/locales.ts:14) while its comment says the crate "resolves catalogs by scanning the repo `locales/` directory".
Why it matters: This is the config-system's declared-vs-consumed disease in pure form: one grammar type-checked in Rust and wired to nothing, one hand-rolled in JS serving all users. They can drift silently (the TS check has no cross-check against `locales/en-US.toml`, and the crate's rule-4 audit never runs in production tooling — see my refs.rs audit).
What correct looks like: either the UI consumes the crate's catalogs (generated `messages.ts` from `locales/*.toml`, one grammar) or the Rust crate is deleted/demoted to a CI-only checker that audits the TS catalog as data.
Confidence: HIGH — Verification: read + dependents grep + baseline #3/#24.

### F-002 [S3]
Where: crates/localization/src/key.rs:15-19
Code:
```
/// Maximum number of dotted segments in a key (doc 07 §6.1: "two to five
/// segments"). NOTE: the `msgid` pattern in doc 07 §3.2
/// (`^[a-z0-9-]+(\.[a-z0-9-]+)+$`) has no upper bound — the prose grammar
/// here is stricter; reconciling the two is a Lead change-control item.
pub const MAX_SEGMENTS: usize = 5;
```
What is wrong: The msgid grammar exists in two divergent forms: `product.schema.json`'s pattern (doc 07 §3.2, no segment cap) and this crate (cap 5). The code admits the drift and parks it on "a Lead change-control item" that has not happened. Consequence today: a 6-segment `identity.description_key` in `product.toml` passes the schema pattern but can never resolve — `catalog.rs:109` parses every catalog key through `MessageKey::new`, so the reference catalog cannot contain it, and `product-config`'s `catalog_checks` (which would flag the unresolved ref) never runs with real inputs (my semantic.rs audit F-001).
Why it matters: Two validators for the same identifier class with different acceptance sets is exactly how "declared but unresolvable" config slips through. The mismatch is documented, which is better than hidden, but documented drift is still drift.
What correct looks like: one grammar constant shared by the schema generator and this crate (schema pattern gains `{0,4}` repetition or the crate drops the cap), whichever the Lead decides — then delete the NOTE.
Confidence: HIGH — Verification: read + prior `audit/files/product.schema.json.audit.md` pattern row + catalog.rs:109 traced.

### F-003 [S4]
Where: crates/localization/src/key.rs:48-53
Code:
```
fn is_valid_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
```
What is wrong: The charset admits leading/trailing/consecutive hyphens, so `player.-.play`, `a.b-`, and `-.--` are "valid" keys, while the `InvalidSegment` error text and the doc comment promise "kebab-case" (`#[error("segment {segment:?} must match [a-z0-9-]+ (kebab-case, ...)")]`, key.rs:44). The regex `[a-z0-9-]+` is faithfully implemented — the lie is calling that kebab-case.
Why it matters: Cosmetic grammar looseness in an API whose entire selling point is a tight grammar; a junk key like `settings.-.label` would be accepted and then live forever (keys are API, renaming is a breaking change).
What correct looks like: reject segments starting or ending with `-` (and `--` runs) if "kebab-case" is the contract, or stop calling it kebab-case.
Confidence: HIGH — Verification: read; test suite has no case for hyphen-edge segments (tests cover empty/uppercase/underscore/space/non-ASCII only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MIN_SEGMENTS` | 11-13 | REAL | — | Const 2; matches doc 07 §6.1 |
| `MAX_SEGMENTS` | 15-19 | REAL | S3 (F-002) | Const 5; honest drift NOTE vs §3.2 pattern |
| `RESERVED_NAMESPACES` | 21-31 | REAL | — | 8 namespaces matching doc 07 §6.1 table; used only by `in_reserved_namespace` (informational) |
| `KeyError` (enum, 3 variants) | 34-46 | REAL | — | thiserror Display; const interpolation compiles (baseline #1 build clean); all variants carry the offending data |
| `is_valid_segment` | 48-53 | REAL | S4 (F-003) | Byte-wise; correctly rejects non-ASCII (é → 0xC3… fails); hyphen-edge looseness |
| `MessageKey` (struct) | 73-76 | REAL | — | Private `raw`; single-constructor invariant verified by grep |
| `MessageKey::new` | 80-100 | REAL | — | Empty → count → per-segment checks; no panics, no allocation beyond the final `to_string` |
| `MessageKey::as_str` | 103-105 | REAL | — | Borrow only |
| `MessageKey::namespace` | 109-113 | REAL | — | `unwrap_or_default()` safe under constructor invariant (documented inline); never `""` in practice |
| `MessageKey::segment_count` | 116-118 | REAL | — | Re-splits per call; O(len), fine |
| `MessageKey::in_reserved_namespace` | 123-125 | REAL | — | Informational by design (test asserts `xtask` is not reserved); honest doc |
| `impl Display` | 128-132 | REAL | — | Writes raw |
| `impl AsRef<str>` | 134-138 | REAL | — | Borrow only |
| tests: `accepts_valid_keys` | 144-161 | REAL | — | 9 concrete positives incl. `xtask.*` non-reserved ns |
| tests: `rejects_invalid_keys_with_typed_errors` | 163-216 | REAL | — | 10 cases pinned to exact `KeyError` values — mutation kills |
| tests: `namespace_and_segments` / `reserved_namespace_table_matches_doc07` / `segment_charset` | 218-249 | REAL | — | Exact assertions; the table test even pins the const order against doc 07 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| accepts_valid_keys | YES | Exact `as_str` round-trip per key |
| rejects_invalid_keys_with_typed_errors | YES | `assert_eq!(got.as_ref(), Some(expected))` per case — any weakened check fails |
| namespace_and_segments | YES | Pins namespace/count/reserved-ness of 2 keys |
| reserved_namespace_table_matches_doc07 | YES | Const array equality — reordering/editing fails |
| segment_charset | YES | 7 assertions incl. non-ASCII negative |
| doc-test (lines 59-71) | YES | Compiled+run by `cargo test` (baseline #3); asserts valid parse + 3 rejections |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "an invalid or raw string can never reach a catalog lookup" | key.rs:6-7 (module doc, doc 07 §6.5) | SHALLOW | True of the crate API; the API is linked by zero shipped code — the real UI lookups go through TS `messages.ts` (F-001) |
| "KEY GRAMMAR (doc 07 §6.1) — enforced by `localization::MessageKey`" | locales/en-US.toml:20 | SHALLOW | Enforced only by the crate's CI tests over the seed file; no runtime consumer |
| "A message key is 2–5 dotted segments" | doc 07 §6.1 / lib.rs | REAL | Implemented + tested (this file) |
| msgid pattern `^[a-z0-9-]+(\.[a-z0-9-]+)+$` | doc 07 §3.2 / product.schema.json | REAL but DIVERGENT from §6.1 | F-002 — two acceptance sets for the same identifier class |

## 7. Dead / duplicated / unreachable code
- Entire file (and crate): unreachable from any shipped entry point — zero dependents (F-001).
- Duplication: §6.1 grammar re-implemented in `ui/scripts/l10n-check.mjs` rule 4 (independent TS reimplementation, no shared source of truth).

## 8. Dependencies & flags
- `std` + `thiserror` only (crate has no internal deps, doc 01 §2.3 L1). No feature flags. Nothing in the workspace opts in, so no build even compiles it into a shipped binary — it exists solely as a test-checked library.

## 9. Open questions for the next agent
- catalog.rs / audit.rs / fallback.rs / locale.rs owner (W11 sibling): confirm the same zero-dependent status applies to the audit entry points (`audit_catalogs_with_refs`) — my refs.rs audit shows the rule-4 reference audit runs only in crate tests.
- Lead item parked at key.rs:18: which grammar wins (pattern vs 5-segment cap)?
- UI i18n owner: `AVAILABLE_LOCALES` hardcodes `["en-US"]` (locales.ts:14) — is any code path planned to generate it from `locales/`, or is the crate's scan dead forever?
