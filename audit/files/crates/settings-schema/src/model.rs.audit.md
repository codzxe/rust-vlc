# AUDIT: crates/settings-schema/src/model.rs
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-21 |
| Lines / functions | 619 / 20 fns + 3 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (chain: `lumen` CLI → apps/cli/src/host.rs:1402 `settings_schema::runtime::validate_value` → runtime.rs:82 `check_value` → model.rs:342; and `lumen --help` → help.rs:34/157 `generate_str`/`embedded()` → emit.rs → `screens_in_use`/`screen_entries`/`fmt_bound` at model.rs:33/46/496) |
| Findings | S0: 0 S1: 0 S2: 3 S3: 1 S4: 2 |
| Commands run | Grep `check_value|screens_in_use|screen_entries|fmt_bound` (callers: parse.rs:342, validate.rs:365, runtime.rs:82, emit.rs:73-362, bin/schema_gen.rs:149); Grep `settings_schema` in apps/ (host.rs:1371,1400,1402,1565; help.rs:34,157); Grep `restart_required|array_policy|deprecated_in|scope` across rs/ts/mjs (consumers found: host.rs:1379-1407, run.rs:650-662, UI schema.ts/SettingsProvider.tsx — but NONE for scope/array_policy/deprecated_in); `rg -c` on schemas/settings.toml (121 entries, 97 literal defaults, 28 default_refs, 0 `scope =`, 0 `array_policy =`, 0 `deprecated_in =`); awk over settings.toml (path/url defaults: `global.log-file`="", `interface.skin-file`="", `privacy.lastfm-submission-url`="", `recording.directory`="~/Videos"); Grep `.control()` repo-wide (only model.rs's own test); sed -n verification of every quoted range |

## 1. One-paragraph verdict
The typed model is real, reachable and executed: `check_value` is the single shared rule used by parse-time default checks (parse.rs:342), the D1 product join (validate.rs:365) and the shipped runtime G4 seam (host.rs:1402 → runtime.rs:82), and the ordering helpers feed the committed generators. Baseline runs all three of its tests green (orchrun-test-workspace-nofailfast.log.txt:4882-4885). But the model is a domain model of a config system that only half exists: (a) its own validator rejects the committed defaults of three entries (empty-string path/url), so those settings cannot round-trip through the G4 seam; (b) `DefaultValue::Ref` has no resolution path anywhere — the shipped consumer fabricates a placeholder string; (c) the three `[PROPOSAL]` extension fields (`scope`, `array_policy`, `deprecated_in`) are declared, parsed and emitted, then read by absolutely nothing — zero declarations in 121 seed entries, zero consumer code, contradicting doc 07 §4.1/§4.4 semantics the doc comments promise.

## 2. What actually works (verified)
- G4 write validation is wired and exercised: host.rs:1402 `settings_schema::runtime::validate_value(&full, &toml_value)` → runtime.rs:82 `check_value(def.ty, &def.domain, value)`; baseline tests `runtime::tests::validate_value_accepts_in_domain_values` / `validate_value_rejects_out_of_domain_values` pass (orchrun-test-workspace-nofailfast.log.txt:4896-4897, asserting V-ENUM `one of auto, forced-only, on, off`, V-RANGE `50..=2000`, V-TYPE).
- Type-tag round-trip and control mapping pinned by passing tests (`model::tests::type_tags_round_trip`, `control_selection_follows_doc_07_g1`, `screens_in_use_sorts_and_dedups` — nofailfast log 4882-4885).
- `check_value`'s Int acceptance of wire numbers is load-bearing and consistent: host.rs `setting_value_to_toml` (1596-1601) converts whole `f64` wire numbers to TOML integers, so `value_as_f64` (model.rs:486-492) accepting `Integer` is required for `float`/`percent` writes to validate — read the pair together; they match.
- `screens_in_use`/`screen_entries` ordering is consumed by all three committed generators (emit.rs:201,271,292-293,304,312,354-362) and by the shipped CLI help path (help.rs:34 `use settings_schema::{GenTarget, generate_str}`).
- `is_ranged`/`is_integer_valued` are consumed by parse.rs:514,582 (E-DOMAIN/E-TYPE enforcement: ranged types must carry `range`, integer types must have integer bounds) — so the silent `Ok(())` fallbacks in `check_range` (441-443) and the enum arm (398-400) are unreachable for schemas that passed parse.

## 3. Findings

### F-001 [S2] The model's own validator rejects the committed defaults of three settings — empty-string path/url defaults cannot round-trip
Where: crates/settings-schema/src/model.rs:367-396 (with schemas/settings.toml:577-597, parse.rs:334-339)
Code:
```rust
        SettingType::Path => match value.as_str() {
            Some(s) if !s.is_empty() => Ok(()),
            Some(_) => Err(violation(
                "V-FORMAT",
                "a non-empty path string",
                value,
                "paths cannot be empty",
```
```rust
        SettingType::Url => match value.as_str() {
            Some(s) if s.starts_with("https://") || s.starts_with("http://") => Ok(()),
```
What is wrong: The committed schema seeds `default = ""` for `global.log-file` (path, settings.toml:162-163), `interface.skin-file` (path, 258-259) and `privacy.lastfm-submission-url` (url, 588-591). `check_value` rejects `""` for both Path ("paths cannot be empty") and Url (no http(s) prefix) with V-FORMAT. parse.rs has to carve a special exception to admit these very defaults — parse.rs:334-339: "w26 Lead: an EMPTY string default on a Path/Url entry is legal — it models \"unconfigured\" … Runtime set-setting stays strict (G4)." The model and the schema therefore disagree about the value domain, and parse papers over it.
Why it matters: The G4 seam makes the default an unreachable value: `config.set-setting global.log-file ""` (or clearing the Last.fm URL field) returns `invalid-argument` even though "" is the setting's own committed default. Read-then-write of the default fails: `config.get-setting` on an unset key returns `SettingValue::String("")` (host.rs:1565-1571 `DefaultValue::Literal` → `toml_to_setting_value`), and feeding that value back through set-setting is rejected. Settings do not round-trip — the exact hostile question of this wave. No test covers set-setting on an empty-defaulted path/url key, which is why this never fired.
What correct looks like: One rule in one place: either the domain admits empty (make `""` legal for path/url as "unconfigured" in `check_value` itself, drop parse's `empty_ok` bypass) or the seed uses a sentinel like `default = "none"` with an explicit nullable domain. The validator must accept every committed default — a schema whose defaults violate its own value rules is incoherent.
Confidence: HIGH — Verification: read (model.rs:367-396, parse.rs:334-339, settings.toml defaults extracted with awk; host.rs:1402/1565-1571). Not executed (no cargo allowed) — but the rejection path is a pure string predicate, no execution risk.

### F-002 [S2] `DefaultValue::Ref` has no resolution path — get-setting fabricates a placeholder string for 28 of 121 settings
Where: crates/settings-schema/src/model.rs:243-250 (with runtime.rs:18,55-57, apps/cli/src/host.rs:1565-1571)
Code:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum DefaultValue {
    Literal(toml::Value),
    Ref(String),
}
```
and the shipped consumer (host.rs:1565-1571):
```rust
fn default_value(def: &settings_schema::SettingDef) -> SettingValue {
    use settings_schema::model::DefaultValue;
    match &def.default {
        DefaultValue::Literal(value) => toml_to_setting_value(value),
        DefaultValue::Ref(path) => SettingValue::String(format!("-> product.toml {path}")),
    }
}
```
What is wrong: The model carries product.toml references (28 of 121 seed entries, e.g. `global.language` → `defaults.locale`, `interface.theme` → `defaults.theme`, `playback.preferred-audio-languages` → `playback.preferred_audio_languages`) but the crate provides no way to resolve them: `runtime.rs` embeds only `schemas/settings.toml` and calls `validate(&schema, None)` (runtime.rs:57) — the product join is explicitly deferred to the generator binary. So at runtime the only thing the shipped CLI can do with a Ref is print it.
Why it matters: `config.get-setting global.language` on an unset key returns the string `"-> product.toml defaults.locale"` — a value that (a) is not the default, (b) has the wrong wire type (String for an enum setting), and (c) cannot be written back: `config.set-setting global.language "-> product.toml defaults.locale"` fails V-ENUM. For 23% of the settings surface the "value" the engine reports is fabricated marker text. The lib.rs doc admits the gap is open ("a future direct call from `product-config` at load time … `[PROPOSAL — Lead decision]`") — that future is not this build. Same family as Wave 8's `audio.set-eq`: the seam exists, the backend does not.
What correct looks like: Either resolve refs at runtime (embed product.toml too, or have the model store the resolved literal at schema-gen time alongside the ref) or make get-setting return an explicit "default is product-level" sentinel in the result schema instead of a type-violating string pretending to be a value.
Confidence: HIGH — Verification: read (model.rs:243-250, runtime.rs:18/55-57, host.rs:1565-1571, 28 default_refs counted in settings.toml).

### F-003 [S2] `scope`, `array_policy`, `deprecated_in`: model fields with documented semantics that nothing consumes — declared config that is never read
Where: crates/settings-schema/src/model.rs:89-96, 272-310
Code:
```rust
    /// `user` (persisted, layer 4) or `session` (layer 6) — `[PROPOSAL]`.
    pub scope: Scope,
    /// Array merge policy — `[PROPOSAL]` (doc 07 §4.4 rule 3).
    pub array_policy: ArrayPolicy,
    /// VLC-style "show advanced" folding.
    pub advanced: bool,
    /// Version that deprecated this entry; empty = active.
    pub deprecated_in: String,
```
```rust
pub enum Scope {
    /// Persisted user preference (layer 4).
    #[default]
    User,
    /// Per-session transient value (layer 6).
    Session,
}
```
What is wrong: Grep over the whole repo (rs/ts/tsx/mjs) shows `def.scope` is read by zero code. The G4 write handler (host.rs:1390-1408) validates and stores into an in-memory `BTreeMap<String, SettingValue>` (host.rs:181) — nothing persists user preferences anywhere (the UI side is localStorage-mock per docs/feature-matrix.toml:2251-2252), and nothing routes by scope; doc 07 §4.1's promise "`config.set-setting` routes by the key's declared scope: user-scope keys persist (layer 4)" is unimplemented. `array_policy` is read by no merge code (product-config's merge.rs does not depend on this crate). `deprecated_in` is read by nothing. The seed itself declares none of them: `rg -c 'scope = |array_policy = |deprecated_in = ' schemas/settings.toml` → 0/0/0 across 121 entries. The UI generator drops all three (gen-settings-screens.mjs:245 mandated-field list and :337-356 entry map carry neither); only emit.rs's .d.ts type surface mentions them (emit.rs:258-264) — and the UI imports that .d.ts nowhere.
Why it matters: This is the wave's central question answered in the negative for three fields: config declared, never consumed. The model advertises persistence/merge/deprecation machinery that does not exist; any consumer trusting `scope: 'user'` to mean "persisted" is misled — the field is a constant `User` for every entry in the product, and even if an author wrote `scope = "session"` nothing would behave differently. Dead weight in the hot path of every parse/validate/emit of the schema.
What correct looks like: Either implement the semantics (a user-preferences file writer keyed on `scope`; merge honoring `array_policy`) or cut the fields until the behavior lands. Keeping them makes the generated .d.ts assert capabilities ("user (persisted, layer 4)") that the binary does not have.
Confidence: HIGH — Verification: read + repo-wide grep (0 consumers, 0 seed declarations, host.rs in-memory map, UI generator field list).

### F-004 [S3] `SettingType::control()` + `Control::as_str()` have zero production callers; G1's control union is a duplicated hardcoded list
Where: crates/settings-schema/src/model.rs:182-223 (with emit.rs:186-194, 226)
Code:
```rust
    pub fn control(self) -> Control {
        match self {
            SettingType::Bool => Control::Toggle,
```
```rust
    for control in [
        "toggle",
        "select",
        "slider",
        "input",
        "list",
        "path-picker",
        "duration",
    ] {
        out.push_str(&format!("  | '{control}'\n"));
    }
```
What is wrong: Repo-wide grep for `.control()` finds only model.rs's own test (539-549). The G1 generator emits the `SettingControl` union from a literal string list (emit.rs:186-194) that duplicates `Control::as_str()`'s seven spellings by hand, and emits no per-entry `control:` value at all — the .d.ts is types-only (lib.rs: "the value-bearing table … lands with card 4.E1"). The UI does its own type→control selection in TS (ui/scripts/gen-settings-screens.mjs / SettingField.tsx) and imports `schemas/generated/settings.d.ts` nowhere (grep: no matches in ui/).
Why it matters: The doc-07 §5.3 "G1 control selection" is implemented twice — once dead in Rust (this file), once live in TypeScript — with no mechanical tie. If the mappings drift (e.g. percent → dial in the frames), the Rust one silently rots while its unit test keeps asserting the old table. This also answers W01-A10's open question (settings-schema/Cargo.toml.audit.md §9): the two generator paths have already forked — the Rust .d.ts carries scope/arrayPolicy/deprecatedIn, the UI schema.ts does not.
What correct looks like: Either emit the per-entry control from `def.control().as_str()` in G1 and have the UI consume the generated artifact, or delete `control()`/`Control` from the model and keep the mapping where it is actually used.
Confidence: HIGH — Verification: read + repo-wide grep (`.control()` only in model.rs tests; `Control::` variants only in model.rs; no ui import of settings.d.ts).

### F-005 [S4] Integer range check loses precision above 2^53
Where: crates/settings-schema/src/model.rs:352-357
Code:
```rust
            let n = value
                .as_integer()
                .ok_or_else(|| violation("V-TYPE", "an integer", value, "write a whole number"))?;
            check_range(domain, n as f64, value, "an integer")
```
What is wrong: `i64 → f64` rounding for |n| > 2^53 makes the inclusive range comparison inexact at the extremes (e.g. a bound of 2^53+1 compared against 9007199254740992.0).
Why it matters: Low — the committed seed's ranges are small (50..=2000 for tick-interval-ms), but the API accepts arbitrary i64 and the doc rule is "rejected, never clamped"; an edge-value write near a huge bound can be mis-adjudicated.
What correct looks like: Compare in i64 when both `range.min`/`max` are integral (`fmt_bound` already treats bounds as integers when whole), falling back to f64 only for float-typed entries.
Confidence: HIGH (the truncation is definitional) — Verification: read. Realistic exposure LOW (seed ranges small).

### F-006 [S4] Url rejection hint contradicts the rule it documents
Where: crates/settings-schema/src/model.rs:382-389
Code:
```rust
        SettingType::Url => match value.as_str() {
            Some(s) if s.starts_with("https://") || s.starts_with("http://") => Ok(()),
            Some(_) => Err(violation(
                "V-FORMAT",
                "an http(s) URL string",
                value,
                "URLs start with `https://` (product URLs must be https, doc 07 §2.10)",
```
What is wrong: The code accepts `http://` while the hint asserts "product URLs must be https, doc 07 §2.10". Doc 07 §2.10 governs *product.toml* URLs (brand links); this is the *user settings* validator, where the rule is http(s)-only, not https-only. A user rejected for `ftp://` is told the wrong reason.
Why it matters: Diagnostics are this crate's product (every rule ships an expected/found/hint triple); a hint citing an inapplicable doc section misdirects the fix.
What correct looks like: Hint should say user-facing URL settings accept http(s) only; drop the §2.10 citation or scope it to product.toml validation.
Confidence: HIGH — Verification: read (rule vs hint text); doc 07 §2.10 located via the model comment itself.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SettingsSchema::entry` | 17-19 | REAL | — | Linear scan; called by runtime.rs:78,95 (G4 unknown-key + schema_meta). O(n) per write, n=121 — fine. |
| `SettingsSchema::len` | 22-24 | REAL | — | Used by lib.rs tests, runtime.rs:115 hint. |
| `SettingsSchema::is_empty` | 27-29 | REAL | — | Trivial and correct; no production caller found (clippy-clean because pub). |
| `SettingsSchema::screens_in_use` | 33-42 | REAL | — | sort+dedup; consumed by emit.rs:201,271,292,304,354 + schema_gen.rs:149. Test-pinned. |
| `SettingsSchema::screen_entries` | 46-60 | REAL | — | (section, order, key) sort — doc 07 §5.4 order; consumed by emit.rs:138,293,312,362. |
| `SettingType::parse` | 131-148 | REAL | — | Rejects unknown incl. `list<bool>` (test). Consumed by parse.rs. |
| `SettingType::as_str` | 151-167 | REAL | — | Round-trip test-pinned; NOTE host.rs:1613-1623 `setting_type_name` re-implements this match by hand (diverging: `List(_) => "list"`) — duplication outside this file. |
| `SettingType::is_integer_valued` | 170-172 | REAL | — | Consumed by parse.rs:582 (integer bound enforcement). |
| `SettingType::is_ranged` | 175-180 | REAL | — | Consumed by parse.rs:514 (E-DOMAIN). |
| `SettingType::control` | 185-195 | REAL (dead in production) | S3 | Zero production callers (F-004); test-only. |
| `Control::as_str` | 212-222 | REAL (dead in production) | S3 | Duplicated by emit.rs:186-194 hardcoded list (F-004). |
| `Domain`/`ValueRange` types | 226-241 | REAL | — | f64 bounds; carries no unit (ms vs raw) — unit lives in SettingType; OK. |
| `DefaultValue` enum | 246-250 | REAL | S2 | Ref variant unresolvable at runtime (F-002). |
| `UiPlacement` | 254-260 | REAL | — | `ui` flag consumed (emit.rs screen_is_internal, UI gen mjs:353). |
| `CliSpec` | 265-270 | REAL | — | `flag` consumed (help gen, host.rs:1382); `short` emitted (emit.rs:252 cliShort) but the CLI never parses short flags for settings (help.rs lists long only) — noted, mild. |
| `Scope` + `as_str` | 273-290 | REAL (dead in production) | S2 | Declared, never read anywhere (F-003). |
| `ArrayPolicy` + `as_str` | 293-310 | REAL (dead in production) | S2 | No merge code reads it (F-003). |
| `ValueViolation` Display/Error | 314-337 | REAL | — | Renders `rule/expected/found/hint`; consumed via ValueError Display in host.rs error path. |
| `check_value` | 342-433 | REAL | S2 | Shared G4/E-DEFAULT rule — the core of the file; rejects committed empty defaults (F-001), i64→f64 truncation (F-005), misleading Url hint (F-006). List arm recursion at 427-429 correctly re-uses element type with outer domain (list<enum> domain is the member set — parse.rs:610-614 populates it). |
| `check_range` | 435-457 | REAL | — | Inclusive bounds, rejects never clamps; unreachable-for-valid-schema `Ok(())` fallback at 441-443 guarded by parse E-DOMAIN. |
| `violation` (helper) | 459-471 | REAL | — | Pure constructor. |
| `render_toml_value` | 474-484 | REAL | — | Arrays/tables render without contents — acceptable for diagnostics. |
| `value_as_f64` | 486-492 | REAL | — | Integer acceptance is load-bearing for the CLI wire path (host.rs:1596-1601). |
| `fmt_bound` | 496-504 | REAL | — | 1e15 guard + i64 back-check; NaN falls through to `{v}`. Consumed by emit.rs:73,77,107,109. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `type_tags_round_trip` (510-535) | YES | Direct assert `parse(tag).as_str() == Some(tag)` over all 13 tags + rejects `list<bool>`/`bogus`; any parse/as_str mismatch flips it. Ran green: nofailfast log 4885. |
| `control_selection_follows_doc_07_g1` (537-550) | YES | Pins the 8-arm control mapping — but the mapping it pins has no production consumer (F-004), so the test guards dead code. Green: 4882. |
| `screens_in_use_sorts_and_dedups` (552-618) | YES | Hand-built 2-entry schema; removing `sort`/`dedup`/the screen filter or breaking `(section,order,key)` each fail an assert. Green: 4884. |
| Gap (not a test) | — | No test anywhere asserts `check_value` against the seed's empty path/url defaults — the exact case F-001 breaks; parse.rs's `empty_ok` bypass exists to keep that case invisible to the E-DEFAULT test path. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "G4 … `config.set-setting` validates every write (type/range/enum) before applying — `invalid-argument`, never silent coercion" | doc 07 §5.3 G4 (docs/07-config-driven-system.md:585) | REAL | host.rs:1402 → runtime.rs:82 → model.rs:342; baseline runtime::tests green (nofailfast 4896-4897). |
| "config.set-setting routes by the key's declared scope: user-scope keys persist (layer 4), session-scope keys stay in memory (layer 6)" | doc 07 §4.1 (:470) | FALSE | `scope` read by zero code; no persistence writer exists; host.rs:1404 inserts into an in-memory BTreeMap only (F-003). |
| "Array merge policy … AppendUnique (plugin enable-lists)" (doc 07 §4.4 rule 3) | model.rs:292-299 + doc 07 §4.4 | PAPER-ONLY | No merge code consumes `array_policy`; 0 seed entries declare it (F-003). |
| "product-level defaults are stated once, in product.toml, and referenced by the schema" (D1) | model.rs:243-245 | SHALLOW | True in data (28 refs) but the runtime cannot resolve them — get-setting returns "-> product.toml …" placeholder (host.rs:1569, F-002). |
| "G1 control selection: bool→toggle, enum→select, …" | doc 07 §5.3 G1; model.rs:182-195 | SHALLOW | Mapping exists in Rust but is consumed by nothing; the live UI computes controls in TS from its own generator (F-004). |
| "All ten mandated slots are always populated after parsing" | model.rs:63-65 | REAL | parse.rs mandates the ten fields (E-FIELD); lib.rs committed-seed test asserts loader carries every row (lib.rs:175-179). |
| "w26 Lead: an EMPTY string default on a Path/Url entry is legal" | parse.rs:334-337 comment | REAL (and self-incriminating) | The carve-out exists because check_value rejects what the seed commits (F-001). |

## 7. Dead / duplicated / unreachable code
- Dead in production: `SettingType::control`, `Control::as_str` (test-only callers, F-004); `Scope`, `ArrayPolicy` (declared-parsed-emitted, never read, F-003); `deprecated_in` (never read, always empty).
- Duplicated: control spellings (model.rs:212-222 vs emit.rs:186-194); type tags (model.rs:as_str vs host.rs:1613-1623 `setting_type_name`, which diverges for List); the whole G1 surface vs ui/scripts/gen-settings-screens.mjs (already forked on scope/arrayPolicy/deprecatedIn).
- Unreachable entry points: none — the model itself is reached from the shipped CLI via two independent chains (G4 validation, help generation).

## 8. Dependencies & flags
Only `toml::Value` (external) is used in this file — allowlisted (boundaries.toml per W01-A10 audit). No unsafe, no unwrap/expect, no feature gates. Workspace lints inherited (baseline #5 clippy clean).

## 9. Open questions for the next agent
- `config.reset-setting` is declared in the commands catalog (crates/commands/src/catalog/command.rs:425-426, ids.rs:162) and in the UI wire types, but grep finds NO handler in apps/ — whoever owns host.rs/dispatch must determine what reset-setting does today (likely a dispatch error) and whether it can reset F-001's empty-default entries.
- Does apps/control-server's `config.set-setting` (e2e.rs:332) route into the same host.rs `setting_set` or reimplement validation? If the latter, check_value has a third caller to reconcile.
- The UI persistence seam is localStorage-mock (docs/feature-matrix.toml:2251-2252) — the "real config.set-setting bridge" is an open handoff; when it lands, F-001's empty-string rejection will bite UI path/url fields immediately.
