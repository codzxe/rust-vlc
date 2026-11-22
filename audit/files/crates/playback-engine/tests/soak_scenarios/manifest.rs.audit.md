# AUDIT: crates/playback-engine/tests/soak_scenarios/manifest.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 974 / 15 production fns + 3 types + 12 test fns |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (soak-suite config loader) — loaded by the soak_scenarios test binary (`SOAK_SCENARIOS` env, default `tests/soak/scenarios.toml`); execution UNVERIFIED (no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | Read (1-760 line-by-line; 761-975 test section via inventory + structure); sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The soak manifest loader: a strict, dependency-free TOML-subset parser and validator where every refusal is a human sentence naming file, line/section, key, and the legal set — unknown keys, unknown sections, unknown array tables, duplicate keys, duplicate scenario ids, malformed ids, wrong types, out-of-range values, wrong kind-axes, missing files, and even "a suite with no scenarios is a ritual, not a measurement" are all loud errors, never defaults. Range checks mirror the JSON schema field by field (sample_ms [1,600000], warmup_fraction (0,0.5], duration_min (0,10080], etc.), floats must be finite, multi-line arrays refused per the in-tree reader rule, and the schema version is pinned on both sides. The 12 tests exercise the real read+parse path through temp files and pin both the accept and every major refuse lane. One hygiene note: post-validation `unsigned_abs()` calls are defensive no-ops that obscure the already-enforced invariants.

## 2. What actually works (verified)

- Loud-refusal discipline (the module doc's "garbage never silently becomes 2 minutes", 12-13) is implemented at every layer: parse_value rejects unterminated strings, embedded quotes, trailing text, multi-line arrays, non-finite floats (328-393); load rejects unknown sections/tables (240-255), non-key lines (257-261), empty/duplicate keys (263-278), unknown top-level keys (283-289), schema drift (290-296), duplicate ids (303-309), and the empty suite (313-318).
- Range checks mirror the schema: every `[defaults]` knob and every `[[scenario]]` field is bounds-checked with the schema's exact intervals (546-600, 669-713), including the exclusive-lower bounds the error messages state correctly ("outside (0, 0.5]").
- Kind-axes enforcement (715-727): duration-driven kinds require `duration_min`, count-driven require `cycles` — the schema's per-kind `required` rule.
- The id pattern check (631-644) implements `^[a-z0-9][a-z0-9-]*$` with the 64-char cap.
- Tests hit the real path: `load_text` writes a temp file and loads it (757-760); the 12 tests cover the minimal accept, overrides, and each refusal family (787-975 inventory: unknown keys/section/top-level, schema check, duplicate ids, kind axes, unknown kind/bad id, ranges, multi-line arrays, empty fixtures, missing file with the path in the sentence).

## 3. Findings

### F-001 [S4] Post-validation `unsigned_abs()` calls are defensive no-ops that hide the invariants
Where: crates/playback-engine/tests/soak_scenarios/manifest.rs:601-612, 732-735
Code (601-604):
```rust
    Ok(Defaults {
        sample_ms: sample_ms.unsigned_abs(),
        warmup_fraction,
        min_cycles: min_cycles.unsigned_abs(),
```
What is wrong: `sample_ms`, `min_cycles`, `fd_growth_budget`, `live_growth_budget_bytes`, `cycles`, `advance_polls`, and `seeks_per_reopen` are all range-checked as non-negative (or ≥1) BEFORE these `unsigned_abs()` calls, which are therefore identity functions. A reader concludes negative values might reach here (they cannot), and a future edit that weakens a check upstream would silently launder a negative through `unsigned_abs` into a huge positive instead of failing.
Why it matters: Hygiene: the conversion implies an invariant the validation already guarantees; better to convert with `u64::try_from(value).expect("range-checked")`-style locality or restructure the checks on the unsigned type.
What correct looks like: Validate on the target type (`u64::try_from` in the check itself) and drop the abs calls.
Confidence: HIGH — Verification: read (check order verified).

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `Kind::parse` | 141-153 | REAL | Enum with legal-set error. |
| `Kind::label` | 157-165 | REAL | Stable report label. |
| `Kind::is_duration_driven` | 169-171 | REAL | Axes split. |
| `load` | 214-324 | REAL | Section walk + every refusal lane (§2). |
| `parse_value` | 328-393 | REAL | Strict value grammar; finite floats. |
| `raw_int`/`raw_float`/`raw_bool`/`raw_str`/`raw_str_array` | 396-484 | REAL | Required extractors with type errors. |
| `raw_opt_int`/`raw_opt_float` | 487-517 | REAL | Optional; int→float coercion documented. |
| `DEFAULTS_KEYS`/`SCENARIO_KEYS` (consts) | 522-533, 618-627 | REAL | `pub` for the loader/schema drift check. |
| `parse_defaults` | 537-613 | REAL | Field-by-field range mirror; F-001 abs calls. |
| `id_is_well_formed` | 631-644 | REAL | Pattern + length. |
| `parse_scenario` | 647-738 | REAL | Keys whitelist, ranges, kind axes; F-001. |
| 12 test fns | 787-975 | REAL | Real-file accept/refuse coverage per family. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All 12 | YES | Each refusal test feeds a concrete violation and asserts the error sentence family; the accept tests pin parsed values (e.g., seeks_per_reopen override + range at 802-828). Real temp-file I/O, not string-level parsing only. |

Serializer discipline (751-753) documented with the allocator rationale.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "unknown keys, wrong types, out-of-range values and duplicate ids are errors, never defaults" (10-13) | module doc | REAL | Every lane verified (§2). |
| "The shape contract is `tests/soak/scenarios.schema.json` … this loader enforces the same contract in Rust" (8-11) | module doc | REAL-UNPROVEN | Field-by-field mirror verified where I read the checks; the JSON schema file itself is another agent's file (loader/schema drift check exists via the `pub` KEY consts). |
| "Multi-line arrays are refused loudly (the boundaries.toml reader's rule)" (22-23) | module doc | REAL | 347-353 + test 943. |
| "garbage never silently becomes 2 minutes" (12-13, the SOAK_MINUTES discipline) | module doc | REAL | No defaulting anywhere: every required key missing is an error (407-409 etc.). |

## 7. Dead / duplicated / unreachable code

The `unsigned_abs()` no-ops (F-001). Nothing else.

## 8. Dependencies & flags

std + testkit only. `SOAK_SCENARIOS` env selects the manifest at the binary root (not this file). No feature gates.

## 9. Open questions for the next agent

- soak_scenarios binary root (main.rs — not my file): does it run the loader/schema drift check using the `pub` KEY consts against the committed JSON schema, and are the three profiles (smoke/sandbox/nightly) actually committed as three files?
- W19: which docs cite the 4 h nightly tier — has it ever run? (The manifest plane existing is not evidence a nightly ran.)
