# AUDIT: crates/localization/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A10 |
| Date | 2026-11-21 |
| Lines / config keys | 25 / 7 (package 5, lints 1, deps 2) |
| File verdict | UNREACHABLE (manifest consistent; the crate it defines — 3,391 LOC — has zero dependents and no runtime consumer) |
| Reachable from shipped entry point? | NO — rg `^localization = ` over all 38 member manifests: zero path-deps; rg `localization::|use localization` over crates/ apps/ tools/ *.rs: matches only inside crates/localization itself |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg dependents over member manifests (none); rg crate-external usage (none); rg `toml::|thiserror` in src (used: catalog.rs, message.rs, key.rs, locale.rs, refs.rs); sed boundaries.toml:88-93 (externals match); ls locales/ (exactly one file: en-US.toml); rg "en-US.toml" in ui/ (no reference — separate catalog); rg "messages.ts" in crates/ (no reference — no cross-catalog comparator); head crates/localization/tests/seed_integrity.rs (CI-wiring claim); rg xtask registry (mod.rs:73-190 — 17 subcommands, no `l10n-audit`); rg product.toml msgid keys (description_key:32, title_idle_key:84 — the only real catalog consumers) |

## 1. One-paragraph verdict
The manifest is internally consistent (both externals used, boundaries entry exact), but the crate it defines is a second orphan in my sample: 3,391 lines of key grammar, fallback chains, plural rules, and catalog auditing that no shipped binary links — while the one surface that actually shows user-facing strings (the UI) carries a completely parallel TypeScript catalog (`ui/src/i18n/messages.ts`) audited by a npm gate (`ui/scripts/l10n-check.mjs`, baseline #24 PASS). The two catalogs are never compared by any code anywhere, despite the UI gate's comment claiming "the Rust-side crates/localization audit owns the cross-catalog comparison". The promised xtask `l10n-audit` CI wiring (card 1.E2) was never registered — the frozen xtask registry has 17 subcommands and it is not one of them.

## 2. What actually works (verified)
- Dep usage: toml (catalog parsing) and thiserror (typed errors) both used in src (rg matches in catalog.rs, message.rs, key.rs, locale.rs, refs.rs).
- Boundary conformance: boundaries.toml:88-93 layer 1, internal_deps=[], external_allowlist=[toml, thiserror] — exact match; baseline #11: no localization violation.
- The catalog-audit rules exist: src/audit.rs implements them (e.g. audit.rs:153 "Rule 2 (cross-file half) — Orphan keys"); tests/seed_integrity.rs is a negative control over the shipped tree (asserts locales/ passes every rule and that product.toml's 2 msgid refs resolve) and ran in baseline #3.
- The reference catalog's real consumers: product.toml msgid keys (`description_key = "product.description"` at product.toml:32, `title_idle_key = "window.title.idle"` at :84) — validated by product-config/src/semantic.rs (reads locales/ listings directly, not via this crate).

## 3. Findings

### F-001 [S3]
Where: crates/localization/Cargo.toml:1-6 (the crate's advertised role) vs the dependency graph
Code:
```
# crates/localization — Lumen localization mechanism (task 1.B3, doc 07 §6).
#
# Registry: doc 01 §2.3 (`localization`, layer L1, no internal deps).
# Catalogs are DATA in `locales/`; this crate provides the mechanism:
```
What is wrong: The "mechanism" ships nowhere. Zero member manifests depend on `localization`; zero non-localization source files use `localization::`. The shipped UI's strings come from a different catalog in a different language (`ui/src/i18n/messages.ts`, TypeScript) with its own audit gate (ui/scripts/l10n-check.mjs — 7 rules, npm-run, baseline #24). `locales/` contains exactly one file (en-US.toml). No code compares the two catalogs (rg `en-US.toml` in ui/: no matches; rg `messages.ts` in crates/: no matches).
Why it matters: Two parallel l10n mechanisms that never meet is the classic drift machine — the UI can rename a key while locales/en-US.toml keeps the old one (or vice versa) and no gate fails. 3,391 lines of Rust l10n machinery are pure cargo-time dead weight; the "fallback chain" and "plural" modules have exactly one catalog to fall back across.
What correct looks like: Either the UI consumes the reference catalog (generated from locales/en-US.toml, with a diff gate), or the Rust crate is reduced to what product.toml validation needs and the manifest says the UI owns runtime l10n.
Confidence: HIGH — Verification: read (all four rg passes above; ls locales/; baseline #24 for the npm gate).

### F-002 [S4]
Where: crates/localization/Cargo.toml:5-6 (claim "the five doc 07 §6.4 catalog-audit rules") + crates/localization/tests/seed_integrity.rs:11-12 (the wiring promise)
Code (seed_integrity.rs:10-12, the enforcement claim):
```
//! The violation fixtures proving that the rules *fire* live in
//! `tests/audit_proof_matrix.rs`; this file is the negative control over
//! the repository itself. CI wiring (`cargo xtask l10n-audit`, doc 07
//! §6.4) is card 1.E2 — until then this suite is the enforcement.
```
What is wrong: The promised CI wiring does not exist. The frozen xtask registry (tools/xtask/src/cmd/mod.rs:73-190) has 17 subcommands — gate, schema-gen, tokens, codegen, check-architecture, check-panics, rebrand, fixtures, visual, feature-audit, config-validator, parity-report, lint-literals, check-identity, sbom, compat-matrix, web-budget — no `l10n-audit`. Enforcement of the Rust-side rules is cargo-test-only (which does run: baseline #3), and the UI-side gate (l10n-check.mjs:7) asserts "the Rust-side crates/localization audit owns the cross-catalog comparison" — a comparison no code performs (F-001).
Why it matters: A stated ownership boundary ("Rust owns cross-catalog comparison") that nothing implements means the one audit that would catch the two-catalog drift of F-001 is a comment, not a check.
What correct looks like: Register `l10n-audit` in xtask (it can shell out like schema-gen does) and implement the cross-catalog diff, or strike the ownership claim from l10n-check.mjs and record the single-catalog reality in this manifest.
Confidence: HIGH — Verification: read (xtask registry enumerated; rg for any cross-catalog reader: none).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| [package] name/version/edition/description/publish | 13-18 | REAL | — | description accurate for the code; but the mechanism has no consumer (F-001) |
| [lints] workspace = true | 20-21 | REAL | — | enforced (baseline #5) |
| [dependencies] toml = "1.1" | 24 | REAL | — | used (catalog parsing); req drift vs settings-schema's "1" — root-manifest F-002 |
| [dependencies] thiserror = "2.0" | 25 | REAL | — | used (typed errors); req drift vs "2" elsewhere — root-manifest F-002 |

## 5. Test honesty
No tests in the manifest. Crate tests (seed_integrity.rs, audit_proof_matrix.rs + 8 fixture trees) ran in baseline #3. seed_integrity is a genuine negative control (asserts the shipped locales/ tree passes the rules); audit_proof_matrix proves the rules fire on planted violations (fixture dirs listed). Both would fail if the audit logic broke — but nothing fails if the UI catalog drifts, which is the gap (F-001/F-002).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Registry: doc 01 §2.3 (localization, layer L1, no internal deps)" | Cargo.toml:3 | REAL | boundaries.toml:88-93 matches |
| "Catalogs are DATA in locales/" | Cargo.toml:4 | REAL | locales/en-US.toml exists (the only file) |
| "this crate provides the mechanism: key grammar, message-format v1 subset, fallback chain, and the five doc 07 §6.4 catalog-audit rules" | Cargo.toml:4-6 | REAL / UNREACHABLE | modules exist (key.rs, message.rs, fallback.rs, audit.rs) — but no runtime consumer (F-001) |
| "both pre-audited in docs/dependencies/preaudit-foundation.md, verdict pass; no NEW crates were introduced" | Cargo.toml:8-9 | REAL-UNPROVEN | deps are exactly toml+thiserror; the preaudit doc itself is another agent's file |
| (ecosystem claim) "the Rust-side crates/localization audit owns the cross-catalog comparison — doc 07 §6.4" | ui/scripts/l10n-check.mjs:7 | FALSE | no code reads both catalogs (rg both directions) |

## 7. Dead / duplicated / unreachable code
The crate is unreachable from shipped entry points (F-001). Duplication: the l10n-audit rules exist twice (Rust audit.rs for locales/*.toml; npm l10n-check.mjs for ui/src/i18n/messages.ts) with zero shared data.

## 8. Dependencies & flags
2 externals, both used and boundary-allowlisted. No internal deps (layer 1). No features, no dev-deps (tests are std+fixture-file based). Version-req drift ("1.1"/"2.0" here vs "1"/"2" in sibling crates) tracked at root-manifest level.

## 9. Open questions for the next agent
- docs/07-config-driven-system.md §6.4 is the source of the five rules and the cross-catalog comparison promise — the doc auditor should extract the exact promise and reconcile with the absent implementation.
- If the UI catalog is ever generated from locales/en-US.toml, product-config's direct locales/ listing (semantic.rs:27-28) and the localization crate's audit need one owner.
