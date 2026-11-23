# AUDIT: product.schema.json
| Field | Value |
|---|---|
| Auditor | W01-A08 |
| Date | 2026-11-23 |
| Lines / functions | 300 / n/a (JSON config: 11 top-level properties + 4 $defs) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev/test plane only. Read by crates/product-config/src/schema.rs (structural walk + x-lumen annotations, lines 78/205/228/271/292/305/523) and tools/xtask/src/cmd/config_validator.rs (L104-140). Only apps/control-server depends on product-config; lumen-cli/desktop/desktop-tauri do not (rg over apps/*/Cargo.toml). No .github workflow and no scripts/ script invokes config-validator (rg: 0 hits). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | rg for schema readers, "default"/x-lumen keyword readers, config-validator in scripts/.github, product-config dependents; sed verifications; baseline citations (xtask-config-validator.txt, full_gate.txt) |

## 1. One-paragraph verdict
The schema is real and structurally complete: I compared every key of the committed product.toml (17 tables, ~70 leaf keys) against it — every product.toml key is covered, no unknown keys, every required key present, every value inside its declared range/pattern; the orchestrator's config-validator run agrees (0 structural violations). The deferral contract in the $comment header is honest and implemented: cross-field rules (min_width<width, id derivation, F1/F2 feature checks, catalog keys, injected enums, asset existence) all exist in crates/product-config/src/semantic.rs and the xtask validator. The custom annotations x-lumen-unit/x-lumen-hint are genuinely read for diagnostics (schema.rs:228,523), not decorative. The real defects: product.toml's four brand-asset paths point into a directory that does not exist (assets/brand/ is absent) — the semantic pass correctly fails 4 violations, but no automated lane (full_gate has no config-validator step; zero workflow references; cargo test's semantic test runs with SemanticInputs::none(), skipping asset checks) so the repo's own product definition stands permanently invalid with nothing red in CI; and the schema's six "default" annotations are read by no code at all — the operative defaults are hardcoded in model.rs/partial.rs serde attributes (currently in agreement, nothing keeps them in sync).

## 2. What actually works (verified)
— verified by line-by-line key comparison (product.toml L10-244 vs schema properties): complete structural coverage, no unknown keys, all values in range (width 1280 in 480..=3840, default_rate 1.0 in 0.25..=4.0, probe_max_bytes 16777216 exactly at the 16 MiB floor, all 4 URLs https, GUID well-formed, bcp47 arrays within maxItems 8).
— verified by citation of audit/runs/baseline/xtask-config-validator.txt: the validator ran the structural walk against this schema and reported 0 structural violations — only 4 semantic asset violations ("brand.icon_ico: asset path `assets/brand/icon.ico` does not exist under the root" x4, "result: FAILURE (4 violation(s))").
— verified by rg: x-lumen-unit consumed at crates/product-config/src/schema.rs:228, x-lumen-hint at :523; minItems/maxItems enforced at :292/:305; not.enum at :205; pattern at :78/:271.
— verified by rg: min_width<width (deferred by schema, claimed in product.toml L70 comment) IS enforced in crates/product-config/src/semantic.rs:210 and apps/desktop/src/window_config.rs:273, with tests at crates/product-config/tests/semantic.rs:46.

## 3. Findings
F-001 [S3]
Where: product.schema.json:292-298 ($defs.repo_path) + product.toml:97-101
Code:
```
    "repo_path": {
      "type": "string",
      "pattern": "^[^/].*$",
      "minLength": 1,
      ...
      "$comment": "existence and magic bytes are checked in the validator's semantic pass (doc 07 §3.3 path existence)."
```
What is wrong: Not a schema defect — a standing data violation the schema's own contract surfaces: product.toml's [brand] references assets/brand/icon.ico, icon.icns, icon-512.png, logo.svg, and `assets/brand/` does not exist in the repo (`ls: cannot access 'assets/brand': No such file or directory`). The semantic pass correctly fails (baseline: 4 violations, FAILURE), but enforcement is orphaned: full_gate.sh has no config-validator step (full_gate.txt steps 1-11), no .github/workflows file references config-validator (rg: 0 hits), and the only cargo-test coverage of the committed file (crates/product-config/tests/semantic.rs:278-291 `root_product_toml_passes_semantic_none`) runs `validate_semantics` with `SemanticInputs::none()` — asset checks off. scripts/release/gen-icon.py:5-6 admits "those brand assets do not exist yet in this repository" and papers over packaging with a generated placeholder. Why it matters: product.toml's header claims validation is "a build-breaking error" — these 4 violations break no build anywhere. What correct looks like: commit the assets (or point [brand] at the placeholder generator's output), and wire config-validator into full_gate/CI. Confidence: HIGH — Verification: read + baseline citation + ls.

F-002 [S3]
Where: product.schema.json:118-119,236-240 vs crates/product-config/src/model.rs:98-101,209-221
Code:
```
        "start_maximized":   { "type": "boolean", "default": false },
        "remember_geometry": { "type": "boolean", "default": true },
```
What is wrong: The six `"default"` annotations (window.start_maximized=false, window.remember_geometry=true, telemetry.enabled=false, crash_reports="off", usage_stats=false, update_checks=false) are read by NO code: the xtask config-validator's documented keyword set (config_validator.rs:11) omits `default`, and product-config's schema.rs has no `"default"` handling (rg: 0 hits). The operative defaults are hardcoded independently — model.rs `#[serde(default)] start_maximized: bool` (false) and `#[serde(default = "default_true")] remember_geometry: bool` (true), plus a manual `impl Default for Telemetry` (model.rs:216-221). The two sources currently agree, but nothing synchronizes them: editing the schema default silently diverges from runtime behavior. Why it matters: the schema is advertised as the single source of truth for product decisions; for defaults it is a second, unread copy. What correct looks like: either have the loader read schema defaults, or delete the annotations and state that defaults live in model.rs. Confidence: HIGH — Verification: read (rg over both validators + model.rs).

F-003 [S4]
Where: product.schema.json:194-199 vs product.toml:205
Code:
```
        "probe_max_bytes": {
          "type": "integer", "minimum": 16777216, "maximum": 67108864,
          ...
          "$comment": "16 MiB floor per the doc 07 §3.5 error contract and the x-lumen-hint rationale; the §2.9 inline comment ('1 MiB..=64 MiB') is the outlier.",
```
What is wrong: Cross-file doc contradiction, self-acknowledged in-band but never fixed: product.toml:205 comments `probe_max_bytes = 16777216      # probing bound (MP §12). schema 1 MiB..=64 MiB` while the schema floor is 16777216 = 16 MiB. The schema's $comment names the product.toml comment as "the outlier" — a known lie shipped in the data file instead of corrected. A developer trusting the comment sets 1048576 and gets a validation error. What correct looks like: fix product.toml:205 to "schema 16 MiB..=64 MiB". Confidence: HIGH — Verification: read (both files).

F-004 [S4]
Where: product.schema.json:153-158 + audit/runs/baseline/xtask-config-validator.txt
Code:
```
        "theme":   { "type": "string", "minLength": 1, "enum": [],
                     "$comment": "enum injected at validation time from the themes/ directory listing (doc 07 §3.2, §3.3)." },
```
What is wrong: `"enum": []` is invalid per JSON Schema draft 2020-12 (enum requires >=1 value; an empty enum matches nothing), so this document is not processable by standard validators — deliberately, per the header $comment ("ignored by other JSON Schema validators per spec"), and only the two in-repo walkers implement the injection semantics. Lock-in accepted and documented, but it means the "$schema": "https://json-schema.org/draft/2020-12/schema" declaration at line 2 is itself misleading: no conforming 2020-12 validator can accept this file. Also note the xtask validator skips ALL pattern constraints (baseline note: "pattern constraints not enforced by this v0 (std has no regex; the product-config crate's CI suite owns them)") — so the semver/GUID/https/hex/msgid/bcp47 patterns in this schema are enforced only by product-config's tests, not by the `cargo xtask config-validator` invocation product.toml:5 names. Confidence: HIGH — Verification: read + baseline citation.

F-005 [S4]
Where: product.schema.json:205-231 ("web")
Code:
```
      "$comment": "Web-target product decisions (doc 09; card 7.K1, issue #49). Optional at the top level so sparse layer fixtures without it stay valid; the web-budget gate errors loudly when it is missing from the committed product.toml. NOT layer-overridable in v1: PartialProduct has no web field, ...",
```
What is wrong: Nothing defective in the constraint itself (verified: product.toml carries [web.size_budgets] 3/6 within 1..=64 / 1..=1024; 6 >= 3), but the schema's enforcement split means the top-level `required` list (L7-8) demands 11 tables while `web` floats free — a product.toml without [web] passes this schema and only the separate web-budget gate catches it (full_gate step 10). Two-gate coupling documented in-band; noting as structure debt, not a bug. Confidence: HIGH — Verification: read + full_gate.txt:76-85 citation.

## 4. Per-key inventory (schema -> product.toml coverage)
| Schema section | product.toml keys | Structural match | Semantic deferral honored? |
|---|---|---|---|
| schema_version | 1 | YES (int >=1) | version==max(migrations.to): yes (semantic.rs, xtask X-row) |
| identity (+ids.*) | 19 keys across 6 tables | YES, all patterns/ranges pass | id-derivation + reserved short_name: yes (semantic.rs:165,211) |
| window | 8 keys | YES (width/height/min_* in range) | min_width<width: yes (semantic.rs:210) |
| brand | 5 keys | YES (accent hex pattern ok) | asset existence: FAILS 4x — repo data defect (F-001) |
| defaults | 3 keys | YES (enum [] injected) | themes/ has no *.toml, profiles/ absent -> honest notes (baseline) |
| features | 22 booleans | YES (additionalProperties boolean) | F1/F2 registry checks: yes (semantic.rs:404-431) |
| playback | 6 keys | YES (bcp47 arrays, enum values, rate 1.0) | — |
| limits | 8 keys | YES (all in range; probe at exact floor) | — |
| web.size_budgets | 2 keys | YES (3 in 1..=64, 6 in 1..=1024) | full>=phase2: web-budget gate (schema comment; consumer verified existing as cmd) |
| telemetry | 4 keys | YES (all optional) | enabled=true => urls.privacy: yes (xtask X-row) |
| urls | 4 keys | YES (all https) | — |
| migrations | entries=[] | YES (entries optional) | from<to ascending: semantic (schema L271 comment) |

## 5. Test honesty
No tests in this file. The validators that consume it are tested: crates/product-config/tests/semantic.rs (incl. `root_product_toml_passes_semantic_none`, L278-291, which skips asset checks via `SemanticInputs::none()` — see F-001) and tools/xtask/tests/config_validator.rs (existence verified by rg; results in baseline cargo-test log).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Validated by product.schema.json via `cargo xtask config-validator`" | product.toml:4-5 | SHALLOW | true for structure only; xtask validator skips patterns (baseline note) and is run by no automated lane (F-001) |
| "a violation is a build-breaking error" | product.toml:5-6 | FALSE for the 4 standing asset violations | no full_gate step, no workflow, cargo test uses SemanticInputs::none() (F-001) |
| "existence and magic bytes are checked in the validator's semantic pass" | schema L297 | REAL | baseline config-validator: 4 existence violations |
| "x-lumen-unit (human unit expansion ...), x-lumen-hint (rule-specific fix hint)" | schema L5 | REAL | schema.rs:228 and :523 consume them |
| "An empty 'enum' [] marks a value whose enum is injected at validation time" | schema L5 | REAL | baseline notes: theme/profile enum injection skipped (empty/absent listings); locale validated against locales/ |
| product.toml comment "schema 1 MiB..=64 MiB" | product.toml:205 | FALSE | schema floor is 16777216 = 16 MiB (F-003) |
| product.toml comment "schema: min_width < width" | product.toml:70 | REAL (semantic, not structural) | semantic.rs:210 + window_config.rs:273 + tests |

## 7. Dead / duplicated / unreachable code
- The six `"default"` keywords: read by nothing (F-002) — the only schema keywords with zero consumers.
- `"description"` annotation on title_template (L123) duplicates the adjacent x-lumen-hint; harmless annotation, also unread.
- Everything else is consumed (verified keyword-by-keyword against schema.rs and config_validator.rs).

## 8. Dependencies & flags
Pure data file. Consumers: crates/product-config (structural+semantic, std-only regex-free pattern engine in patterns.rs) and tools/xtask config_validator (structural subset + cross-file). No feature flags.

## 9. Open questions for the next agent
- Does any Phase-5 control-server build ship product-config (its only app dependent)? If yes, the schema walk becomes runtime-reachable there; today it is test/dev-plane only.
- Who owns committing the missing assets/brand/* files — gen-icon.py generates a placeholder PNG for packaging, but the .ico/.icns/svg remain ungenerated (S3, F-001 follow-up).
