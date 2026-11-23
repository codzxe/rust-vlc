# AUDIT: crates/playback-engine/tests/soak_scenarios/manifest.rs
| Field | Value |
|---|---|
| Auditor | W03-A07 |
| Date | 2026-11-23 |
| Lines / functions | 974 / 29 (17 loader fns/consts/types + 12 tests + 2 test helpers + MINIMAL fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only module: `mod manifest;` in the integration-test binary crates/playback-engine/tests/soak_scenarios/main.rs:44. It IS wired end-to-end inside the test world: the same binary loads the shipped tests/soak/scenarios.toml (plus nightly/sandbox/forced-threshold variants, all present in tests/soak/) and runs the real soak suite (baseline: `soak_scenario_suite ... ok`, binary finished in 39.22s). Not compiled into lumen-cli/desktop. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p playback-engine --test soak_scenarios manifest::` -> 12 passed / 0 failed (exit 0, my run); baseline audit/runs/baseline/cargo-test-workspace.txt:4502-4539 (32 passed incl. all 12 manifest tests + shipped-manifest schema validation); sed/rg verifications. |

## 1. One-paragraph verdict
A genuinely strict, dependency-free TOML-subset manifest loader for the soak suite: unknown keys/sections/kinds, wrong types, out-of-range values, duplicate ids, duplicate keys, empty fixture lists, multi-line arrays and wrong schema versions all refuse with human sentences naming file/line/key — the "garbage never silently becomes a default" contract is implemented, not just claimed, and 12 parser tests plus the shipped-manifest schema check pass under my own run. Three defects, none corrupting a valid run: a malformed table header `[defaults` (missing `]`) is silently accepted as the defaults section — a strictness hole in a loader whose whole point is loud refusal; a provably dead `inner.contains('"')` check; and the duplicate-key refusal path (274-278) has no test pinning it.

## 2. What actually works (verified)
— `cargo test -p playback-engine --test soak_scenarios manifest::` -> `test result: ok. 12 passed; 0 failed` (exit 0). Range checks verified by reading: warmup_fraction/max_skip_rate use half-open `(0, 0.5]`/`(0, 1]` with the `|| x <= 0.0` term (553, 565); `inf`/`nan` floats rejected via `float.is_finite()` (384-386); `[[scenario]]` trailing text refused (231-235); `schema` required and pinned to 1 (290-296), matching tests/soak/scenarios.schema.json:11 (`"const": 1`). The shipped manifest is validated against that schema by the same binary (baseline `shipped_manifests_validate_against_the_schema_contract ... ok`).

## 3. Findings
F-001 [S3]
Where: crates/playback-engine/tests/soak_scenarios/manifest.rs:245-248
Code:
```
        if let Some(name) = line.strip_prefix('[') {
            let name = name.strip_suffix(']').unwrap_or(name);
            section = match name.trim() {
                "defaults" => Section::Defaults,
```
What is wrong: when the closing `]` is missing, `strip_suffix(']')` returns `None` and `unwrap_or(name)` keeps the UN-suffixed text — so the malformed header `[defaults` parses to name `"defaults"` and is silently accepted as the `[defaults]` section (identically, `[defaults x]`-shaped junk that happens to trim clean would be refused, but the plain missing-bracket typo is not). A typo'd manifest header should be a loud line-numbered refusal under this loader's own contract ("loud refusals ... never defaults", lines 10-13); instead it silently changes nothing observable and ships the run. Why it matters: this loader's entire value proposition is refusing malformed input; the header line is the one place it forgives. What correct looks like: `let Some(name) = line.strip_prefix('[').and_then(|n| n.strip_suffix(']')) else { return Err(...) }` (refuse any `[`-line that is not exactly `[name]`). Confidence: HIGH — Verification: read (Rust strip_prefix/strip_suffix semantics are unambiguous; no executed check because injecting this input requires editing repo test files, which is forbidden).
Confidence: HIGH — Verification: read / UNVERIFIED (executed)

F-002 [S4]
Where: crates/playback-engine/tests/soak_scenarios/manifest.rs:339-344
Code:
```
        let inner = &stripped[..close];
        if inner.contains('"') {
            return Err(format!(
                "{where_}: embedded quotes are not supported in {text:?}"
            ));
        }
```
What is wrong: `close` is `stripped.find('"')` (line 331-333) — the FIRST quote in the string — so `inner = stripped[..close]` contains no `"` by construction; the `contains` check is unreachable dead code and the "embedded quotes are not supported" error can never fire. (The equivalent array-item path at 364-370 has no such dead check.) What correct looks like: delete the dead branch (the first-quote rule already implements the no-embedded-quotes subset). Confidence: HIGH — Verification: read.
Confidence: HIGH — Verification: read

F-003 [S4]
Where: crates/playback-engine/tests/soak_scenarios/manifest.rs:274-278 (untested path); tests at 786-973
Code:
```
        if target.contains_key(key) {
            return Err(format!(
                "{source}:{line_no}: duplicate key {key:?} in the same section"
            ));
        }
```
What is wrong: the duplicate-key refusal has no test — none of the 12 tests writes a section with a repeated key, so a regression that silently lets the LAST (or FIRST) duplicate win would pass the whole suite. Same gap for the F-001 malformed-header shape. Blast radius: parser only. What correct looks like: one test with `sample_ms` twice in `[defaults]` asserting the refusal sentence. Confidence: HIGH — Verification: read (rg over the tests mod: no test constructs a duplicate key).
Confidence: HIGH — Verification: read

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Kind::parse | 141-153 | REAL | — | all 5 wire names + typed unknown error |
| Kind::label | 156-165 | REAL | — | stable report labels |
| Kind::is_duration_driven | 168-171 | REAL | — | axes split |
| load | 214-324 | REAL | S3 | strict line scanner; malformed `[defaults` header accepted (F-001) |
| parse_value | 328-393 | REAL | S4 | strings/ints/floats/bools/single-line arrays; dead contains check (F-002); inf/nan refused |
| raw_int | 396-411 | REAL | — | required, typed errors |
| raw_float | 414-430 | REAL | — | accepts int literal as float |
| raw_bool | 433-448 | REAL | — | — |
| raw_str | 451-466 | REAL | — | — |
| raw_str_array | 469-484 | REAL | — | — |
| raw_opt_int | 487-500 | REAL | — | — |
| raw_opt_float | 503-517 | REAL | — | — |
| DEFAULTS_KEYS (const) | 522-533 | REAL | — | pub for drift check |
| parse_defaults | 537-613 | REAL | — | full range mirror of the schema; unsigned_abs safe post-validation |
| SCENARIO_KEYS (const) | 618-627 | REAL | — | — |
| id_is_well_formed | 631-644 | REAL | — | ^[a-z0-9][a-z0-9-]*$, ≤64 |
| parse_scenario | 647-738 | REAL | — | kind axes enforced; defaults 1/16 for polls/seeks |
| tests::serial | 751-753 | REAL | — | whole-binary serializer (global allocator) |
| tests::load_text | 757-763 | REAL | — | real file-read path via temp dir |
| tests::minimal_manifest_parses_with_defaults | 786-799 | REAL | — | pins defaults 1/16 |
| tests::seeks_per_reopen_overrides_and_range_checks | 801-826 | REAL | — | override + both range ends |
| tests::unknown_keys_refuse_loudly | 828-840 | REAL | — | — |
| tests::unknown_section_and_top_level_refuse | 842-857 | REAL | — | — |
| tests::schema_version_is_checked | 859-865 | REAL | — | — |
| tests::duplicate_scenario_ids_refuse | 867-876 | REAL | — | — |
| tests::kind_axes_are_enforced | 878-899 | REAL | — | both directions |
| tests::unknown_kind_and_bad_id_refuse | 901-911 | REAL | — | — |
| tests::out_of_range_values_refuse | 913-940 | REAL | — | 5 key/value pairs, sentence asserts |
| tests::multi_line_arrays_refuse | 942-951 | REAL | — | — |
| tests::empty_fixture_list_refuses | 953-962 | REAL | — | — |
| tests::missing_file_refuses_with_the_path | 964-973 | REAL | — | — |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 12 parser tests | YES | each drives the real read+parse path through a temp file and asserts the typed refusal sentence or the parsed value |
| (coverage gaps) | — | duplicate-key-in-section refusal (274-278) and malformed `[defaults` header (F-001) have no test (F-003) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "unknown keys, wrong types, out-of-range values and duplicate ids are errors, never defaults" | manifest.rs:10-13 | REAL (with one hole) | tests 828-940 pass; F-001 shows the header line escapes this contract |
| "Multi-line arrays are refused loudly" | manifest.rs:22-23 | REAL | parse_value 347-353 + test 942 |
| "Full-line # comments only" | manifest.rs:23 | REAL | line 226 (`starts_with('#')` after trim); inline `#` lands in parse_value and refuses |
| "the CI-short smoke, the bounded sandbox profile and the 4 h nightly tier are three FILES" | manifest.rs:6-8 | REAL | tests/soak/{scenarios,scenarios-sandbox,scenarios-nightly}.toml all exist |
| "shape contract is tests/soak/scenarios.schema.json (draft 2020-12)" | manifest.rs:8-9 | REAL | file exists; `"const": 1` at schema line 11 == SCHEMA_VERSION; baseline `shipped_manifests_validate_against_the_schema_contract ... ok` |
| baseline 32 passed incl. soak run | audit/runs/baseline/cargo-test-workspace.txt:4502-4539 | REAL | my run of the manifest subset: 12 passed, exit 0 |

## 7. Dead / duplicated / unreachable code
- Dead `inner.contains('"')` branch (340-344) — F-002.
- The `.ok_or_else(|| ... "value before any [[scenario]]")` guard at 270-272 is defensive-unreachable (section Scenario(i) is only ever set right after a push) — acceptable belt-and-braces, noted not counted.

## 8. Dependencies & flags
Only `std` + `testkit::Must` (test mod). No serde/toml crate — hand-rolled subset parser by design (Lead-owned dependency-free rule, documented). No feature gates. Tests serialize via testkit::alloc_count::serialize_tests because the binary root installs the counting global allocator.

## 9. Open questions for the next agent
- Does runner.rs (same binary) treat a duplicate-key-tolerant or malformed-header manifest any differently (i.e., can F-001 change a soak run's meaning in practice)? runner.rs is another agent's file.
- The loader validates fixture path STRINGS only; existence is checked "when the plans are built" (comment at 101-103) — confirm runner.rs actually fails loudly on a missing fixture.
