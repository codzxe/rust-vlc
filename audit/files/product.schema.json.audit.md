# AUDIT: product.schema.json
| Field | Value |
|---|---|
| Auditor | W01-A07 |
| Date | 2026-11-21 |
| Lines / functions | 300 / 45 schema units (12 root properties + 4 $defs + nested keys) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (runtime: `lumen-control-server` loads it by default — apps/control-server/src/main.rs:39 `DEFAULT_SCHEMA`, config.rs:219; gate: `cargo xtask config-validator` — baseline INDEX #10 exit 1; tests: `cargo test -p product-config` via tests/common/mod.rs:20-31; include_str! in tests/rebrand/rebrand_acceptance.rs:63, compiled via crates/product-config/tests/rebrand_host.rs:13-14). NOT consumed by the `lumen` CLI (apps/cli has no product-config dependency — apps/cli/src/host.rs:970 comment defers the join). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | rg -n "product\\.schema\\.json" (consumers: xtask config-validator, control-server, rebrand tests, packaging schema comment); Read of crates/product-config/src/{schema.rs,patterns.rs,semantic.rs,load.rs} + tests/{common/mod.rs,semantic.rs,fixtures_matrix.rs}; Read of tools/xtask/src/cmd/config_validator.rs (walk + cross_file); rg -n "pattern\\|magic\\|F1\\|registry" in config_validator.rs (absent); Grep "core-playback" *.rs (tests + product.toml only — no engine registry); ls themes/ (no *.toml) / profiles/ (absent); Read apps/cli/src/host.rs:1634-1643 (hardcoded engine features); baseline INDEX #10 (config-validator exit 1, 4 missing brand assets), #1 (build exit 0), #3 (workspace tests). |

## 1. One-paragraph verdict
This schema is a real, operative contract — not decoration. Its structural keyword set (type/required/additionalProperties/enum/const/minimum/maximum/minLength/maxLength/minItems/maxItems/items/pattern/not.enum/$ref) is fully implemented by crates/product-config/src/schema.rs:120-384 with a hand-written matcher registry (patterns.rs) whose coverage is mechanically checked at load (schema.rs:74-93, fails loudly on drift), and its structural rules run against the committed product.toml in `cargo test -p product-config` (tests/common/mod.rs:20-44). The semantic claims are only partially operative, however: F1/F2 (feature-registry intersection) has no production data source and never runs against the committed file; magic-byte checks never run against the committed file; and the theme/profile injected enums are unsatisfiable in the committed repo and downgraded to "notes" by the gate. The merge-blocking gate (`cargo xtask config-validator`) is a parallel reimplementation that enforces a subset (no patterns — honestly disclosed in its own output) and currently FAILS on main because the committed product.toml references four brand assets that do not exist.

## 2. What actually works (verified)
— Baseline INDEX #10: `cargo xtask config-validator` exit 1 with 4 violations (icon.ico, icon.icns, icon-512.png, logo.svg missing) — the schema's path-existence rule fired on the real committed files. The gate tool, the schema, and the semantic asset rule are all demonstrably live.
— crates/product-config/src/schema.rs:178-181 skips empty enums (`&& !variants.is_empty()`) exactly as the schema's `enum: []` injection contract (lines 153-158) requires; semantic.rs:328-383 implements the injected-enum check when a listing is supplied.
— Pattern enforcement is real and drift-guarded: schema.rs:74-93 walks the schema and errors `UnmatchedPattern` if any `pattern` string lacks a matcher; patterns.rs:300-323 pins the exact schema strings; range/pattern/enum fixtures in crates/product-config/tests/fixtures/ (range-probe-max-bytes.toml, pattern-*.toml, feature-unknown-f1.toml, …) are validated against the committed schema via load_fixture → root_schema().
— Rule F2's schema enum `hw_decode_policy` (lines 177-178) matches product.toml's shipped "software-only" and ADR-001's vocabulary.

## 3. Findings

### F-001 [S2]
Where: product.schema.json:5 and :164 vs tools/xtask/src/cmd/config_validator.rs:19-21, apps/cli/src/host.rs:1637-1641, crates/product-config/src/semantic.rs:432-436
Code (schema claim, product.schema.json:164):
```
      "$comment": "unknown feature ids are rejected in the semantic pass against the engine feature registry (rule F1, doc 07 §2.7/§3.3); a static enum here would drift.",
```
Code (gate admission, config_validator.rs:19-21):
```
//! * Rule F1/F2 (feature ids vs the engine feature registry) needs the
//!   registry surface; v0 checks that every `features.*` value is boolean.
//! * Magic-byte checks for brand assets are the crate's, not this tool's.
```
Code (engine's actual "registry", host.rs:1637-1641):
```
        features: vec![
            "first-vertical:avi-mjpeg-pcm".to_string(),
            "audio.null-device".to_string(),
            "video.software-compositor".to_string(),
        ],
```
What is wrong: The schema's headline semantic protections — F1/F2 feature-registry intersection and brand-asset magic bytes — have NO enforcement path against the committed product.toml. `FeatureRegistry::from_status` is called only from test fixtures (crates/product-config/tests/semantic.rs:201,218); the xtask gate checks only that `features.*` values are boolean; the committed-file semantic test runs with `SemanticInputs::none()` (tests/semantic.rs:289), which skips feature_checks and asset_checks entirely (src/semantic.rs:405, 433-436). Worse, the vocabularies can never meet: the committed product.toml names `core-playback`, `local-file`, `hw-decode`, etc. (product.toml:141-171), which appear in no engine code (Grep "core-playback" *.rs → tests and product.toml only), while the engine's feature surface is the three hardcoded strings above with zero overlap.
Why it matters: MP §5's "config can never enable a missing codec/protocol/capability" is the security posture this schema advertises; for the shipped config it has never executed once, and if it did, every feature id in product.toml would be a violation. A product.toml that flips `hw-decode = true` today passes every gate that exists (boolean type is all that is checked).
What correct looks like: one authoritative engine feature registry (the structure behind `engine_info()`, not a test fixture) exported from the engine crate and supplied by the xtask gate to `validate_semantics`, so F1/F2 run on the committed file; the schema's comment then describes a live mechanism.
Confidence: HIGH — Verification: read (all call sites traced; no production `from_status` caller exists).

### F-002 [S3]
Where: product.schema.json:153-158; product.toml:106-113; tools/xtask/src/cmd/config_validator.rs:469-472
Code (product.schema.json:153-154):
```
        "theme":   { "type": "string", "minLength": 1, "enum": [],
                     "$comment": "enum injected at validation time from the themes/ directory listing (doc 07 §3.2, §3.3)." },
```
Code (gate downgrade, config_validator.rs:469-472):
```
                None => notes.push(format!("{key}: `{dir}/` does not exist yet — enum injection skipped (parked plane, doc 07 §9 C4)")),
                Some(ids) if ids.is_empty() => notes.push(format!(
                    "{key}: `{dir}/` has no *.toml candidates — enum injection skipped until the plane lands (doc 07 §9 C4)"
                )),
```
What is wrong: The schema declares `defaults.theme`/`defaults.profile` as injected enums over `themes/<id>.toml` / `profiles/<id>.toml`. In the committed repo `themes/` contains no `*.toml` at all (only `generated/` and `tokens.json`) and `profiles/` does not exist, so the contract is unsatisfiable for ANY value — and the committed product.toml carries `theme = "dark"` (product.toml:108) and `profile = "default"` (product.toml:113), references to files that do not exist. The gate downgrades this to a printed note, so the shipped config silently dangles.
Why it matters: The schema presents an enforceable contract; the repo ships a config that violates it; the enforcement tool chose to look away. Anyone building on `defaults.profile = "default"` (playback defaults) is depending on a file that is not there.
What correct looks like: either land `themes/dark.toml` and `profiles/default.toml`, or the schema should mark these keys as reserved-for-future (e.g. an explicit `x-lumen-status: parked` extension) instead of an enum contract that cannot currently be satisfied.
Confidence: HIGH — Verification: read (ls themes/, ls profiles/ run in worktree; product.toml Read).

### F-003 [S3]
Where: product.schema.json:21-25, 120-125; tools/xtask/src/cmd/config_validator.rs:607-623 vs crates/product-config/src/semantic.rs:126-138; config_validator.rs:500-516
Code (xtask pascal_case, config_validator.rs:607-612):
```
fn pascal_case(name: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for c in name.chars() {
        if c.is_alphanumeric() {
```
Code (crate pascal_case, semantic.rs:128-129):
```
    for word in name.split([' ', '-', '_']) {
        let mut chars = word.chars();
```
What is wrong: Two independent validators enforce this one schema with divergent logic. (a) The §3.3(b) derived-identifier rule is computed by two different `pascal_case` implementations: the xtask's (capitalize each alphanumeric run) vs the crate's (split only on space/hyphen/underscore). A schema-legal `identity.name` containing a dot (name has no `pattern`, lines 21-25) — e.g. "Lumen 2.0" — yields `Lumen20` in the gate but `Lumen2.0` in the crate, so the same product.toml gets opposite verdicts on `app_user_model_id` from the two enforcers. (b) `title_template`: the schema pattern rejects any stray `{` (product.schema.json:122), but the gate's variable scan silently stops at an unterminated `{` (`else { break; }`, config_validator.rs:503) — a template like `"foo {"` passes the gate and fails the crate. (c) The xtask's own test suite validates a hand-mirrored SCHEMA fixture (tools/xtask/tests/config_validator.rs:69 `must(fs::write(self.root.join("product.schema.json"), SCHEMA));`), not the committed schema, so gate/crate drift is only caught by humans.
Why it matters: A contract enforced by two disagreeing implementations is not a contract; merge gates pass what the runtime validator rejects (or vice versa).
What correct looks like: the xtask gate should shell the validation through crates/product-config (one implementation), or at minimum import its pascal_case/matchers; its tests should load the committed schema file.
Confidence: HIGH — Verification: read (both implementations and the fixture-mirror test line verified).

### F-004 [S4]
Where: product.schema.json:122; apps/desktop/src/window_config.rs:123-126
Code (product.schema.json:122):
```
          "pattern": "^(\\{product_name\\}|\\{media_title\\}|[^{}]|\\{\\{)*$",
```
Code (renderer, window_config.rs:123-126):
```
    pub fn render_media_title(&self, product_name: &str, media_title: &str) -> String {
        self.title_template
            .replace("{product_name}", product_name)
            .replace("{media_title}", media_title)
```
What is wrong: The pattern's fourth alternative `\{\{` admits a literal `{{` while a literal `}}` is rejected (no alternative matches `}`); the renderer is a plain `.replace()` that gives `{{` no escape meaning, so the schema blesses a token with no defined semantics and asymmetrically bans its mirror.
Why it matters: A template author who assumes `{{`/`}}` escaping gets half the behavior: `{{media_title` validates, `{{media_title}}` does not. Confusing, undocumented in the hint (line 124).
What correct looks like: either drop `\{\{` from the pattern, or document+implement `{{`/`}}` escaping consistently in both the pattern and the renderer.
Confidence: HIGH — Verification: read (pattern and renderer code); matcher mirrors the asymmetry (patterns.rs:118-138).

### F-005 [S4]
Where: product.schema.json:194-198
Code:
```
        "probe_max_bytes": {
          "type": "integer", "minimum": 16777216, "maximum": 67108864,
          "x-lumen-unit": "MiB",
          "$comment": "16 MiB floor per the doc 07 §3.5 error contract and the x-lumen-hint rationale; the §2.9 inline comment ('1 MiB..=64 MiB') is the outlier.",
```
What is wrong: The schema itself records a contradiction with docs/07-config-driven-system.md §2.9 ("1 MiB..=64 MiB") and resolves it unilaterally in a $comment.
Why it matters: Two normative documents disagree on a validated range; whichever a contributor reads, the other surprises them. The doc side is a FALSE claim against the code (for the doc auditor: doc 07 §2.9 range is not what the schema enforces).
What correct looks like: fix doc 07 §2.9 to 16 MiB..=64 MiB (the schema's rationale — probing below 16 MiB rejects valid multi-track files — is coherent) and drop the "outlier" note.
Confidence: HIGH — Verification: read (schema); doc 07 §2.9 text not re-read by me — flagged for the doc-claims auditor.

### F-006 [S4]
Where: product.schema.json:3, 153-158, 23, 292-298
Code (product.schema.json:292-294):
```
    "repo_path": {
      "type": "string",
      "pattern": "^[^/].*$",
```
What is wrong: Hygiene cluster: (a) `$id` uses a placeholder domain (`https://lumen-player.example.org/schemas/product.v1.json`, line 3) — intentional clean-room posture but worth recording; (b) `"enum": []` is invalid per the JSON Schema 2020-12 meta-schema (enum requires ≥1 item), so no standard validator can consume this schema — the in-repo structural pass special-cases it (schema.rs:178-181), which is documented at line 5 but means the file is only usable by this repo's tooling; (c) `repo_path`'s regex `^[^/].*$` under ECMA semantics (JSON Schema regex) rejects strings containing line terminators (`.` excludes \n) while the hand matcher (patterns.rs:209-211) accepts them — an in-repo/invisible divergence; (d) the reserved-name `not.enum` (line 23) is exact-match, case-sensitive — "Vlc" or "VLC media player" pass; consistent with doc 07 §3.3(a)'s literal list, so recorded, not charged.
Why it matters: Individually trivial; collectively they mean the schema is a private dialect, not a JSON Schema.
What correct looks like: acceptable as-is for a clean-room monorepo, but a header line saying "this schema is validated by crates/product-config's subset engine, not by standard JSON Schema tooling" would remove the false expectation.
Confidence: HIGH — Verification: read.

## 4. Per-key table (schema units)
| Key (schema path) | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| $schema / $id / title / $comment | 2-5 | REAL | — | Meta; $comment accurately maps constraints to passes (verified against schema.rs/semantic.rs) |
| root required / additionalProperties:false | 7-9 | REAL | — | Enforced: schema.rs:329-346 (required, Full mode), 351-379 (unknown keys + did-you-mean) |
| schema_version | 11-14 | REAL | — | min 1, no const; == max(migrations.to) enforced semantically (semantic.rs:259-271, xtask 533-564) |
| identity.name | 21-25 | REAL | — | len 1-40, not.enum VLC/VideoLAN/vlc; no pattern → pascal_case divergence reachable (F-003) |
| identity.short_name | 26-30 | REAL | — | pattern + hint; reserved check semantic (semantic.rs:163-175) |
| identity.publisher | 31 | REAL | — | len 1-80 |
| identity.display_version | 32-36 | REAL | — | semver-shaped; matcher faithful (patterns.rs:50-68) |
| identity.version_code | 37-40 | REAL | — | int ≥ 1 |
| identity.description_key | 41-44 | REAL | — | $ref msgid + catalog existence (xtask 487-497 against locales/en-US.toml — exists) |
| identity.ids.reverse_dns | 50-54 | REAL | — | lowercase reverse-DNS; later labels may lead/trail with '-' (regex allows; harmless) |
| identity.ids.windows / macos / linux / web | 55-103 | REAL | — | minLength 1 + §3.3(b) semantic derivation (semantic.rs:178-207, xtask 567-586) — two divergent implementations (F-003) |
| msi_upgrade_code | 64-68 | REAL | — | GUID pattern + matcher (patterns.rs:99-116); doc 07 §3.3(c) diff-aware warn is [PROPOSAL], absent — schema doesn't claim it, OK |
| window.width/height | 114-115 | REAL | — | 480-3840 / 320-2160; range fixture range-window-width.toml proves enforcement |
| window.min_width/min_height | 116-117 | REAL | — | no max; < width/height enforced semantically (semantic.rs:210-227) — schema doesn't say so but semantic does; fine |
| window.start_maximized / remember_geometry | 118-119 | REAL | — | boolean defaults; defaults applied by model/merge, not schema (JSON Schema "default" is annotation-only — correct usage) |
| window.title_template | 120-125 | REAL | S4 | F-004 asymmetry; gate's scan diverges on stray '{' (F-003b) |
| window.title_idle_key | 126-129 | REAL | — | $ref msgid + catalog check |
| brand.accent_color | 137-141 | REAL | — | ^#[0-9a-f]{6}$; "must equal color.accent.primary once token extraction lands" is a PROMISE not enforced anywhere — noted, low value |
| brand.icon_ico/icns/png_512 + logo_svg | 142-145 | REAL | — | $ref repo_path; existence gate-checked (baseline #10 fired ×4); magic bytes NEVER run on committed file (F-001) |
| defaults.theme/locale/profile | 148-159 | REAL | S3 | F-002: theme/profile dangling in committed repo; locale OK (locales/en-US.toml exists) |
| features | 161-165 | REAL | S2 | additionalProperties boolean; F1/F2 dead for committed file (F-001) |
| playback.preferred_audio/subtitle_languages | 172-175 | REAL | — | 0-8 items × $ref bcp47 |
| playback.subtitle_mode / hw_decode_policy / start_paused / default_rate | 176-180 | REAL | — | enums + 0.25..=4.0; hw policy vocabulary matches ADR-001 |
| limits.* (8 keys) | 183-203 | REAL | — | all bounded; range-limits-multi + range-probe-max-bytes fixtures; S4 doc contradiction on probe floor (F-005) |
| web / web.size_budgets | 205-231 | REAL | — | optional at root (documented); consumed by xtask web-budget (baseline #16 PASS proves wiring) |
| telemetry.* | 232-242 | REAL | — | defaults false; enabled⇒urls.privacy enforced both validators |
| urls.* | 243-253 | REAL | — | $ref https_url; all optional |
| migrations.entries | 254-274 | REAL | — | chain rules enforced (semantic.rs:229-271, xtask 525-565) |
| $defs.msgid | 277-281 | REAL | — | matcher faithful |
| $defs.bcp47 | 282-285 | REAL | — | simplified BCP-47 (no singletons/extensions); documented as such |
| $defs.https_url | 287-291 | REAL | — | ^https://.+$ |
| $defs.repo_path | 292-298 | REAL | S4 | regex weaker than hint; '..'/backslash enforced only semantically for the 4 brand keys (semantic.rs:307-326) — the only $ref users, so covered |

## 5. Test honesty (tests covering this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| crates/product-config/tests/fixtures_matrix.rs (pattern-*, range-*, enum-*, unknown-*, reserved-*, feature-f1/f2 fixtures vs committed schema) | YES | Fixtures assert specific diagnostic counts/keys against the committed schema via root_schema(); mutating a bound/pattern/required list flips fixture outcomes. Pattern-string drift additionally trips Schema::from_str's UnmatchedPattern (schema.rs:74-93) making root_schema() panic |
| patterns.rs:300-323 every_matcher_is_registered_under_its_schema_string | YES (drift) | Pins the exact schema pattern strings; a renamed pattern fails the assert |
| crates/product-config/tests/semantic.rs (committed product.toml in-document rules, :276-289) | PARTIAL | Runs SemanticInputs::none() — proves cross-field/migrations rules on the committed file, but cannot catch F-001/F-002 class gaps by construction |
| tools/xtask/tests/config_validator.rs | NO (against this file) | Line 69 writes its own mirrored SCHEMA fixture, not the committed product.schema.json — gate-side schema drift is untested (F-003c) |
| tests/rebrand/rebrand_acceptance.rs (include_str! committed schema) | YES (drift/load) | Compiled via crates/product-config/tests/rebrand_host.rs:13-14; unreachable!() panics if the committed schema fails to load |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Validated by product.schema.json via `cargo xtask config-validator`" | product.toml:4-5 | REAL (subset) | Baseline INDEX #10: gate ran, exit 1, 4 asset violations; but patterns/F1/F2/magic-bytes excluded (config_validator.rs:14-21) |
| "a violation is a build-breaking error" | product.toml:5-6 | FALSE | Baseline INDEX #1 build exit 0 CLEAN while INDEX #10 shows 4 standing violations — violations exist and the build does not break; the gate is a separate xtask step, not cargo build |
| "unknown feature ids are rejected ... against the engine feature registry (rule F1)" | product.schema.json:164 (and doc 07 §2.7) | REAL-UNPROVEN (mechanism) / dead for committed file | semantic.rs:404-430 implements it; only test fixtures supply a registry (tests/semantic.rs:201,218); no production caller; committed vocabularies disjoint (host.rs:1637-1641) — F-001 |
| "path existence + magic bytes ... live in the crate's semantic pass" | product.schema.json:5 | REAL-UNPROVEN | semantic.rs:432-525 implements both; asset_checks skipped for committed file (SemanticInputs::none(), tests/semantic.rs:289); gate does existence only (config_validator.rs:446-452) |
| probe_max_bytes range "1 MiB..=64 MiB" | docs/07-config-driven-system.md §2.9 (per schema's own note) | FALSE (vs schema) | product.schema.json:195 enforces minimum 16777216 (16 MiB); schema $comment names §2.9 as "the outlier" — for the doc auditor |

## 7. Dead / duplicated / unreachable code
- F-001: `FeatureRegistry`/`FeatureStatus` (semantic.rs:47-80) — production-dead; only test constructors call `from_status`.
- F-003: entire xtask structural walk (config_validator.rs:242-425) duplicates crates/product-config/src/schema.rs with less coverage; two `pascal_case` implementations.
- `x-lumen-unit` is read only for the literal "MiB" (schema.rs:227-230) — any other unit string silently degrades to no-unit diagnostics; currently only "MiB" appears.

## 8. Dependencies & flags
- No feature flags touch this file. Loaded from: tools/xtask (gate), apps/control-server (runtime default `--schema product.schema.json`), crates/product-config tests, tests/rebrand acceptance (include_str!). packaging.schema.json mirrors its conventions (separate file, not my scope).
- The rebrand allowlist (tools/xtask/data/rebrand-allowlist.toml:64) protects this file from rebrand rewrites — appropriate (it must stay vendor-neutral).

## 9. Open questions for the next agent
- apps/control-server/src/config.rs:219 — confirm the runtime path joins the repo root (a deployed binary outside the repo would not find product.schema.json); does it fall back silently?
- Who renders `defaults.profile` at runtime (playback profile plane is "parked" — doc 07 §9 C4)? If nothing reads it, F-002 is cosmetic; if the desktop shell reads it, it is a runtime dangling ref.
- doc 07 §3.3(a) reserved list vs schema not.enum: doc lists 2 names, schema 3 — confirm intentional.
- For the CI auditor: ci.yml's schema-validation job runs `cargo test -p product-config` (line 276), which is what actually enforces patterns on the committed product.toml — the "config-validator" name in full_gate.sh alone under-validates.
