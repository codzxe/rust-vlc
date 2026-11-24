# AUDIT: crates/settings-schema/src/model.rs
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-23 |
| Lines / functions | 619 / 19 (+3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — lumen-cli: main → run.rs:165 `lumen_dispatch` → dispatch.rs:151 `ConfigSetSetting` → host.rs:1390 `setting_set` → settings-schema runtime.rs:73 `validate_value` → model.rs:342 `check_value` (apps/cli/Cargo.toml:61 depends on settings-schema; baseline cargo-build-release-lumen-cli.txt exit 0). Desktop reuses the same `CliHost` (apps/desktop/src/dispatch.rs doc header "the SAME host the CLI drives"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 6 |
| Commands run | rg callers (check_value/control/is_empty/Scope::as_str/ArrayPolicy::as_str); sed quote verification; baseline cargo-test-workspace.txt:5389 `settings-schema exit_code=0` (43 unit + 8 integration passed) |

## 1. One-paragraph verdict
This is the real settings model and the real value validator. `check_value`
(type/range/enum/format) has three live callers — parse.rs:342 (E-DEFAULT
default-literal check), validate.rs:365 (D1 product.toml join), and
runtime.rs:82 (the G4 runtime seam) — and the runtime seam is wired into the
shipped lumen-cli `config.set-setting` path, so **ranges ARE enforced at
runtime** (answer to the wave's hostile-focus question). The defects are dead
surface, not fake logic: the entire G1 "UI control hint" chain
(`SettingType::control` → `Control` → `Control::as_str`) is emitted to nobody,
`Scope::as_str`/`ArrayPolicy::as_str` and the `scope`/`array_policy` fields are
parsed-then-never-read, and the runtime validator silently passes
type/domain-mismatched entries instead of failing hard.

## 2. What actually works (verified)
— check_value callers verified by `rg check_value` → parse.rs:342,
validate.rs:365, runtime.rs:82; runtime chain to CLI verified by reading
apps/cli/src/host.rs:1390-1416 (`setting_set` calls
`settings_schema::runtime::validate_value` before storing).
— baseline audit/runs/baseline/cargo-test-workspace.txt:5389: settings-schema
exit_code=0, 43 unit + 8 integration tests passed, including runtime.rs
V-RANGE test asserting `50..=2000` rejection (runtime.rs:182-189).

## 3. Findings
F-001 [S3]
Where: crates/settings-schema/src/model.rs:185-195 (+ 210-222)
Code:
```
    pub fn control(self) -> Control {
        match self {
            SettingType::Bool => Control::Toggle,
            SettingType::Enum => Control::Select,
            SettingType::Int | SettingType::Float | SettingType::Percent => Control::Slider,
            SettingType::String | SettingType::Url => Control::Input,
            SettingType::Path => Control::PathPicker,
            SettingType::List(_) => Control::List,
            SettingType::DurationMs => Control::Duration,
        }
    }
```
What is wrong: dead end-to-end. `rg '\.control\(\)'` finds zero callers
outside its own unit test (model.rs:538-549). The G1 TS artifact
(schemas/generated/settings.d.ts) only *declares* `control: SettingControl;`
(emit.rs:226) as an interface field; no emitter ever writes a per-entry
control value, and nothing in apps/ imports SettingEntry/SettingsByScreen
(`rg` over apps/: no hits). The doc comment claims "The UI control hint
(doc 07 §5.3 G1 control selection)" — the UI never receives it. A unit test
proves dead code works. Why it matters: G1 is claimed delivered; the control
selection logic ships to no consumer. What correct looks like: emit per-entry
`control` values into a consumed artifact, or delete the chain.
Confidence: HIGH — Verification: ran rg over repo; read emit.rs G1 emitter
(lines 180-273, interface-only, function ends without emitting data).

F-002 [S4]
Where: crates/settings-schema/src/model.rs:397-400 and 441-443
Code:
```
        SettingType::Enum => {
            let Domain::Enumeration(members) = domain else {
                return Ok(());
            };
```
```
    let Domain::Range(range) = domain else {
        return Ok(());
    };
```
What is wrong: a type/domain mismatch (enum without enumeration, ranged type
without range) silently validates as Ok — accept-anything fallback in the
function the docs call the single enforcement point. Mitigated by parse-time
E-DOMAIN checks (parse.rs:514-548) and by runtime.rs:57 self-validating the
embedded schema, but `check_value` is a pub re-export (lib.rs:60); any future
caller with an unvalidated schema gets a no-op validator. What correct looks
like: return a `V-DOMAIN` violation (or debug_assert) on mismatch.
Confidence: HIGH — Verification: read parse.rs:505-548 (E-DOMAIN) and
runtime.rs:53-68 (self-check).

F-003 [S4]
Where: crates/settings-schema/src/model.rs:352-357
Code:
```
        SettingType::Int | SettingType::DurationMs => {
            let n = value
                .as_integer()
                .ok_or_else(|| violation("V-TYPE", "an integer", value, "write a whole number"))?;
            check_range(domain, n as f64, value, "an integer")
```
What is wrong: i64→f64 cast loses precision above 2^53; an integer value
equal to a bound ±1 ULP beyond 2^53 mis-validates. Current schema ranges are
small (e.g. 50..=2000), so unreachable today; latent only.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/settings-schema/src/model.rs:486-492
Code:
```
fn value_as_f64(value: &toml::Value) -> Option<f64> {
    match value {
        toml::Value::Integer(n) => Some(*n as f64),
        toml::Value::Float(f) => Some(*f),
        _ => None,
    }
}
```
What is wrong: a `float`/`percent` setting silently accepts a TOML integer
(`250` for a float entry) — a type coercion, contradicting the module's own
"never silent coercion" contract (model.rs:312-313). Lenient-by-design
arguably, but the two float-typed entries (schemas/settings.toml:668,1112)
then report `applied` values that differ in TOML type from what was written.
Confidence: MEDIUM — Verification: read; no doc text found that permits int
for float.
F-005 [S4]
Where: crates/settings-schema/src/model.rs:382-389
Code:
```
        SettingType::Url => match value.as_str() {
            Some(s) if s.starts_with("https://") || s.starts_with("http://") => Ok(()),
            Some(_) => Err(violation(
                "V-FORMAT",
                "an http(s) URL string",
                value,
                "URLs start with `https://` (product URLs must be https, doc 07 §2.10)",
```
What is wrong: the check accepts plain `http://` while the fix hint asserts
"product URLs must be https, doc 07 §2.10". §2.10 governs product.toml
URLs, not user setting values — the hint imports an unrelated rule to justify
a check it doesn't implement. Misleading diagnostics.
Confidence: HIGH — Verification: read.
F-006 [S4]
Where: crates/settings-schema/src/model.rs:282-310; fields 89-92
What is wrong: `Scope::as_str` and `ArrayPolicy::as_str` have zero callers
(`rg '\.as_str'`/`scope.as_str|array_policy.as_str` → no hits); the parsed
`scope`/`array_policy` fields are never read by any generator or runtime
(emit.rs:258-260 hard-codes the TS union literals `'user' | 'session'` /
`'replace' | 'append-unique'` instead of emitting per-entry values). Parsed,
validated, dead. Doc comment "the `[PROPOSAL]` extensions carry their
documented defaults" (63-65) implies generators consume them; none does.
Confidence: HIGH — Verification: rg for `.scope`/`.array_policy` reads — only
parse.rs writes them.
F-007 [S4]
Where: crates/settings-schema/src/model.rs:553-618 (test)
What is wrong: `screens_in_use_sorts_and_dedups` puts its two entries on
DIFFERENT screens ("b" and "a"), so the composite sort at model.rs:52-58
(section → order → key) is never exercised with 2+ same-screen entries; the
multi-key tie-breaking chain is untested. Would not fail if the `then(order)`
or `then(key)` clauses were deleted or permuted.
Confidence: HIGH — Verification: read; mutation reasoning.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SettingsSchema::entry | 17-19 | REAL | — | linear scan; runtime.rs:78,95 |
| SettingsSchema::len | 22-24 | REAL | — | runtime.rs:115 |
| SettingsSchema::is_empty | 27-29 | REAL-UNPROVEN (unused) | S4 | zero tree callers |
| SettingsSchema::screens_in_use | 33-42 | REAL | — | emit.rs ×7, schema_gen.rs:149 |
| SettingsSchema::screen_entries | 46-60 | REAL | — | composite sort untested (F-007) |
| SettingType::parse | 131-148 | REAL | — | parse.rs |
| SettingType::as_str | 151-167 | REAL | — | round-trip tested |
| SettingType::is_integer_valued | 170-172 | REAL | — | parse.rs:582 |
| SettingType::is_ranged | 175-180 | REAL | — | parse.rs:514 |
| SettingType::control | 185-195 | UNREACHABLE | S3 | F-001: no consumer |
| Control::as_str | 212-222 | UNREACHABLE | S3 | only control() → dead chain |
| Scope::as_str | 284-290 | UNREACHABLE | S4 | F-006 |
| ArrayPolicy::as_str | 304-310 | UNREACHABLE | S4 | F-006 |
| check_value | 342-433 | REAL | — | 3 callers; runtime-enforced via CLI |
| check_range | 435-457 | REAL | S4 | silent-Ok fallback (F-002), NaN min silently unbounds (min>max parse check can't catch NaN) |
| violation | 459-471 | REAL | — | internal |
| render_toml_value | 474-484 | REAL | — | internal + re-export |
| value_as_f64 | 486-492 | REAL | S4 | F-004 int→float leniency |
| fmt_bound | 496-504 | REAL | — | emit.rs ×5; NaN renders "NaN" (unreachable via valid schema) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| type_tags_round_trip | YES | direct mapping assertions incl. rejects |
| control_selection_follows_doc_07_g1 | YES locally / proves nothing shipped | tests dead code (F-001) |
| screens_in_use_sorts_and_dedups | PARTIAL | dedup+screen-sort yes; (section,order,key) sort never exercised (F-007) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Shared by the parse-time literal-default check (E-DEFAULT) and the runtime config.set-setting validation (G4)" (339-341) | model.rs doc | REAL | parse.rs:342, runtime.rs:82, host.rs:1410 |
| "invalid-argument, never silent coercion" (312-313) | model.rs doc | MOSTLY | int-for-float coercion accepted (F-004); wording is model.rs's own |
| "UI control hint (G1)" | model.rs:182-184 | FALSE-ish | no consumer receives it (F-001) |

## 7. Dead / duplicated / unreachable code
`SettingType::control` + `Control` + `Control::as_str` (F-001);
`Scope::as_str`, `ArrayPolicy::as_str`, `SettingDef.scope`/`.array_policy`
consumers (F-006); `SettingsSchema::is_empty` unused in tree.

## 8. Dependencies & flags
toml (value model), thiserror only in runtime.rs (not here). No feature gates
in this file; settings-schema has no [features] gating model.rs (whole crate
ships in lumen-cli release build per baseline).

## 9. Open questions for the next agent
- parse.rs `value_as_number`: does it accept TOML `nan` for range bounds? If
  yes, `min = nan` passes the `min > max` parse check and silently disables
  the lower bound at runtime (check_range comparisons are all false vs NaN).
  Verify in parse.rs audit.
- Who, if anyone, will emit the G1 per-entry JSON (`SettingEntry` objects)?
  Interface exists (emit.rs:227-264), data does not.
