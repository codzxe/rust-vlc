# AUDIT: crates/product-config/src/model.rs
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-21 |
| Lines / functions | 260 / 22 (18 structs/enums + `default_true` + `Default for Telemetry` + 2 doc-verified claims) |
| File verdict | UNREACHABLE (real, correct data model; zero shipped-runtime consumers — every path that reads product.toml at runtime uses something else) |
| Reachable from shipped entry point? | NO. `rg -l "product-config" --glob Cargo.toml` → only `apps/control-server` (+ own crate). The shipped `lumen` CLI has no product-config dep; the tauri shell parses `[window]` with a bespoke std-only parser (`apps/desktop-tauri/src/app.rs:243` → `desktop::window_config::parse_window_table`, `apps/desktop/src/window_config.rs`), not this model; control-server (the sole binary dep) refuses to start under the shipped product.toml (`require_feature`, config.rs:149-158, `features.control-server = false`). Build/test-time consumers: `tests/rebrand/rebrand_acceptance.rs`, product-config's own tests. xtask does NOT use this crate (config_validator.rs:28-29 uses its own `toml_lite`). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -l "product-config" --glob Cargo.toml` → 4 hits, one real dependent. `rg -n "set_setting|ConfigSetSetting"` → dispatch.rs:151-155 (CLI settings seam uses settings-schema, not this model). `python3` JSON read of product.schema.json (required list, defaults) → telemetry/migrations required, no default annotations. `rg -n "to_string|serde_json::to|toml::to"` over crate → no serialization call site. `rg -ln "about_facts|cli_version_line|resolve_window_title|installer_identity|log_line_prefix|log_startup_line"` → only product-config itself + tests/rebrand. Execution evidence (corrected): INDEX runs #3/#4 abort before this crate, but the orchestrator no-fail-fast run (orchrun-test-workspace-nofailfast.log.txt:4613+) executed product_config unittests 41 passed/0 failed AND all integration suites incl. `rebrand_host` (the #[path]-wired rebrand acceptance) — all ok. Cross-referenced audit/files/product.toml.audit.md (W01-A08, S0:3). |

## 1. One-paragraph verdict
The data model itself is competent: 18 `deny_unknown_fields` structs that exactly mirror product.schema.json's 12 top-level properties, kebab-case enums matching the shipped spellings, deterministic BTreeMap feature table, all-off Telemetry default. But it is a model of a file nothing shipped ever loads. The only binary that depends on this crate (control-server) is feature-gated off by the very file it would load; the tauri shell reads `[window]` through a third, hand-rolled line-oriented parser instead of this crate; the CLI hard-codes mirror constants for the `[limits]` it should consume. And the duplication has already produced live drift: tauri.conf.json opens the window at 1280x**800** while this model's source of truth says 1280x**720**. Cross-wave note: this resolves W01-A08's open question "does lumen-tauri read product.toml at runtime?" — yes, but via its own parser, so the SSOT claim stays false.

## 2. What actually works (verified)
- Field-set completeness: `python3` read of product.schema.json `properties` = {brand, defaults, features, identity, limits, migrations, playback, schema_version, telemetry, urls, web, window} — exactly the 12 fields of `ProductConfig` (model.rs:18-38). The module doc's "Every field of product.toml has a home here" is TRUE.
- The `[web]`-not-layer-overridable comment (model.rs:29-31) is TRUE: `PartialProduct` (partial.rs:20-32) has no top-level `web` field (the `web` at partial.rs:60 is `identity.ids.web`), so a user layer carrying `[web]` fails `deny_unknown_fields`.
- `Telemetry::default()` (model.rs:216-225) is all-off — matches product.toml's shipped `[telemetry]` and MP §12 "no default telemetry".
- `Window.start_maximized`/`remember_geometry` serde defaults (false / `default_true`) match the schema's JSON-Schema `default` annotations (python read: false / true).
- Crate layering claim (lib.rs: "layer L1, no internal deps") holds for this file: only serde/serde_json/toml/thiserror (Cargo.toml `[dependencies]`), no workspace deps.

## 3. Findings

