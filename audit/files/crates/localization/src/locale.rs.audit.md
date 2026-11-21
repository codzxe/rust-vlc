# AUDIT: crates/localization/src/locale.rs
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-21 |
| Lines / functions | 222 / 12 (8 production items + 3 tests + 1 doctest) |
| File verdict | UNREACHABLE (correct, tested code in a crate with ZERO dependents — nothing in the workspace links it) |
| Reachable from shipped entry point? | NO. `rg -n "^localization\s*=" --glob Cargo.toml` over the whole repo → NO manifest declares a dependency on the localization crate (the only "localization" hits in other Cargo.tomls are comments). Neither `lumen` CLI, nor desktop/desktop-tauri, nor control-server, nor xtask, nor any crate depends on it. It compiles (baseline build-release-workspace.log.txt:245 "Compiling localization v0.1.0") and links into nothing. Every `Locale::parse` call site is inside crates/localization itself (audit.rs, catalog.rs, fallback.rs, message.rs — tests + catalog loading that itself has no consumer). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "Locale::parse|Locale::reference"` → all callers in-crate. `rg -l "localization" --glob Cargo.toml` → own crate only (others are comments). `python3` read of product.schema.json `$defs.bcp47` → pattern `^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$`. `rg -n "locales/|en-US\.toml"` in *.rs → only xtask config-validator (2-key existence check) + rebrand inspection note. `rg -n "product.description|window.title.idle" ui/src/i18n/messages.ts` → 0 matches (UI catalog is separate). `rg -n "idle_title_key" apps` → no non-test caller. Execution evidence (corrected): orchestrator no-fail-fast run (orchrun-test-workspace-nofailfast.log.txt) executed the localization suite — unittests 44 passed / **1 FAILED** (`fallback::tests::missing_everywhere_records_diagnostic_and_never_panics`, log:2727-2731, fallback.rs:292) — this file's 3 tests are among the 44 passed, and its doctest passed (`test crates/localization/src/locale.rs - locale::Locale (line 38) ... ok`, log:6407). |

## 1. One-paragraph verdict
locale.rs is the best-audited file in my batch and the most beside-the-point: a clean, correct, hand-traceable BCP-47 tag validator with honest error taxonomy, canonical casing, a sound `language_primary` invariant, and real content-asserting tests — inside a crate that no binary, tool, or sibling crate depends on. The product's actual localization runs on a different implementation entirely (ui/src/i18n/messages.ts, a 2,900-line TS catalog gated by `npm run l10n:check`, baseline #24 PASS), the reference TOML catalog (locales/en-US.toml) is read by nothing at runtime (only the xtask validator checks two keys in it), and the localized idle window title this subsystem would render is hardcoded as `"title": "Lumen"` in tauri.conf.json. The two defects inside the file itself (extlang mis-casing, singleton-subtag acceptance beyond the pinned schema shape) are latent precisely because nothing can reach them.

## 2. What actually works (verified)
- `parse` hand-traced against all 15 test cases (8 positive incl. `zh-hans-cn`→`zh-Hans-CN`, `es-419` stays numeric-lowercase; 7 negative incl. `en_US` → InvalidSubtag with the hyphen hint, `en--US` → EmptySubtag) — every trace matches the test's expectation; ASCII-only enforcement via `is_ascii_alphabetic`/`is_ascii_alphanumeric` is correct for BCP-47.
- `language_primary` (129-137): the documented invariant (parse guarantees a 2–3 lowercase-letter first subtag) holds — fields are private, `parse` is the only constructor, so no re-validation is genuinely needed; `unwrap_or_default` guards the impossible case only.
- `Locale::reference()` (60-64) single-sources the tag literal from `crate::catalog::REFERENCE_LOCALE` instead of duplicating "en-US" — the doc's "single source of truth" claim for the literal is TRUE (verified: no other "en-US" literal in locale.rs).
- Determinism: `Ord`/`Hash` derived over the owned String; canonicalization is a pure function of input.
- No `unsafe`, no panics, no swallowed errors; error enum carries actionable hints (the `en_US` → "BCP-47 uses hyphens" message is genuinely good UX).
- NOT verified by execution in INDEX runs #3/#4 (both abort before localization) — but EXECUTED in the orchestrator no-fail-fast repair run: this file's 3 unit tests passed (among localization's 44 passed; orchrun log:2674+) and the doctest passed (`test crates/localization/src/locale.rs - locale::Locale (line 38) ... ok`, log:6407). Notably the same suite has 1 FAILING test elsewhere in the crate: `fallback::tests::missing_everywhere_records_diagnostic_and_never_panics` panicked at fallback.rs:292 expecting `"missing:fixture.does-not-exist"`, got `"fixture.does-not-exist"` (log:2727-2731) — the crate ships RED in the only run that executed it (fallback.rs is A07's assignment; flagged for the wave gate).

## 3. Findings

### F-001 [S3]
Where: crates/localization/src/locale.rs:49-52 (the type) — crate-level reachability defect
Code:
```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Locale {
    tag: String,
}
```
What is wrong: The entire crate is dead weight in the build graph. No manifest in the repo declares a dependency on `localization` (verified across every Cargo.toml; the "localization" matches in crates/playlists/Cargo.toml:5 and plugin-api are prose comments). The runtime l10n of the shipped product is a separate TypeScript implementation: ui/src/i18n/messages.ts (the catalog), ui/scripts/l10n-check.mjs (the gate, baseline #24 PASS). The one product-facing artifact this subsystem validates — locales/en-US.toml, whose header says "Every other catalog … is audited against THIS file by `crates/localization`" — is read at runtime by nobody; the xtask config-validator checks exactly two msgids in it (identity.description_key, window.title_idle_key), and that validator itself is unwired from CI (W01-A08 F-001). The UI's language picker hardcodes its list while citing the dead crate as authority: ui/src/screens/settings/areas/language/locales.ts `AVAILABLE_LOCALES: readonly string[] = ["en-US"]` with a comment deferring to `load_locales_dir, catalog.rs` — a function whose only callers are the crate's own tests. The localized idle window title (`window.title.idle` msgid) is resolved by no one: `idle_title_key()` (apps/desktop/src/window_config.rs:114) has zero non-test callers and tauri.conf.json hardcodes `"title": "Lumen"`.
Why it matters: One of the four "owning crates" of the config-driven system (doc 07 header: product-config, settings-schema, localization, theme-tokens) is completely disconnected — its validators, fallback chains, plural rules and canonicalization guard nothing that ships. Every guarantee doc 07 §6 describes (key grammar, placeholder parity, fallback `pt-BR → pt → en-US`) is enforced only inside this crate's own tests — and in the one recorded run that executed them, the crate was RED (fallback.rs:292, orchrun log:2727): the orphaned subsystem is not even passing its own suite.
What correct looks like: either the desktop shell/CLI links this crate for catalog loading (its `load_locales_dir` exists and works per its tests), or the crate, locales/*.toml and doc 07 §6 are consolidated into the UI-side system that actually renders strings.
Confidence: HIGH — Verification: read + rg dependency graph (commands in header); test-execution gap cited from baseline logs.

### F-002 [S4]
Where: crates/localization/src/locale.rs:109-117
Code:
```
            } else if (subtag.len() == 2 || subtag.len() == 3)
                && subtag.chars().all(|c| c.is_ascii_alphabetic())
            {
                // Region subtag: uppercase (US, BRA, 419 stays as-is below).
                canonical.push_str(&subtag.to_ascii_uppercase());
```
What is wrong: Every 3-letter alphabetic subtag is uppercased as a "region". In BCP-47 a 3-letter subtag in the post-language position can be an EXTENDED LANGUAGE subtag (e.g. `zh-yue-Hant-HK`), whose canonical casing per RFC 5646 §2.1.1 is lowercase. This code canonicalizes `zh-yue` → `zh-YUE`, producing a tag that will not byte-match a correctly named catalog file `zh-yue-Hant-HK.toml` — the file-name-equals-canonical-tag contract (doc 07 §6.3) breaks for exactly the locale families that need it most. The module doc's limitation note defers extlang *validation* to Phase 4, but this is a *casing* error, not a validation gap.
Why it matters: Latent today (locales/ contains only en-US.toml), but the first real multi-locale catalog with an extlang tag silently fails to resolve. Blast radius is zero while the crate is unreachable (F-001), which is the only reason this is S4.
What correct looks like: uppercase only 2-letter alpha subtags (always regions) and 3-letter subtags known to be regions; leave 3-letter subtags lowercase unless a registry lookup confirms region (or defer canonical casing of 3-letter subtags entirely).
Confidence: HIGH — Verification: read (trace: "zh-yue-hant-hk" → "zh-YUE-Hant-HK"); no counter-test exists (the test table has no extlang case).

### F-003 [S4]
Where: crates/localization/src/locale.rs:4-6 vs 95
Code:
```
//! Validation follows the shape pinned by the product schema
//! (doc 07 §3.2 `$defs.bcp47`): `^[a-z]{2,3}(-[A-Za-z0-9]{2,8})*$`,
```
vs the code (line 95):
```
            if subtag.len() > 8 || !subtag.chars().all(|c| c.is_ascii_alphanumeric()) {
```
What is wrong: The doc claims the validation follows the pinned schema shape, but the code accepts subtags of length 1–8 (only emptiness is rejected) while the pinned regex requires 2–8 (`(-[A-Za-z0-9]{2,8})*`, verified by python read of product.schema.json `$defs.bcp47`). `Locale::parse("en-u-ca-buddhist")` succeeds here (singleton `u` — a legal BCP-47 extension opener) but the same tag is REJECTED by the product schema for `defaults.locale`. Two validation surfaces for the same concept disagree, and the module doc misstates which shape is implemented.
Why it matters: A catalog file named `en-u.toml`-style (or a defaults.locale value) validated by one surface can be rejected by the other; the drift is invisible until a second catalog or a locale-carrying config path exists.
What correct looks like: align the lower bound (either reject singletons here, or widen the schema regex and fix the doc), and make the doc quote the implemented shape.
Confidence: HIGH — Verification: read + python JSON read of the schema pattern.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `LocaleError` | 15-32 | REAL | — | 4 variants, actionable messages; the `en_US` hint is genuinely useful |
| `Locale` (struct) | 49-52 | REAL / UNREACHABLE | S3 | private field, parse-only construction (F-001) |
| `Locale::reference` | 60-64 | REAL / UNREACHABLE | — | single-sources REFERENCE_LOCALE (verified); only in-crate callers (fallback.rs:106, tests) |
| `Locale::parse` | 67-120 | REAL / UNREACHABLE | S4 | hand-traced all test cases correct; extlang mis-casing (F-002); accepts 1-char subtags beyond pinned shape (F-003); O(n) single pass; no panics |
| `Locale::as_str` | 122-125 | REAL | — | trivial accessor |
| `Locale::language_primary` | 129-137 | REAL | — | invariant sound (parse-only construction); `unwrap_or_default` documents the impossible branch |
| `Display` | 140-144 | REAL | — | delegates to tag |
| `AsRef<str>` | 146-150 | REAL | — | trivial |
| `parses_and_canonicalizes` (test) | 156-172 | REAL | — | 8 cases incl. casing; would fail if canonicalization broke |
| `rejects_malformed_tags` (test) | 174-209 | REAL | — | 7 cases, asserts exact error variants |
| `language_primary_parent` (test) | 211-221 | REAL | — | 4 cases |
| doctest | 38-47 | REAL | — | runs under cargo test; never executed in baseline |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parses_and_canonicalizes | YES | byte-exact canonical expectations incl. script Titlecase and numeric-region passthrough |
| rejects_malformed_tags | YES | asserts exact LocaleError variants (not just is_err) |
| language_primary_parent | YES | exact parent strings |
| doctest | YES | public-API usage with asserts |
| (execution) | — | EXECUTED and PASSED in the orchestrator no-fail-fast run: 3/3 unit tests ok (localization lib suite 44 passed), doctest ok (orchrun log:6407). Suite-level caveat: the same run shows 1 FAILED test elsewhere in the crate (fallback.rs:292) — the crate is red overall in its only executed run. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "file name = BCP-47 tag" (catalogs) | doc 07 §6.3 / locales/en-US.toml:6 | PAPER-ONLY | no runtime code loads catalogs through this crate (F-001); only its own tests |
| "audited against THIS file by crates/localization" | locales/en-US.toml:6-8 | REAL | audit.rs implements it and rebrand-style audit tests ran ok in the no-fail-fast run — but zero external dependents; the crate is otherwise red (fallback.rs:292 FAIL in the same run) |
| "Validation follows the shape pinned by the product schema" | locale.rs:4-5 | FALSE (superset) | code accepts 1-char subtags; schema regex requires 2-8 (F-003) |
| "2–3-letter regions uppercase" (canonical casing) | locale.rs:7 / doc 07 §3.2 | SHALLOW | correct for regions, wrong for 3-letter extlangs (F-002) |
| en-US "always available as the last link of the fallback chain" | locale.rs:55-56 / doc 07 §6.3 | PAPER-ONLY | fallback chain (fallback.rs) has no shipped consumer; the UI ships a single hardcoded locale |

## 7. Dead / duplicated / unreachable code
- The whole crate is unreachable (F-001): no manifest depends on `localization`.
- Duplicated l10n implementations: this crate (+ locales/*.toml) vs ui/src/i18n/messages.ts + l10n-check.mjs — two catalogs, two grammar rule sets, one of them shipped (the TS one), one of them documented as the system (doc 07 §6, the Rust one).
- `window.title.idle` / `product.description` msgids: present in en-US.toml, absent from the UI catalog that actually renders strings; the idle title is hardcoded in tauri.conf.json.

## 8. Dependencies & flags
toml 1.1 + thiserror 2.0 only (crate level); this file needs neither (std + fmt + thiserror derive). No cfg/feature gates in the file. Crate compiles clean in baseline build #1 (log line 245) while linking into nothing — pure build-graph dead weight today.

## 9. Open questions for the next agent
- A10 (catalog.rs / lib.rs): does the crate's public API (Catalog, load_locales_dir, audit) get referenced by ANY doc claiming it is wired (doc 01 §2.3 registry) — if docs claim runtime l10n flows through this crate, that is a FALSE doc row worth a doc-audit file.
- A04 (key.rs/refs.rs): does the key-grammar implementation (2–5 dotted kebab segments) match the UI-side l10n-check.mjs rule 4 grammar, or do the two disagree like the tag shapes do (F-003)?
- Wave summary: aggregate this with product-config unreachability — 2 of the 4 config-system "owning crates" have no shipped-runtime consumer.
