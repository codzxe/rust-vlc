# AUDIT: crates/product-config/src/runtime.rs
| Field | Value |
|---|---|
| Auditor | W11-A08 |
| Date | 2026-11-21 |
| Lines / functions | 213 / 9 items (7 pub fns + 2 pub structs) + 2 unit tests (+7 acceptance tests via tests/rebrand) |
| File verdict | UNREACHABLE (real, correct, executed-in-tests code; ZERO production callers — the doc's "runtime plane" is a library awaiting hosts) |
| Reachable from shipped entry point? | NO — repo-wide rg for all 7 function names: only `lib.rs:59-60` (re-export), this file's tests (orchrun log:4640,4651 ok), and `tests/rebrand/rebrand_acceptance.rs` (7/7 ok, orchrun log:4721-4734). No `apps/` binary calls any of them. The product-config CRATE's only runtime consumer is `apps/control-server` (config.rs:161-171, `identity.display_version` → engineInfo), which uses none of these functions |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | rg all 7 fn names repo-wide (callers: re-export + tests only); rg "product-config" in **/Cargo.toml (single dependent: apps/control-server:43); read apps/desktop/src/window_config.rs (own [window] parser + own title renderer); read apps/cli/src/demux_host.rs:65-74 + host.rs:961-974,1631-1634 (mirror constants, crate-version engine_info); sed product.toml:196-206 (limits values 1024/512); sed product.schema.json:16-40,109-128 (name unconstrained for braces; title_template pattern admits `{{`); orchrun log: rebrand_host 7/7, runtime unit tests ok; sed -n verification of every quote below |

## 1. One-paragraph verdict
The seven identity resolvers are pure, correct, panic-free, and executed — but only by tests. Not one production call site exists: doc 07 §8.2's headline "every surface reads `product-config` at run time" is FALSE in the shipped tree — the CLI prints a compile-time crate version (`env!("CARGO_PKG_VERSION")`, baseline `engine-version: PASS (lumen 0.1.3)`), the desktop shell re-implements window parsing and title rendering in its own std-only reader, and only control-server reads the crate (one field). The code is honest about this ("Hosts that do not exist yet … are recorded as honest gaps"), the acceptance table marks "host wiring BLOCKED", and the worklog records the handoff — but the doc's runtime-rebrand promise remains unmet, and the CLI additionally MIRRORS product.toml `[limits]` as constants instead of reading the file. This is the Wave-8 `audio.set-eq` pattern at the identity plane: a config seam built, validated, tested — and awaited by nothing.

## 2. What actually works (verified)
- All 7 functions traced over the model (`Identity`, `Window`, `Brand` in model.rs): correct field use, no panics, `#[must_use]` purity. Executed and passing (baseline): orchrun log:4640 (`title_resolution_substitutes_only_the_legal_variables ... ok`), :4651 (`the_shipped_product_toml_resolves_lumen_identity ... ok` — include_str! of the REAL product.toml, a live drift-tie), and :4721-4734 (`rebrand_acceptance` 7/7 with the Nova fixture — window title, installer ids, About, CLI identity, log identity all resolve).
- `installer_identity`'s `desktop_bundle_identifier` format claim verified against the committed tauri.conf.json:5 `"identifier": "dev.lumen.desktop"` — matches `format!("{}.desktop", reverse_dns)` for `dev.lumen`.
- Mirror constants currently MATCH the config: product.toml:203-204 `queue_packets_audio = 1024` / `queue_packets_video = 512` == demux_host.rs:71-73 constants (verified by sed).

## 3. Findings

### F-001 [S2]
Where: crates/product-config/src/runtime.rs:1-21 (the declared runtime plane) vs docs/07-config-driven-system.md:792,817; hard evidence apps/cli/src/host.rs:1631-1633, apps/cli/src/help.rs:29-31, tests/rebrand/rebrand_acceptance.rs:28
Code (doc 07:792, the claim):
```
| **Runtime** — config-driven | every surface reads `product-config` at run time | window title (template + `identity.name`), About page, CLI `--version`/banner, log line prefix, control-server `engineInfo`, PWA manifest name, onboarding copy via `product.*` keys |
```
and (doc 07:817, §8.3 step 3, the asserted acceptance):
```
3. Build (tier 2) and boot the CLI headlessly; assert `--version` prints `Nova Player …`; assert a log line prefix carries `nova`.
```
vs reality (apps/cli/src/host.rs:1631-1633):
```rust
pub fn engine_info() -> EngineInfo {
    EngineInfo {
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
```
and (apps/cli/src/help.rs:29-31):
```rust
//! Product identity (`product.toml` name/logo) is not wired in v0 —
//! `--version` reports the crate version and says so `[PROPOSAL:
//! product-config joins with the rebrand-propagation card]`.
```
What is wrong: Zero production callers of `resolve_window_title`, `log_line_prefix`, `log_startup_line`, `cli_version_line`, `cli_usage_header`, `installer_identity`, `about_facts` (repo-wide rg: re-export + tests only). The shipped `lumen` binary's identity surfaces are compile-time constants: engine/version from `CARGO_PKG_VERSION` (baseline rc-smoke-engine-only.log.txt:12 `engine-version: PASS (lumen 0.1.3)` — not `cli_version_line`'s "Lumen 0.1.3" from `identity.name`), usage header a literal (help.rs), log prefix never derived from `short_name`. The desktop shell parses `[window]` with its OWN std-only reader and renders titles with its OWN `render_media_title` (apps/desktop/src/window_config.rs:123-127) rather than this crate. Only `apps/control-server` loads product-config at runtime, and reads exactly one identity field (config.rs:168 `engine_version`).
Why it matters: The config system's headline runtime promise — rebrand the product by editing product.toml — does not reach the flagship binary. A Nova rebrand changes nothing the shipped CLI prints. The acceptance suite itself concedes it (rebrand_acceptance.rs:28: "resolution PASS; host wiring BLOCKED") and the worklog records the pending handoff (docs/worklog/archive-2026-10.md:8461), but doc 07 §8.2/§8.3 state it as current behavior with no [PROPOSAL]/[VERIFY] marker on those rows.
What correct looks like: Wire apps/cli help.rs's banner/usage to `cli_version_line`/`cli_usage_header` fed by a loaded ProductConfig (the handoff already written in the worklog), then delete the BLOCKED row; until then, doc 07 §8.2's runtime row and §8.3 step 3 must carry the same BLOCKED marker the acceptance report honestly carries.
Confidence: HIGH — Verification: read (rg callers, host.rs, help.rs, window_config.rs, control-server config.rs) + baseline citation (rc-smoke-engine-only.log.txt:12).

### F-002 [S3]
Where: crates/product-config/src/runtime.rs:70-76
Code:
```rust
    match media_title {
        Some(media) => window
            .title_template
            .replace("{product_name}", &identity.name)
            .replace("{media_title}", media),
        None => identity.name.clone(),
    }
```
What is wrong: `resolve_window_title` takes `&Window` but never reads `window.title_idle_key`. The idle branch hardwires `identity.name`. The doc comment (runtime.rs:61-63) justifies this: the idle key's "pre-localization rendering is the product name itself" — true only while `locales/en-US.toml`'s `window.title.idle` stays exactly `"{product_name}"` (it is, today — locales/en-US.toml:75). Meanwhile the real consumer path for the idle title — the desktop shell — resolves the KEY, not the name: `WindowTitle::Idle { key: self.title_idle_key.clone() }` handed to the webview's i18n layer (apps/desktop/src/window_config.rs:142-144, doc comment 149-151). Two divergent idle-title semantics for one config field; the Rust-side localization join ("wording joins via … `window.title_idle_key` with the localization layer", runtime.rs:16-18) cannot happen because the localization crate has zero dependents (file-1 audit, F-001/F-003).
Why it matters: `title_idle_key` is declared in product.toml, schema-required (`product.schema.json:111`), msgid-validated by config-validator — and rendered by no Rust code. If the catalog's idle wording ever gains text beyond `{product_name}`, this function silently returns the wrong title while the desktop path renders the right one. Wave-8 `audio.set-eq` family: config declared, validated, never consumed at this seam.
What correct looks like: Either render the idle branch through the (future) localization join — taking the rendered idle string as a parameter or resolving the key — or delete the `window` parameter's idle pretense and document that the idle title is the product name by current definition.
Confidence: HIGH — Verification: read (runtime.rs, window_config.rs, locales/en-US.toml, product.schema.json).

### F-003 [S3] (cross-file — apps/cli/src/demux_host.rs; owner: CLI app, not in W11-A08 list)
Where: apps/cli/src/demux_host.rs:65-73, consumed at apps/cli/src/host.rs:969-974
Code (demux_host.rs:66-73):
```rust
/// no product-config yet (that join is the rebrand-propagation card's,
/// see `host.rs`'s `default_value` `[PROPOSAL]`), so these constants
/// mirror the shipped values and the open path consumes the product
/// posture **today**; when the product-config load lands it replaces
/// this pair 1:1 (`limits.queue_packets_audio/video`). `[PROPOSAL]`
pub const PRODUCT_QUEUE_PACKETS_AUDIO: u64 = 1024;
/// The video half of the same mirror.
pub const PRODUCT_QUEUE_PACKETS_VIDEO: u64 = 512;
```
What is wrong: The shipped CLI consumes MIRROR CONSTANTS for product.toml `[limits]` (real values at product.toml:203-204: 1024/512 — matching today). The comment itself cites "product.toml:199-200", a stale line reference (199-200 are the section comment; the values live at 203-204). A product.toml limits change (schema range allows audio 64..=8192, video 64..=4096) silently does not reach the shipped binary — the queue posture is config-declared but code-mirrored.
Why it matters: The exact Wave-11 hostile question — config declared vs config consumed — in the playback path (host.rs:971-974 builds the queue plan from the constants). Drift is invisible until someone diffs the constant against the file by hand.
What correct looks like: Load `[limits]` via product-config at CLI startup (the recorded [PROPOSAL]), or add a drift-tie test that reads product.toml and asserts the constants equal it (the same pattern this repo already uses for help goldens).
Confidence: HIGH — Verification: read (demux_host.rs, host.rs, product.toml, product.schema.json ranges).

### F-004 [S4]
Where: crates/product-config/src/runtime.rs:71-74 vs product.schema.json:120-125
Code (product.schema.json:122):
```
          "pattern": "^(\\{product_name\\}|\\{media_title\\}|[^{}]|\\{\\{)*$",
```
What is wrong: The schema's title_template pattern admits `{{` escape sequences, but `resolve_window_title` performs only two literal `str::replace` calls — a legal template `{{` is emitted verbatim as `{{`, never unescaped to `{`. The `{{`/`}}` escape semantics are defined for the CATALOG message format (doc 07 §6.2, localization crate), not for title_template; the schema pattern silently imports the escape grammar without a renderer implementing it. The desktop's duplicate renderer (window_config.rs:123-127) shares the omission — consistently wrong in both.
Why it matters: A rebrand config author reading the schema regex will reasonably use `{{` to render a literal brace and get doubled braces in the window title. Limited blast radius (titles rarely need literal braces), hence S4.
What correct looks like: Either implement escape unescaping in both renderers or tighten the schema pattern to reject `{{` (making the two grammars explicitly different).
Confidence: HIGH — Verification: read (schema pattern vs replace-only implementation; no test covers `{{` in title_template).

### F-005 [S4]
Where: crates/product-config/src/runtime.rs:71-74; product.schema.json:21-25
Code (product.schema.json:21-25):
```json
        "name": {
          "type": "string", "minLength": 1, "maxLength": 40,
          "not": { "enum": ["VLC", "VideoLAN", "vlc"] },
```
What is wrong: `identity.name` has no charset/pattern constraint — a name containing `{media_title}` or `{product_name}` is schema-legal. Because the media branch replaces `{product_name}` first and `{media_title}` second, a brace-bearing product name gets template-expanded by the second replace (and a `{product_name}` inside the name double-expands). A media title containing `{product_name}` is NOT substituted (first replace already ran) — asymmetric behavior from replace ordering, not from a spec.
Why it matters: Theoretical today (no shipped caller, names are brand strings), but this is the kind of ordering subtlety that becomes a bug the day the function gains its first host. Documented-or-constrained is the fix.
What correct looks like: Add `"pattern": "^[^{}]+$"` to identity.name in the schema (braces are meaningless in brand names), or expand both variables in a single left-to-right scan.
Confidence: HIGH — Verification: read (schema, replace chain; no test covers brace-bearing names).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `InstallerIdentity` (struct) | 28-40 | REAL | — | Plain data; desktop_bundle_identifier format verified against tauri.conf.json |
| `AboutFacts` (struct) | 46-55 | REAL | — | Plain data |
| `resolve_window_title` | 65-77 | REAL | S3/S4 | Ignores `title_idle_key` (F-002); no `{{` unescape (F-004); replace-order asymmetry (F-005); media branch correct for the two legal variables |
| `log_line_prefix` | 82-84 | REAL | — | lowercased short_name per doc 07 §8.2; short_name pattern is `[A-Za-z0-9-]` so lowercase is total |
| `log_startup_line` | 89-94 | REAL | — | name + display_version + reverse_dns; no caller |
| `cli_version_line` | 99-101 | REAL | S2 (context) | Correct; contradicted by the shipped CLI's crate-version output (F-001) |
| `cli_usage_header` | 105-107 | REAL | S2 (context) | Correct; no caller; shipped usage header is a literal |
| `installer_identity` | 113-121 | REAL | — | Field-for-field derivation; tauri shape verified |
| `about_facts` | 126-133 | REAL | — | Field-for-field; logo path `assets/brand/logo.svg` — the asset does NOT exist (baseline config-validator: 4 violations include logo.svg) — config-validator ownership |
| tests: `the_shipped_product_toml_resolves_lumen_identity` | 143-167 | REAL | — | include_str! real product.toml — live drift-tie; asserts exact strings incl. "Big Buck Bunny — Lumen", "Lumen 0.1.3", "dev.lumen.desktop" |
| tests: `title_resolution_substitutes_only_the_legal_variables` | 173-212 | REAL | — | Hand-built Nova identity; template without media slot ignores the media arg |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `the_shipped_product_toml_resolves_lumen_identity` | YES | Real product.toml via include_str! (drift-tie: editing product.toml identity fails this test); exact-string assertions on every function's output. VERIFIED PASSING (orchrun log:4651) |
| `title_resolution_substitutes_only_the_legal_variables` | YES | A broken replace (e.g. substituting media into a slotless template) changes the result "Nova Player". VERIFIED PASSING (orchrun log:4640) |
| (acceptance) `tests/rebrand/rebrand_acceptance.rs` — 7 tests, Nova fixture via `#[path]` host (tests/rebrand_host.rs:13) | YES | Each surface asserted exactly (idle title, playing title, installer ids, About facts, version line "Nova Player 0.2.0-nova", usage header, log prefix "nova" + reverse_dns in startup line); an aggregate report test re-checks all surfaces. VERIFIED PASSING 7/7 (orchrun log:4721-4734). NOTE: these prove the FUNCTIONS, not any shipped surface — the report's own table marks CLI host wiring BLOCKED |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every surface reads `product-config` at run time" (window title, About, CLI --version/banner, log prefix, engineInfo, PWA, onboarding) | doc 07 §8.2:792 | FALSE | rg: zero production callers; CLI = CARGO_PKG_VERSION (host.rs:1631-1633) + literal usage header (help.rs:29-31); desktop = own parser/renderer (window_config.rs); only control-server engineInfo reads one field (F-001) |
| §8.3 step 3: "boot the CLI headlessly; assert `--version` prints `Nova Player …`" | doc 07 §8.3:817 | FALSE | Baseline rc-smoke-engine-only.log.txt:12 `engine-version: PASS (lumen 0.1.3)` — crate name+version, not identity; acceptance table itself says "host wiring BLOCKED" (rebrand_acceptance.rs:28) |
| `identity.name` → idle title + `{product_name}` in template | doc 07 §8.2:800 | PARTIAL | Product-config's idle = bare name (F-002); desktop path = idle KEY via webview i18n; both yield the product name only while the catalog template is `{product_name}` |
| `identity.ids.*` → installer columns; `{reverse_dns}.desktop` matches tauri.conf.json | doc 07 §8.2 + runtime.rs:110-111 | REAL | tauri.conf.json:5 `"dev.lumen.desktop"` == derived id (verified) |
| Module doc: "Hosts that do not exist yet … are recorded as honest gaps in the acceptance report, not faked here" | runtime.rs:19-21 | REAL | rebrand_acceptance.rs:24-30 table carries the BLOCKED markers; worklog archive-2026-10.md:8461 records the wiring handoff |
| Rebrand acceptance (§8.3 step 4) drives the resolvers with Nova | runtime.rs:9-11 | REAL | tests/rebrand/rebrand_acceptance.rs, 7/7 passing (orchrun log:4721-4734) |

## 7. Dead / duplicated / unreachable code
- All 7 functions: unreachable from shipped entry points (test-only). The crate's single runtime consumer (control-server) uses none of them.
- Duplicated logic: `resolve_window_title`'s media branch ≡ `apps/desktop/src/window_config.rs::render_media_title` (123-127) — same two-replace chain over the same template variables, implemented twice in two crates (and a THIRD copy of the variable-substitution idea exists in the localization crate's message renderer for `{name}` placeholders).
- `[window]` parsing duplicated: product-config's toml-crate model vs apps/desktop's std-only line-oriented reader (window_config.rs:191+) — the desktop reader exists by C5 std-only policy, recorded as a "seam" (window_config.rs:8-16), but it is still a second parser of the same table.

## 8. Dependencies & flags
- No dependencies beyond the crate's own model (`use crate::model::{Brand, Identity, Window}`). Pure functions; no I/O/clock; `#[must_use]` on all. No unsafe/unwrap/expect/panic (workspace lints).
- Context: product-config crate deps (toml, thiserror) — see the crate's Cargo.toml; the file under audit uses none of them directly.

## 9. Open questions for the next agent
- Who owns the rebrand-propagation card that wires the CLI banner (worklog:8461 handoff)? Until it lands, doc 07 §8.2/§8.3 rows should carry BLOCKED markers — a doc fix, not a code fix.
- F-003's mirror constants: is a drift-tie test (constants == product.toml [limits]) acceptable interim, or does the CLI load product-config at startup? The CLI's `default_value` [PROPOSAL] (host.rs:1562-1569, `DefaultValue::Ref(path)` → wire string `"-> product.toml {path}"`) also looks like a placeholder that leaks config internals to the wire — check with the settings-runtime agent.
- `about_facts.logo_svg` = "assets/brand/logo.svg" — the file does not exist (baseline INDEX row 10: 4 brand-asset violations). Does any About surface render it today (ui/src/screens/settings/areas/updates-about)? If yes, it renders a missing path.
- tauri.conf.json consumption of `WindowBuildSpec` (the desktop hands `WindowTitle::Idle{key}` to the wrapper) is UNVERIFIED — the tauri build is sandbox-blocked (baseline row 2). Flag as UNVERIFIED for the desktop/tauri wave.