### F-001 [S2]
Where: crates/product-config/src/model.rs:17-39 (the whole typed surface)
Code:
```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductConfig {
    pub schema_version: u32,
    pub identity: Identity,
```
What is wrong: The typed product model has ZERO runtime consumers among shipped entry points. Proven consumer map: (a) `lumen` CLI — no product-config dependency (apps/cli/Cargo.toml); its queue caps are hard-coded mirrors (`apps/cli/src/demux_host.rs:69-73` "The CLI loads no product-config yet … these constants mirror the shipped values"); (b) `lumen-tauri` — parses `[window]` via `apps/desktop/src/window_config.rs` (a std-only, line-oriented "not a general TOML parser" reader, its own doc admits), reached at `apps/desktop-tauri/src/app.rs:243` `parse_window_table(include_str!("../../../product.toml"))`; this model is bypassed; (c) control-server — the ONLY dependent — exits `FeatureDisabled` under the shipped product.toml (`features.control-server = false` → `require_feature`, apps/control-server/src/config.rs:149-158); (d) xtask validators/rebrand use their own `toml_lite` parsing (tools/xtask/src/cmd/config_validator.rs:28-29), not this crate. The duplication cost is already visible: `apps/desktop-tauri/tauri.conf.json` windows entry says `"width": 1280, "height": 800` while product.toml (the model's source) says `height = 720` — the first-paint window and the SSOT disagree; app.rs then resizes to the parsed 720 (app.rs:255-261), i.e. a visible resize flicker born exactly of the two-sources problem this crate was built to end.
Why it matters: "Single source of truth" (product.toml:4) with three independent parsers (this crate, xtask toml_lite, desktop window_config) and one hard-coded mirror in the CLI is four sources drifting independently — the 800-vs-720 mismatch is the first confirmed casualty. Editing `[window]`/`[limits]`/`[playback]` in product.toml changes nothing at runtime except the tauri shell's post-launch resize.
What correct looks like: the tauri shell and CLI load this crate (it exists, with `load_file` + `runtime` helpers), or a gate asserts tauri.conf.json == product.toml `[window]` and the CLI mirrors == `[limits]`.
Confidence: HIGH — Verification: read (all cited lines) + rg dependency graph; tauri runtime behavior UNVERIFIED (baseline #2: tauri build blocked by missing gtk libs in sandbox).
(Cross-ref: W01-A08 audit/files/product.toml.audit.md F-004 — same pattern found from the file side; this finding adds the tauri-shell bypass and the 800/720 drift.)

### F-002 [S3]
Where: crates/product-config/src/model.rs:206-243 (Telemetry, Urls), 130-168 (Playback), 170-182 (Limits subset)
Code:
```
pub struct Telemetry {
    pub enabled: bool,
    pub crash_reports: CrashReports,
    pub usage_stats: bool,
    pub update_checks: bool,
}
```
What is wrong: Per-field consumer map (rg across apps/crates/tools for each field): `telemetry.*` — read only by semantic.rs:274 (the cross-field `enabled ⇒ urls.privacy` check) and the xtask mirror of that check; no telemetry system exists. `urls.*` — read only by xtask rebrand; the UI's About/updates screens contain no product URLs (rg `lumen-player.example.org` in ui/src → none). `playback.*` — no runtime reader; `subtitle_mode` exists as a settings.toml `default_ref` target that the CLI answers with a placeholder string (W01-A08 F-004: host.rs:1569 `"-> product.toml {path}"`). `limits.cache_demux_mib`/`probe_max_bytes`/`max_playlist_items`/`max_open_sessions`/`max_wasm_extension_memory_mib` and `identity.version_code` — no consumer found (W01-A08 §7 concurs from the file side). `limits.cache_network_mib` is mirrored as a constant in `crates/streaming/src/ranged.rs` and re-mirrored at 8 MiB for web in `apps/web/src/io/fetch_port.rs:41-45`.
Why it matters: These are typed, validated, documented config keys wired to nothing — the model gives them first-class struct fields, which lends them an authority the runtime does not honor. A user/operator setting `telemetry.update_checks = true` or `playback.default_rate = 1.5` changes zero behavior.
What correct looks like: either consumers or `# [PARKED]`-style markers in product.toml per key (the file already does this for some), and the model docs should say which sections are validation-only today.
Confidence: HIGH — Verification: rg per-field (commands in header); UI checked for URL rendering.
(Scope: product.toml's placeholder values were W01-A08 F-007; this finding is about the MODEL fields' consumption, not the values.)

### F-003 [S4]
Where: crates/product-config/src/model.rs:4-8 (module doc) vs 34-38
Code:
```
//! Optionality mirrors `product.schema.json`: fields the schema marks
//! required are plain; fields carrying JSON-Schema `default` annotations
//! (window geometry persistence flags, telemetry — doc 07 §3.2) fall back to
//! those defaults; `urls.*` are optional
```
What is wrong: The schema does NOT give telemetry a default annotation. Python read of product.schema.json: `telemetry` and `migrations` are both in `required` and neither carries a `default` key (`telemetry | required: True | has default: False`; same for migrations). Yet the model serde-defaults both (model.rs:34-35, 37-38). The two stages disagree about optionality: inside this crate's own pipeline the structural stage catches a missing `[telemetry]` first, but any direct deserializer (serde_json::from_value, as merge/rebrand code paths could do) accepts a document the schema calls invalid.
Why it matters: The doc's stated invariant ("optionality mirrors the schema") is false for 2 of 12 sections; a future consumer trusting the doc gets silently-injected defaults where the schema promised a hard error.
What correct looks like: either mark telemetry/migrations optional in the schema, or drop the serde defaults and let the typed stage reject missing sections.
Confidence: HIGH — Verification: read + python JSON read of product.schema.json.

### F-004 [S4]
Where: crates/product-config/src/model.rs:15 (and every struct: `Serialize` derive)
Code:
```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductConfig {
```
What is wrong: Every struct derives `Serialize`, but nothing serializes a ProductConfig anywhere: `rg -n "toml::to_|serde_json::to" crates/product-config` → zero call sites (the hits are diagnostic string rendering). There is consequently no round-trip test (parse → serialize → parse) in the crate — for a config system this is the cheapest guard against accidental non-round-trippable shapes (e.g. a future `#[serde(skip)]`).
Why it matters: Dead capability plus a missing test for the wave's "do settings round-trip?" question; today the answer is UNPROVEN for product.toml at the model level.
What correct looks like: one test: `load_str(root) → toml::to_string → load_str again → assert_eq`, or delete the derives until needed.
Confidence: HIGH — Verification: read + rg (no serialization sites).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ProductConfig` | 15-39 | REAL (shape) | S2 | 12 fields == schema properties (verified); unreachable at runtime (F-001); serde defaults on telemetry/migrations diverge from schema (F-003) |
| `Identity` | 41-52 | REAL (shape) | S3 | consumers: control-server engine_info (feature-off), xtool build tools, product-config::runtime fns — which only tests/rebrand call (see lib.rs audit) |
| `IdentityIds` | 54-63 | REAL (shape) | — | per-OS id structs below; validator enforces reverse_dns+PascalCase (baseline #10 passed that check) |
| `WindowsIds` | 65-70 | REAL (shape) | — | msi_upgrade_code consumed by packaging only |
| `MacosIds` | 72-76 | REAL (shape) | — | bundle_id for packaging |
| `LinuxIds` | 78-82 | REAL (shape) | — | appstream_id for packaging |
| `WebIds` | 84-88 | REAL (shape) | — | PWA manifest id; packaging only |
| `Window` | 90-104 | REAL (shape) | S2 | consumed by tauri shell via a DIFFERENT parser (F-001); tauri.conf.json duplicate drifts (800 vs 720) |
| `default_true` | 106-108 | REAL | — | matches schema default true |
| `Brand` | 110-119 | REAL (shape) | — | icon paths point at nonexistent assets (baseline #10, 4 violations — file-level S0 already recorded by W01-A08; model shape itself fine) |
| `Defaults` | 121-128 | REAL (shape) | S3 | theme/profile targets don't exist (W01-A08 F-005); locale join real (baseline #10 note) |
| `Playback` | 130-140 | REAL (shape) | S3 | zero runtime readers (F-002) |
| `SubtitleMode` | 142-149 | REAL | — | kebab-case; matches product.toml "auto" |
| `HwDecodePolicy` | 151-168 | REAL | — | 3-variant vocab post-ADR-001; shipped value software-only; no engine reader (policy is declarative until Phase 7) |
| `Limits` | 170-182 | REAL (shape) | S3 | 5 of 8 fields unconsumed; queues mirrored as constants; network mib mirrored twice (F-002) |
| `WebSection` | 184-194 | REAL | — | consumed by xtask web-budget — baseline #16 PASS (the one section with an executed consumer) |
| `WebSizeBudgets` | 196-204 | REAL | — | read by web_budget gate; deterministic u64s |
| `Telemetry` | 206-214 | REAL (shape) | S3 | validation-only consumer (F-002) |
| `Default for Telemetry` | 216-225 | REAL | — | all-off; matches MP §12 |
| `CrashReports` | 227-233 | REAL | — | off/ask/send vocab |
| `Urls` | 235-243 | REAL (shape) | S3 | no renderer; example.org placeholders (W01-A08 F-007) |
| `Migrations` / `MigrationEntry` | 245-260 | REAL (shape) | — | empty ledger; validator cross-checks schema_version == max(to) (xtask side, baseline #10 ran it) |

## 5. Test honesty (no tests in this file)
The model is exercised by crates/product-config/tests/{merge.rs, golden_s35.rs, web_budgets.rs, common} and the #[path]-wired tests/rebrand (via crates/product-config/tests/rebrand_host.rs). Execution evidence: the orchestrator no-fail-fast run executed all of them — product_config unittests 41 passed/0 failed (orchrun log:4613), every integration suite ok, rebrand_host ok (log:4724-4729 `rebrand_acceptance::surface_* ... ok`) — so the model IS test-executed. What no suite does: round-trip a parsed ProductConfig through Serialize (F-004 stands), and nothing executes the model from a shipped binary (control-server path exists but is feature-gated off; its e2e test loads fixtures, not the shipped product.toml). tests/rebrand/rebrand_acceptance.rs:28 itself records "(d) CLI identity … resolution PASS; host wiring BLOCKED" — the test's own table admits no host consumes these resolvers.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Every field of product.toml has a home here" | model.rs:1-2 | REAL | python read: schema properties == struct fields (12/12) |
| "Optionality mirrors product.schema.json" | model.rs:4-5 | FALSE (2 sections) | telemetry/migrations schema-required, no default; serde-defaulted (F-003) |
| "NOT layer-overridable in v1 … PartialProduct has no web field" | model.rs:29-31 | REAL | partial.rs:20-32 — no top-level web (line 60 is identity.ids.web) |
| "Every surface that shows a name reads from here at runtime" (product.toml §2.1, feeding Identity) | product.toml:13-15 | FALSE | CLI --version uses CARGO_PKG_VERSION (help.rs:98-102 [PROPOSAL]); runtime fns test-only; W01-A08 F-004 |
| web budgets "reads THESE values" | product.toml §2.9a | REAL | baseline #16 web-budget PASS (only executed consumer) |

## 7. Dead / duplicated / unreachable code
- Whole model: unreachable from shipped entry points (F-001) — reachable only via control-server (feature-off) and tests.
- `Serialize` derives: zero call sites (F-004).
- Duplicated parsers of the same file: this crate, tools/xtask toml_lite, apps/desktop window_config (line-oriented), ui Node TOML-subset (settings side) — four readings of "single source of truth".

## 8. Dependencies & flags
serde (derive), serde_json, toml, thiserror — all external; no workspace deps; no feature gates in this file (`rg -n "cfg\(" model.rs` → none). Deny_unknown_fields on all 18 structs (verified by reading each derive).

## 9. Open questions for the next agent
- A08 (product-config/src/runtime.rs): the six re-exported runtime fns are consumed only by tests/rebrand — is that "runtime" module anything but a test fixture factory? (My lib.rs audit records this.)
- A09 (merge.rs): does any layer merge reject a partial carrying schema-inconsistent values (e.g. window.width > 3840) since the partial stage skips range checks the base schema enforced?
- Baseline gap (corrected): the suite DID run in the orchestrator no-fail-fast repair run — no REQUEST-ORCH-RUN needed for product-config tests. Remaining unexecuted: any shipped binary loading the model at runtime.
