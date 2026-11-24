# AUDIT: crates/product-config/src/model.rs
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines / functions | 260 / 18 types (11 structs, 3 enums, 1 Default impl, 1 fn default_true) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO for the release lane (lumen-cli/desktop make zero `product_config::` calls — rg; apps/desktop/src/main.rs:3 mentions "product-config" in a comment only). Real consumer: apps/control-server (feature-gated off in product.toml, not in release lane) via `load_file`. Plus tests/xtask. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `python3` schema.json vs model comparison (exit 0); `rg` quotes/partial.rs web check (exit 0/1); baseline product-config section cited (all suites pass) |

## 1. One-paragraph verdict
A clean, data-only typed model of product.toml: 19 `deny_unknown_fields` derives, closed kebab-case vocabularies for SubtitleMode/HwDecodePolicy/CrashReports, privacy-safe all-off Telemetry default, and a deliberately non-layer-overridable `Option<WebSection>`. I verified the struct/required-field sets match product.schema.json field-for-field (window: 6 required + 2 defaulted on both sides; root: 11 required + optional web on both sides), and that serde's typed enums reject wrong-typed values (`subtitle_mode = 3` fails the typed parse) — the enum hole Wave 1 found lives ONLY in the xtask's parallel validator, not here or in crate schema.rs. No logic, so no correctness surface beyond drift.

## 2. What actually works (verified)
— verified by `python3` JSON dump of product.schema.json compared against the model (window/identity/playback/limits/brand/defaults/telemetry/root required+field sets identical; root required = 11 = model's non-defaulted fields; web optional both sides) and by baseline product-config suites passing (exit 0). Enum vocabularies match schema.json exactly (subtitle_mode 4, hw_decode_policy 3, crash_reports 3).

## 3. Findings
F-001 [S4]
Where: crates/product-config/src/model.rs:1-8 vs product.schema.json
Code:
```
//! The typed product model (doc 07 §2). Every field of `product.toml` has a
//! home here; `serde(deny_unknown_fields)` keeps the set exact.
```
What is wrong: two sources of truth for the same shape — the JSON schema (validated structurally) and this serde model (validated typed). They agree today (verified), but nothing in model.rs ties them: a field added to the schema but not the model (or vice versa) fails only where the two validation stages disagree, and the serde stage's error ("unknown field") is less actionable than the schema stage's (file/line/hint). The crate's golden/fixture tests are the real drift guard, not the model.
Confidence: HIGH — Verification: read + python3 comparison (in sync NOW).
F-002 [S4]
Where: crates/product-config/src/model.rs:49 / :139 / :174-181
Code:
```
    pub version_code: u64,
```
and:
```
    pub default_rate: f64,
```
What is wrong: raw numeric types with NO in-model bounds — `version_code: u64` accepts 18446744073709551615 and `default_rate: f64` accepts NaN/inf from a TOML float (`nan` is valid TOML). All bound enforcement lives in schema.rs (JSON-Schema minimum/maximum) and semantic.rs; a caller constructing `ProductConfig` programmatically (the merge path does, via PartialProduct) bypasses JSON-Schema and relies on the merge stage re-validation. `default_rate = nan` passes the schema range check? No — schema.rs:226 `value.is_number()` + minimum/maximum comparison with NaN: `nan < 0.25` is false, `nan > 4.0` is false → NaN PASSES the schema range check (comparisons with NaN are false). Then serde parses `f64::NAN` happily. UNVERIFIED whether semantic.rs re-checks default_rate; flag for semantic.rs's owner.
Confidence: MEDIUM — Verification: read; NaN path reasoned, not executed (cargo-build-budget — no test binary run).
[NAN follow-up: schema.rs:234-235 `below = min.is_some_and(|m| found < m)` — for found=NaN both `found < m` and `found > m` are false → no diagnostic. This is a REAL hole in the range check for NaN, though TOML `nan` for default_rate is an exotic input; severity kept at S4 here because the typed/semantic layer may catch it — UNVERIFIED.]

## 4. Per-function table (per type; file >100 LOC)
| Type | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ProductConfig | 15-39 | REAL | — | 12 fields, web optional, telemetry/migrations/urls defaulted |
| Identity | 42-52 | REAL | — | 7 fields, matches schema required set |
| IdentityIds + per-OS ids | 55-88 | REAL | — | reverse_dns + windows/macos/linux/web |
| Window | 90-104 | REAL | — | u32 geometry; start_maximized default false; remember_geometry default_true |
| default_true | 106-108 | REAL | — | |
| Brand | 111-119 | REAL | — | 5 asset paths |
| Defaults | 122-128 | REAL | — | theme/locale/profile — schema enum:[] (injected from dirs) |
| Playback | 131-140 | REAL | S4 | default_rate f64 (F-002) |
| SubtitleMode | 142-149 | REAL | — | closed 4-value kebab enum; typed parse rejects ints |
| HwDecodePolicy | 151-168 | REAL | — | closed 3-value; post-ADR-001 comment honest (shipped stays software-only) |
| Limits | 170-182 | REAL | — | 8 u64 bounds; enforcement in schema/semantic |
| WebSection / WebSizeBudgets | 190-204 | REAL | — | Option, no code-side default (gate errors loudly) — verified comment vs PartialProduct (no web field) |
| Telemetry + Default | 207-225 | REAL | — | privacy default all-off |
| CrashReports | 227-233 | REAL | — | off/ask/send matches schema |
| Urls | 236-243 | REAL | — | all Option (cross-field rule makes absence meaningful) |
| Migrations / MigrationEntry | 246-260 | REAL | — | ledger only; no migrator execution in this crate |

## 5. Test honesty
No tests in-file (data model). Covered indirectly by tests/semantic.rs, tests/golden_s35.rs, tests/fixtures_matrix.rs (17+3+27 tests, baseline pass).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every field of product.toml has a home here; deny_unknown_fields keeps the set exact" | model.rs:1-2 | REAL | python3 field-set comparison: identical |
| "fields carrying JSON-Schema default annotations … fall back to those defaults" | model.rs:4-6 | REAL | window defaults (start_maximized/remember_geometry) + telemetry/urls/migrations `#[serde(default)]` match schema defaults (crash_reports default "off" == Telemetry::default) |
| "NOT layer-overridable in v1: PartialProduct has no web field, so a layer document carrying [web] fails loudly" | model.rs:29-31 | REAL | rg partial.rs: 12 fields, no web |
| hw policy "shipped product.toml stays software-only until Phase 7" | model.rs:154-156 | REAL-UNPROVEN | comment vs product.toml value not checked by me (product.toml read is other agents'); schema $comment repeats it |

## 7. Dead / duplicated / unreachable code
None dead. Whole crate's app-reachability: control-server only (gated off). Model is otherwise exercised by tests/xtask.

## 8. Dependencies & flags
serde (derive) only. No feature gates.

## 9. Open questions for the next agent
- Does semantic.rs bound-check `default_rate` (NaN/inf) and `version_code` overflow? If not, NaN default_rate passes the whole crate pipeline (schema range check provably NaN-blind, schema.rs:234-235) — potential S2 for whoever owns semantic.rs/schema.rs.
- MigrationEntry.migrator is a String with no registry — who resolves/loads migrators? (No code in this crate.)
