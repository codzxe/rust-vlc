# AUDIT: crates/product-config/src/partial.rs
| Field | Value |
|---|---|
| Auditor | W11-A04 |
| Date | 2026-11-13 |
| Lines / functions | 166 / 14 units (13 serde structs + 1 method) |
| File verdict | UNREACHABLE (correct, tested data shapes; zero callers outside product-config's own tests) |
| Reachable from shipped entry point? | NO — `load_partial_str`/`load_partial_file`/`resolve_layers`/`PartialProduct` have zero callers outside crates/product-config (rg over apps/, tools/, tests/, crates/ minus product-config: 0 hits). apps/control-server (the only product-config dependent) uses `load_file` (full model) only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `cargo test -p product-config --test web_budgets` -> 3 passed, 0 failed; `cargo test -p product-config --lib patterns` (spillover check) -> 11 passed; rg caller traces; field-parity diff of partial.rs vs model.rs (all 11 sections, 40+ fields, one deliberate exclusion) |

## 1. One-paragraph verdict
Well-built sparse-override shapes: every struct is all-`Option` with the correct serde combo (`deny_unknown_fields, default`), field parity with the full `model.rs` is complete except one deliberate, tested exclusion (`[web]` — layers must not override wasm size budgets, pinned by `layers_cannot_override_web_budgets`), and merge semantics downstream are sound (per-key insert for `features`, per-field for structs — verified in merge.rs:403-406/468-486). But nothing uses it: no app, tool, or integration test loads a user-preferences layer through `PartialProduct`; the doc's claim "v1 (card 1.B1) loads this from user preferences" is false in the current tree. Worse, the one designed rejection path (`[web]` in a layer) surfaces through serde's unknown-field error whose diagnostic hint tells the user "this is a product-config crate bug; please report it" — a designed rule reported as an internal error.

## 2. What actually works (verified)
— `cargo test -p product-config --test web_budgets` -> 3 passed, incl. `layers_cannot_override_web_budgets ... ok` (a layer doc with `[web.size_budgets]` fails loudly — the exclusion is enforced, not just documented).
— Field-parity check (read, field-by-field against model.rs): identity (7 fields incl. 4 id-platform sub-structs), window (8), brand (5 incl. icon_png_512 — initially looked missing due to my rg pattern excluding digits; verified present at model.rs:117), defaults (3), features (map), playback (6), limits (8), telemetry (4), urls (4), migrations (entries) — all present; only `web` (model.rs:33 `Option<WebSection>`) is excluded, deliberately.
— serde correctness (read): `#[serde(deny_unknown_fields, default)]` on every partial struct is exactly the right combo for sparse docs (missing = None; unknown = error). `is_empty` (L36-38) via `== Default` is correct.
— Merge semantics (read, merge.rs:297+): `features` merges per-key (L403-406), scalar/struct fields override per-field — a higher layer cannot clobber unset sibling keys.

## 3. Findings
F-001 [S3]
Where: crates/product-config/src/partial.rs:14-17 (claim) + repo caller graph (contradiction)
Code: (verbatim, L14-17)
```
/// A sparse override document for any precedence layer (doc 07 §4.1).
///
/// v1 (card 1.B1) loads this from user preferences; profile/admin/flag/
/// session loaders are Phase-2+ cards that reuse this shape.
```
What is wrong: "v1 loads this from user preferences" — nothing does. rg for `load_partial|resolve_layers|PartialProduct` outside crates/product-config: 0 hits in apps/, tools/, tests/, crates/. The layer machinery (partial.rs + merge.rs) is exercised only by product-config's own tests (tests/merge.rs, tests/web_budgets.rs). control-server, the crate's only dependent, calls `load_file` (full model). Why it matters: the entire doc-07 §4 precedence-layer system (layers 2-6, sparse overrides) exists as shapes + tests with no shipped reader; "user preferences" is not a product surface today. What correct looks like: mark the doc "[PLANNED — no v1 loader]" or wire a loader. Confidence: HIGH — Verification: read + rg (repo-wide).
F-002 [S3]
Where: crates/product-config/src/partial.rs:20-32 (absence of `web`) + crates/product-config/src/load.rs:130-133 (the hint that fires)
Code: (verbatim, load.rs:130-133)
```
            expected: "a sparse subset of the product.toml shape (crates/product-config/src/partial.rs)".to_string(),
            found: err.to_string(),
            hint: "the schema pass and the partial model disagree — this is a product-config crate bug; please report it with the offending file.".to_string(),
```
What is wrong: the deliberate `web` exclusion (tested in web_budgets.rs:58-70, documented at model.rs:184-189 as "budgets are a product decision") is enforced by serde's `deny_unknown_fields` on `PartialProduct`, and that error path is reported with the hint above — telling the user a *designed rejection* is a crate bug to be reported. partial.rs's own doc (L6-8) promises typos surface "as startup errors with did-you-mean hints (§4.4 rule 4, §4.5 T10)" — the `web` case gets no did-you-mean and an actively wrong hint. Why it matters: any user who puts `[web]` in a preference layer is directed to file a bug report against a working rule. What correct looks like: special-case `web` in `load_partial_str` (or pre-check) with "budgets are not layer-overridable (doc 07 §4.4; model.rs)" plus the did-you-mean machinery. Confidence: HIGH — Verification: read (schema.rs has no `web` special-case in Partial mode — rg: 0 hits — so the serde step is the failing stage; the web_budgets test passing confirms failure occurs, my trace identifies which diagnostic). UNVERIFIED by direct diagnostic capture (would need a scratch harness; build budget).
F-003 [S4]
Where: crates/product-config/src/partial.rs:1-8
Code: (verbatim, L1-8)
```
//! The user-layer shape: a *partial* product configuration (doc 07 §4).
//!
//! Layers 2–6 (profile, admin, user preferences, flags, session — §4.1) are
//! expressed as sparse overrides over the product defaults: only keys the
//! layer sets are present; everything else falls through to the next-lower
//! layer. Unknown keys are rejected at parse time (`deny_unknown_fields`) so
//! typos surface as startup errors with did-you-mean hints (§4.4 rule 4,
//! §4.5 T10) — never as silent no-ops.
```
What is wrong: the module doc says "sparse ... product configuration" and "unknown keys are rejected ... with did-you-mean hints" but never mentions that `[web]` — a legal product.toml section — is excluded from the partial shape. The exclusion is documented in model.rs and the web_budgets test, not in the file that defines the shape. A maintainer adding a field to model.rs has no in-file warning that partial.rs must deliberately NOT mirror it (or must, for every other section). What correct looks like: one sentence here: "`web` is product-level only (model.rs:184-189) and is rejected in layers by design". Confidence: HIGH — Verification: read.

## 4. Per-unit table (13 structs + 1 method; all trivially correct serde shapes)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PartialProduct | 18-32 | REAL | S3 | 11 of 12 model sections; `web` absent by design (F-002/F-003) |
| PartialProduct::is_empty | 34-39 | REAL | — | `== Default` comparison, correct |
| PartialIdentity | 41-51 | REAL | — | 7 fields, parity ✓ |
| PartialIdentityIds | 53-61 | REAL | — | 5 platform sub-structs, parity ✓ |
| PartialWindowsIds / PartialMacosIds / PartialLinuxIds / PartialWebIds | 63-86 | REAL | — | parity ✓ |
| PartialWindow | 88-99 | REAL | — | 8 fields, parity ✓ |
| PartialBrand | 101-109 | REAL | — | 5 fields incl. icon_png_512, parity ✓ |
| PartialDefaults | 111-117 | REAL | — | theme/locale/profile, parity ✓ |
| PartialPlayback | 119-128 | REAL | — | 6 fields incl. typed enums from model, parity ✓ |
| PartialLimits | 130-141 | REAL | — | 8 fields, parity ✓ |
| PartialTelemetry | 143-150 | REAL | — | 4 fields, parity ✓ |
| PartialUrls | 152-159 | REAL | — | 4 fields, parity ✓ |
| PartialMigrations | 161-165 | REAL | — | entries Vec, parity ✓ |

## 5. Test honesty
No tests in this file. Guarding tests (run/verified):
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| web_budgets::layers_cannot_override_web_budgets | YES | a layer doc with `[web.size_budgets]` must produce diagnostics — catches accidental re-addition of `web` to PartialProduct |
| web_budgets::committed_budgets_match_doc_09... | YES | pins committed product.toml values 3/6 MiB |
| tests/merge.rs (layer machinery over these shapes) | YES (not re-read line-by-line; scope) | uses load_partial_str + resolve_layers end-to-end |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v1 (card 1.B1) loads this from user preferences" (L16-17) | partial.rs doc | FALSE | F-001: zero external callers (rg) |
| "Unknown keys are rejected at parse time ... with did-you-mean hints ... never as silent no-ops" (L6-8) | partial.rs doc | PARTLY FALSE | rejection: REAL (deny_unknown_fields + schema pass); did-you-mean: exists only for schema-known keys (schema.rs:362-363), NOT for the designed `web` exclusion — which instead reports "crate bug" (F-002) |
| "a sparse subset of the product.toml shape" (load.rs expected text) | load.rs:131 | TRUE minus `web` | parity check (§2) |

## 7. Dead / duplicated / unreachable code
- The whole file is unreachable from shipped entry points (F-001): product-config's only dependent (control-server) uses the full-model path.
- No dead fields: every partial field has a merge_over arm (verified by reading merge.rs:297-486 field-by-field for the sections audited).

## 8. Dependencies & flags
serde (Serialize/Deserialize) only, via crate deps (serde, toml, thiserror). No unsafe, no features, no cfg. `BTreeMap` ordering gives deterministic serialization.

## 9. Open questions for the next agent
- merge.rs auditor (same wave?): confirm every PartialXxx field has a merge_over arm — I verified identity/window/features/urls/migrations/playback/limits sections; a full mechanical cross-check of the remaining arms belongs to the merge.rs audit.
- If/when a user-preferences loader lands (F-001), the F-002 hint must be fixed first or every designed rejection will be reported as a bug.
