# AUDIT: crates/product-config/src/semantic.rs
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines / functions | 565 / 17 (+3 tests) |
| File verdict | UNREACHABLE (real, tested code; no shipped or CI path reaches it) |
| Reachable from shipped entry point? | NO. `validate_semantics` is called ONLY by crates/product-config/tests/semantic.rs:22,289. product-config is not a dependency of apps/cli or apps/desktop (rg over their Cargo.tomls: no match). The crate's one production consumer, apps/control-server (main.rs:154 `load_file`), runs parse+structural+typed and NEVER the semantic stage. `cross_field_checks` is also called by merge.rs:217 `resolve_layers`, which itself has no production callers. The designated consumer (`cargo xtask config-validator`, doc 07 §3.4) re-implements the rules in tools/xtask/src/cmd/config_validator.rs instead of calling this crate. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 4 S4: 3 |
| Commands run | rg callers (validate_semantics, cross_field_checks, SemanticInputs, pascal_case, from_status, product_config::, resolve_layers); sed quote verification; rg accent/product-config over Cargo.tomls; baseline cargo-test-workspace.txt (product-config exit 0) |

## 1. One-paragraph verdict
The logic in this file is real and honestly tested: every §3.3 cross-field rule
the doc inventory lists (geometry, migration chain, telemetry→privacy,
reserved short_name, derived ids, injected enums, catalog keys, F1/F2,
asset existence + magic bytes) is implemented and each has an integration
test. But it is a validator that validates nothing in production: the only
caller is its own test file, the xtask tool that doc 07 §3.4 designates as
the wiring re-implemented the rules (with a DIVERGENT pascal_case), and
control-server loads product.toml without stage 3. On top of that, the
Wave-1-flagged accent-vs-token cross-check is confirmed missing — not just
unwired: `SemanticInputs` has no field that could carry theme-token values,
so the crate mechanism cannot express doc 07 §2.3's "MUST equal the theme
token `color.accent.primary`" rule, which the tree violates today
(blue #4f8cff in product.toml vs orange #d37918 shipped in the UI CSS).

## 2. What actually works (verified)
— All §3.3 cross-field rules implemented and covered by
crates/product-config/tests/semantic.rs (20 tests incl.
`root_product_toml_passes_semantic_none` on the real product.toml); passed in
baseline audit/runs/baseline/cargo-test-workspace.txt (product-config exit 0).
— Verified myself: product.toml:95 `accent_color = "#4f8cff"` vs
ui/src/tokens/generated/tokens.css:31 `--color-accent: #d37918;` — the
mismatch the missing rule should have caught.

## 3. Findings
F-001 [S3]
Where: crates/product-config/src/semantic.rs:21-36 (SemanticInputs), whole file
What is wrong: the accent-vs-token cross-check (hostile focus; Wave 1
config_validator.rs.audit.md F-003) is absent, and the crate cannot even
express it — `SemanticInputs` carries themes/locales/profiles listings,
catalog keys, and a feature registry, but no theme-token values. doc 07:170-172:
"Accent color. MUST equal the theme token `color.accent.primary` once the
token extraction lands (doc 08)" — token extraction HAS landed
(themes/tokens.json + ui/src/tokens/generated/tokens.css), so the deferral is
over. The tree violates the rule today: product.toml:95
`accent_color = "#4f8cff"` (blue) vs tokens.css:31 `--color-accent: #d37918`
(orange) and themes/tokens.json:22 `"default-accent": "orange"`. The one
brand value that names the product's accent color is wrong and nothing
anywhere detects it. What correct looks like: a `SemanticInputs` field
carrying the resolved `color.accent.primary` token value (or the accent
primaries map) + a `brand.accent_color` membership/equality check.
Confidence: HIGH — Verification: read whole file; rg accent in semantic.rs
(zero hits); verified both literals with rg.
F-002 [S3]
Where: crates/product-config/src/semantic.rs:229-258
Code:
```
    // Migration chain: from < to, ascending, no gaps; schema_version == max(to).
    let entries = &config.migrations.entries;
    for (i, entry) in entries.iter().enumerate() {
        let key_path = format!("migrations.entries.{i}.from");
        if entry.from >= entry.to {
```
What is wrong: the chain's ANCHOR is never checked. The loop verifies
per-entry from<to and contiguity (`entry.from != prev.to` for i>0) but never
that `entries[0].from == 1`, even though the diagnostic's own hint (line 254)
says "migration entries must form one unbroken ascending chain from schema
version 1" and product.schema.json:272's $comment promises the same. A config
with a single entry `{from = 2, to = 3}` and `schema_version = 3` passes
schema (from minimum 1) and this pass clean — yet a v1 install has no
migration path to v3. No test covers the anchor either
(tests/semantic.rs has inverted-entry and version-mismatch tests only).
Confidence: HIGH — Verification: read; rg for `from == 1`/anchor in crate: none.
F-003 [S3]
Where: crates/product-config/src/semantic.rs:126-138 vs tools/xtask/src/cmd/config_validator.rs:607-621
Code (this file):
```
pub fn pascal_case(name: &str) -> String {
    let mut out = String::new();
    for word in name.split([' ', '-', '_']) {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
```
Code (xtask copy):
```
fn pascal_case(name: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for c in name.chars() {
        if c.is_alphanumeric() {
```
What is wrong: two implementations of doc 07 §3.3(b) with different
semantics. This file splits ONLY on space/hyphen/underscore and keeps other
characters in-word; the xtask copy drops every non-alphanumeric char and
re-capitalizes after it. For `identity.name = "Nova.Player"` this file derives
`org.example.Nova.Player` and the xtask validator derives
`org.example.NovaPlayer` — the same product.toml passes one id check and
fails the other. Divergent duplication of the same rule with no shared
source. Why it matters: rule §3.3(b) has two truths depending on which tool
you run; neither is wired into CI (Wave 1: config-validator wired into
nothing).
Confidence: HIGH — Verification: read both; derived outputs by hand-tracing
both algorithms.
F-004 [S3]
Where: crates/product-config/src/semantic.rs:141-158; crates/product-config/src/lib.rs:14-19
What is wrong: the semantic pass ships nowhere. `validate_semantics` callers:
its own integration test only. `cross_field_checks`' second caller
(merge.rs:217) is itself production-dead. control-server — the only binary
that loads product.toml at runtime (main.rs:154) — skips the semantic stage,
so a product.toml with a broken migration chain, reserved short_name, or
dangling description_key boots fine in the one shipped consumer that reads
it. lib.rs:14-19 presents "4. Semantic" as a pipeline stage as if it were
part of the load flow. Real code, zero production enforcement.
Confidence: HIGH — Verification: rg all callers.
F-005 [S4]
Where: crates/product-config/src/semantic.rs:243-247
Code:
```
        if i > 0 {
            let prev = match entries.get(i - 1) {
                Some(p) => p,
                None => continue,
            };
```
What is wrong: dead defensive arm — for `i > 0` within
`entries.iter().enumerate()`, `entries.get(i - 1)` is always `Some`.
Confidence: HIGH — Verification: read.
F-006 [S4]
Where: crates/product-config/src/semantic.rs:404-429
What is wrong: on every unknown feature id, `registry.known_ids()` is rebuilt
and joined (O(registry) per offending key → O(n·m) overall). Trivial at
current registry size; also the expected-string duplicates the full id list
per diagnostic. Cosmetic perf in a dead-code path.
Confidence: HIGH — Verification: read.
F-007 [S4]
Where: crates/product-config/src/semantic.rs:508-516 (SVG magic)
Code:
```
                let mut rest = bytes;
                if rest.starts_with(&[0xEF, 0xBB, 0xBF]) {
                    rest = &rest[3..];
                }
```
What is wrong: the UTF-8 BOM branch is untested (in-file tests cover the
whitespace case only, line 562: `magic_ok(b"\n  <?xml")`); an SVG in UTF-16
(with BOM FF FE) is correctly rejected but nothing pins the BOM-skip
behavior. Also any XML (not just SVG) passes — acknowledged heuristic in
`magic()` doc text ("starts (after whitespace/BOM) with `<`"), so not a
rule violation, just a weak check with no test for its one special case.
Confidence: HIGH — Verification: read + test list.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SemanticInputs::none | 40-42 | REAL-UNPROVEN (test-only use) | — | default struct |
| FeatureRegistry::from_status | 60-71 | REAL-UNPROVEN | — | test-only callers |
| FeatureRegistry::status | 73-75 | REAL | — | feature_checks |
| FeatureRegistry::known_ids | 77-79 | REAL | S4 | rebuilt per unknown id (F-006) |
| quoted | 91-93 | REAL | — | Debug-format quoting |
| Ctx::push | 102-121 | REAL | — | line via LineIndex.line_of |
| pascal_case | 126-138 | REAL | S3 | divergent twin in xtask (F-003) |
| validate_semantics | 141-158 | UNREACHABLE | S3 | test-only caller (F-004) |
| cross_field_checks | 162-305 | REAL-UNPROVEN | S3 | merge.rs caller also dead; anchor gap F-002 |
| unsafe_path_problem | 307-326 | REAL | — | tested; solid |
| injected_enums | 328-383 | REAL | — | deferred when listing None; themes/ plane does not exist in tree (Wave 1) |
| catalog_checks | 385-402 | REAL | — | deferred when None |
| feature_checks | 404-430 | REAL | S4 | F1/F2 correct; O(n·m) nit |
| asset_checks | 432-472 | REAL | — | reads whole file; correct magic dispatch |
| AssetKind::name | 483-490 | REAL | — | |
| AssetKind::magic | 492-499 | REAL | — | doc text matches behavior |
| AssetKind::magic_ok | 502-524 | REAL | S4 | BOM branch untested (F-007) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pascal_case_derives_identifiers | YES | 3 exact assertions |
| path_problems_are_detected | YES | 6 asserts incl. drive letter, `..`, backslash |
| magic_bytes_detect_formats | PARTIAL | no BOM-prefix SVG case (F-007) |
| (integration tests/semantic.rs, 20) | YES | per-rule positive+negative incl. deferred-inputs tests and real root product.toml guard |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| §3.3 cross-field rules all enforced | doc 07:424 table | REAL-UNPROVEN | implemented here (162-430) but no production caller (F-004) |
| "MUST equal the theme token color.accent.primary once the token extraction lands" | doc 07:170-172 | FALSE | no check in this file, none expressible via SemanticInputs, tree violates (F-001) |
| "the cargo xtask config-validator tool (card 1.E2 wiring) supplies the real listings" | semantic.rs:7-10 | FALSE | xtask config-validator does not depend on this crate (rg: comments only); it re-implements |
| "(c) msi_upgrade_code ... validator warns if it changes between commits [PROPOSAL]" | doc 07:424 | PAPER-ONLY | no diff-aware check anywhere in this file; marked PROPOSAL |
| "Shared with the merge pass, which re-runs them on the *resolved* config" | semantic.rs:160-161 | REAL | merge.rs:217 — but merge itself is production-dead |

## 7. Dead / duplicated / unreachable code
Unreachable: the whole pass in production (F-004). Duplicated: pascal_case
(F-003). Dead arm: entries.get(i-1) None continue (F-005).

## 8. Dependencies & flags
std only (std::fs for assets). No feature gates. Crate not in lumen-cli/desktop
dependency graphs (rg over Cargo.tomls).

## 9. Open questions for the next agent
- If control-server is a shipped surface, its startup skips every semantic
  rule — is that a deliberate v1 cut or an oversight? (control-server owner.)
- merge.rs `resolve_layers` re-runs cross_field_checks on resolved configs but
  nothing production calls resolve_layers either — audit merge.rs reachability
  when assigned.
- Who owns the accent rule end-state: SemanticInputs grows a token field, or
  theme-tokens' own lint owns it? (theme-tokens/lib.rs audit — my file, next.)
