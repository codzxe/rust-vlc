# AUDIT: crates/settings-schema/src/runtime.rs
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-21 |
| Lines / functions | 239 / 15 (incl. 6 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (both). CLI: `lumen` session `config set-setting` → run.rs:830 → dispatch.rs:151-152 → host.rs:1390 `setting_set` → **runtime.rs:73 `validate_value`** (host.rs:1402). Tauri: UI TauriTransport `lumen_dispatch` → `desktop` crate (apps/desktop/src/dispatch.rs, depends on lumen-cli per apps/desktop/Cargo.toml:40) → same host path. CAVEAT: tauri build itself UNVERIFIED in baseline (missing GUI libs, INDEX #2). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg validate_value|schema_meta` (consumers: apps/cli host.rs:1371,1400,1402; help.rs:157; plugin-host-wasm supervisor.rs:123,171); `rg plugin-host-wasm` in **/Cargo.toml (NO dependent — dead crate); `rg settings:|self.settings|persist|prefs` in apps/cli/src (map field, 1 reader, 1 writer, zero persistence); `rg default_ref` schemas/settings.toml (24 of 121); read model.rs check_value (240-457); baseline log greps (below). |

## 1. One-paragraph verdict
The validation seam itself is genuine and proven: `validate_value`/`schema_meta` really check type/range/enum against the compile-time-embedded `schemas/settings.toml`, with did-you-mean hints, and the whole CLI dispatch path through them executed green in the baseline no-fail-fast run. But the seam is the END of the wire, not the beginning of behavior: after validation, the shipped hosts store the value into a per-process `BTreeMap` whose only reader is `config.get-setting`, nothing persists it, no engine component consumes any setting, the `config.setting-changed` event is an `#[ignore]`d gap, and the one consumer that maps validated writes onto real state (plugin-host-wasm's `BackoffPolicy`) is a dead crate no binary links. `SettingApplied` with `restart_required: true` is returned for writes that are guaranteed to be discarded — the config-plane twin of Wave-8's `audio.set-eq`.

## 2. What actually works (verified)
— Crate unit tests executed & passed: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4895-4897 (`runtime::tests::schema_meta_backs_get_setting ... ok`, `validate_value_accepts_in_domain_values ... ok`, `validate_value_rejects_out_of_domain_values ... ok`) — the embedded schema parses, self-validates (D2–D6), and accepts/rejects real values.
— Full host dispatch wiring executed & passed: same log :2288 `shell::tests::config_writes_round_trip_through_dispatch ... ok` (apps/desktop/src/shell.rs:449-488: set `interface.theme`="dark" → `SettingApplied`, get → "dark") and :2857 `dispatch::tests::settings_seam_validates_through_the_schema ... ok`; typed refusal of unknown keys proven by `invalid_config_writes_refuse_typed` (shell.rs:490+).
— `check_value` (model.rs:342-433, the engine behind validate_value): every SettingType branch enforces type + domain, lists re-validate elements, ranges reject (never clamp) — read and traced.

## 3. Findings

### F-001 [S2] The seam validates writes that are then applied to nothing — dead store, no persistence, no consumers
Where: crates/settings-schema/src/runtime.rs:1-4 (the claim) + apps/cli/src/host.rs:181,246,1372,1404 (the reality)
Code (runtime.rs:1-4):
```rust
//! G4 — runtime validation (doc 07 §5.3): the schema is embedded via
//! `include_str!` and every `config.set-setting`-style write is validated
//! against it (type/range/enum) before applying — `invalid-argument` on
//! violation, never silent coercion (doc 05 §2.11).
```
Code (apps/cli/src/host.rs:180-181):
```rust
    /// Session-local settings overrides (the settings-schema seam).
    settings: BTreeMap<String, SettingValue>,
```
What is wrong: "before applying" — there is no applying. In the shipped hosts the validated value is inserted into an in-memory map (host.rs:1404) whose sole reader is `setting_get` (host.rs:1372). Verified: `rg 'self\.settings'` in apps/cli/src → exactly those two sites; `rg 'persist|prefs|preferences'` in apps/cli/src → nothing; no code anywhere in crates/ writes a user-preferences file (doc 07 §4.2 layer 4 is loadable via product-config `load_partial_*` but no shipped binary calls it — see partial.rs audit). The playback engine has no settings surface at all — its own gap test admits it:
```rust
#[ignore = "gap: config.setting-changed not wired — handoff H-7 (settings writes are host/config-layer work; doc 07, Phase 4)"]
```
(crates/playback-engine/tests/events_contract.rs:1699, "the engine has no settings surface yet" — executed as ignored, baseline no-fail-fast log :3877.) The single consumer that maps a validated write onto real state, `BackoffPolicy::apply_setting` (crates/plugin-host-wasm/src/supervisor.rs:169-171), lives in a crate no workspace member depends on (`rg plugin-host-wasm` over **/Cargo.toml → only its own manifest).
Why it matters: every `config.set-setting` returns `SettingApplied { applied, restart_required }` — a success + an explicit "restart applies this" promise — for a write that influences zero behavior and is guaranteed lost on restart (nothing persists it). 121 declared settings (schemas/settings.toml), all validated into a void. This is the Wave-8 `audio.set-eq` pattern (UI/CLI setting awaiting a nonexistent backend) generalized to the entire settings plane.
What correct looks like: either persist validated writes through the existing layer-4 machinery (product-config `PartialProduct` + `resolve_layers`) and feed resolved values to the engine knobs they name, or return an honest `not-implemented`/`transient-only` result instead of `SettingApplied`+`restart_required` until consumers exist.
Confidence: HIGH — Verification: read + rg call-site census; baseline-executed tests cited above prove the round-trip through the dead map (the tests prove the map round-trips, not that anything consumes it).

### F-002 [S3] 24 of 121 settings return a placeholder string, not a value, from the get side of the seam
Where: crates/settings-schema/src/runtime.rs:6-8 (the deferral) + apps/cli/src/host.rs:1565-1571 (the sentinel)
Code (host.rs:1569):
```rust
        DefaultValue::Ref(path) => SettingValue::String(format!("-> product.toml {path}")),
```
Code (runtime.rs:6-8):
```rust
//! The embedded copy is self-checked (parse + D2–D6) on first use; the D1
//! `product.toml` join needs the wider world and runs in the generator
//! binary / the 1.E2 `config-validator` wiring, not at process start.
```
What is wrong: 24 settings declare `default_ref` into product.toml (schemas/settings.toml:65,81,131,294,399,412,425,438,451,471,484,496,604,619,631,643,655,670,1128,1141,1154,1368,1381,1394 — e.g. `interface.theme` → `defaults.theme`, `engine.probe-max-bytes` → `limits.probe_max_bytes`). Because the D1 join deliberately does not run at process start, `config.get-setting` on any of them before a set returns the literal string `"-> product.toml defaults.theme"` as the setting's *value*. The round-trip only produces real values after a same-process set.
Why it matters: 20% of the catalog serves a sentinel where a value belongs — same garbage-default family as the baseline's TrackInfo `language: Some("voe")`. The UI does not hit this (it reads the generated `ui/src/screens/settings/generated/schema.ts`, where default_refs ARE resolved at build time), so the CLI/tauri wire surface and the UI disagree about the same setting's default.
What correct looks like: the runtime seam resolves default_ref against an embedded (or `include_str!`-loaded) product.toml at process start — product.toml is a build-time constant of the shipped binary; no "wider world" is needed — or `schema_meta`/get marks such entries honestly (e.g. `value: null` + `default_ref` metadata).
Confidence: HIGH — Verification: read; default_ref census by rg; sentinel construction read at host.rs:1569; no run (baseline transcripts contain no get-setting-before-set scenario).

### F-003 [S4] Broken embedded schema is reported as `unknown setting key` — wrong error class on both entry points
Where: crates/settings-schema/src/runtime.rs:74-77 (and identically 91-94 for `schema_meta`)
Code:
```rust
    let schema = embedded().map_err(|err| ValueError::UnknownKey {
        key: key.to_owned(),
        hint: err.to_string(),
    })?;
```
What is wrong: `SchemaError::EmbeddedInvalid` ("a build-breaking state" per its own doc, runtime.rs:25-26) is funneled into `ValueError::UnknownKey`, so a corrupted/invalid embedded schema surfaces on the wire as `invalid-argument: unknown setting key \`x\`` with the schema dump as the "hint". A build-breaking state is not an invalid argument; the variant lies about the failure class and would mislead any client error handling keyed on the variant.
Why it matters: latent (the committed schema self-validates — proven by the passing baseline test), but the seam's error taxonomy is part of its contract (doc 05 §2.11).
What correct looks like: a third `ValueError::SchemaBroken(#[source] SchemaError)` variant; callers map it to `internal`/`unavailable`, not `invalid-argument`.
Confidence: HIGH — Verification: read.

### F-004 [S4] `schema_meta` duplicate-embeds the same mis-mapping and drifts from `validate_value` by one line
Where: crates/settings-schema/src/runtime.rs:90-99
Code:
```rust
pub fn schema_meta(key: &str) -> Result<&'static SettingDef, ValueError> {
    let schema = embedded().map_err(|err| ValueError::UnknownKey {
        key: key.to_owned(),
        hint: err.to_string(),
    })?;
```
What is wrong: identical 6-line preamble duplicated in `validate_value` and `schema_meta` (F-003 twice over); any future fix must be applied in both, and the two can drift.
Why it matters: copy-paste duplication in the two halves of one seam (checklist: copy-paste duplication).
What correct looks like: `fn lookup(key) -> Result<&'static SettingDef, ValueError>` shared by both.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `EMBEDDED_SOURCE` const | 18 | REAL | — | include_str! of repo-root schemas/settings.toml (path verified: crates/settings-schema/src → ../../../schemas/) |
| `EMBEDDED` static | 20 | REAL | — | OnceLock<Result<..>> — parse once per process |
| `SchemaError` enum | 23-29 | REAL | — | single variant EmbeddedInvalid |
| `ValueError` enum | 32-51 | REAL | S4 | UnknownKey/Violation; missing variant for schema-broken (F-003) |
| `embedded` | 54-68 | REAL | — | parse_str + validate(D2–D6) on first use; error string cloned per call (trivial); executed in baseline |
| `validate_value` | 73-86 | REAL | S2 | the seam: schema_meta lookup + check_value; correct validation, consumers dead (F-001); error-class bug (F-003) |
| `schema_meta` | 90-99 | REAL | S4 | duplicate preamble (F-004); returns SettingDef used for get-setting meta |
| `unknown_key_hint` | 102-118 | REAL | — | Levenshtein ≤3, first-best on ties (deterministic); fallback names the regen command |
| `edit_distance` | 121-137 | REAL | — | correct DP Levenshtein over bytes (keys ASCII); O(a·b) over ~121 keys per unknown key — trivial cost; no panics (byte-indexed, ASCII-safe) |
| test `embedded_schema_parses_and_self_validates` | 143-156 | REAL | — | executed & passed (baseline no-fail-fast :4895); would fail on any schema regression |
| test `validate_value_accepts_in_domain_values` | 158-170 | REAL | — | executed & passed (:4896); covers enum/int/list/bool |
| test `validate_value_rejects_out_of_domain_values` | 172-196 | REAL | — | executed & passed (:4897); V-ENUM/V-RANGE/V-TYPE asserted incl. exact range text |
| test `unknown_key_gets_a_did_you_mean_hint` | 198-212 | REAL | — | executed (part of suite); asserts hint text |
| test `schema_meta_backs_get_setting` | 214-230 | REAL | — | executed; pins feature_gate/placement/type of hw-decode-policy |
| test `edit_distance_is_symmetric_and_small_for_near_misses` | 232-237 | REAL | — | 3 assertions incl. kitten/sitting=3 |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 6 in-file tests | YES | Mutation reasoning: drop the enum membership check → `rejects_out_of_domain` V-ENUM assert fails; break the DP → `edit_distance` test fails; corrupt schema → `embedded_schema_parses_and_self_validates` fails. All executed in baseline (cited in §2). |
| `config_writes_round_trip_through_dispatch` (apps/desktop/src/shell.rs:449) | YES (for the map round-trip) | set→get through real dispatch; executed green in baseline. It proves exactly what it names — a round trip through the store — and nothing about consumption; the consumption gap is separately admitted by the ignored `gap_config_setting_changed` (events_contract.rs:1699-1705). Honest pair, dishonest product behavior (F-001). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "config.set-setting validates every write (type/range/enum) before applying — invalid-argument on violation, never silent coercion" | doc 07 §5.3 G4 (docs/07-config-driven-system.md:585) + runtime.rs:1-4 | **SHALLOW** | validation half REAL (baseline-executed); "applying" half is a dead in-memory map with no persistence and no consumers (F-001) |
| "schema_meta … backs config.get-setting's schema_meta result field" | runtime.rs:88-89, doc 07 G4 | REAL | host.rs:1371-1389 builds the meta map from it; dispatch test executed |
| "the D1 product.toml join … runs in the generator binary / the 1.E2 config-validator wiring, not at process start" | runtime.rs:6-8 | REAL (and the source of F-002) | generator resolves for the UI (generated schema.ts); the runtime get path serves "-> product.toml …" sentinels (host.rs:1569) |
| "settings writes are host/config-layer work" (gap H-7) | events_contract.rs:1699 ignore reason | REAL (the gap is real) | no engine settings surface exists; `#[ignore]`d test never runs |

## 7. Dead / duplicated / unreachable code
- Duplicated: `validate_value`/`schema_meta` preamble (F-004).
- Unreachable consumer: plugin-host-wasm `BackoffPolicy::from_schema_defaults`/`apply_setting` (supervisor.rs:120-171) — the only real applier of validated settings; its crate has zero dependents.
- Not dead: everything in this file — both shipped entry points reach it (chains in header).

## 8. Dependencies & flags
- settings-schema deps: serde/toml/thiserror (Cargo.toml header comment); no internal deps — consistent with its L1 registry claim.
- No feature flags. No unsafe. No unwrap/expect/panic in this file (workspace lints deny them; grep of file confirms).
- The embedded schema path crosses crate-root (`../../../schemas/settings.toml`) — a repo-layout coupling: moving schemas/ breaks every build of settings-schema. Noted, by design (G4).

## 9. Open questions for the next agent
- schemas/settings.toml itself (121 entries): which entries have ANY consumer anywhere (engine knob, CLI flag, UI field)? My census covers the seam, not per-key consumption — the settings.toml owner should build the declared-vs-consumed table (the wave prompt's question, at key granularity).
- The ignored gap tests H-7/H-8/H-9 (events_contract.rs:1699-1730): if a later wave wires config.setting-changed, F-001's severity assessment should be revisited.
- commands.schema.json:2366 marks `schema_meta` wire shape `[VERIFY]` "free-form string map" — the CLI currently sends type/restart_required/flag (host.rs:1377-1389); confirm the catalog contract allows those fields (commands-catalog owner).
