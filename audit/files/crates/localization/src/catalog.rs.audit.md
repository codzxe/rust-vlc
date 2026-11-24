# AUDIT: crates/localization/src/catalog.rs
| Field | Value |
|---|---|
| Auditor | W11-A05 |
| Date | 2026-11-23 |
| Lines / functions | 461 / 13 fns + 9 tests |
| File verdict | REAL (crate UNREACHABLE from shipped entry points — see below) |
| Reachable from shipped entry point? | NO. No crate or app depends on `localization` (rg over all Cargo.toml: only its own manifest; the playlists hit is a comment). Mechanism-only L1 crate consumed by its own tests + xtask-side audit tooling; shipped lumen-cli/desktop never load a catalog. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `rg` reverse-deps (exit 0); `sed -n` quote checks (exit 0 x3); baseline `cargo-test-workspace.txt:2879-2977` cited (45 passed, exit 0) |

## 1. One-paragraph verdict
A real, tight flat-catalog loader: `toml::Table` parse (duplicate keys are TOML parse errors, and the duplicate-key extraction test proves it), value-shape check before key-grammar check so `[section]` nesting gets the flat-catalog hint, key validation via `MessageKey::new`, message parsing with file-line/column mapping. All 9 catalog unit tests plus the crate's 45-test suite pass in baseline. The defects are cosmetic diagnostics and a swallowed directory-entry error. The crate as a whole ships nothing to users yet — no dependency edge from any app — so every line here is dev-lane tooling today.

## 2. What actually works (verified)
— verified by `sed -n '2879,2977p' audit/runs/baseline/cargo-test-workspace.txt` -> localization: 45 passed / 0 failed, incl. all 9 `catalog::tests::*` (duplicate-key position, non-string, nested-table hint, invalid-key, message-format line, empty catalog, real `locales/` dir load, placeholder set). Exit code 0.

## 3. Findings
F-001 [S4]
Where: crates/localization/src/catalog.rs:242-243
Code:
```
    let needle = format!("\\"{raw_key}\\"");
    let file_line = src.find(&needle).map(|byte| line_col(src, byte).0);
```
What is wrong: `find` returns the FIRST occurrence of the quoted key text anywhere in the file. If an earlier message's text contains the literal `"a.b"`, or the same key substring appears inside another quoted key (e.g. `"a.b.c"` contains `"a.b"` only when quoted — the needle includes quotes so this exact case is safe, but a message value containing `"a.b"` is not), the reported line/column for a message-format error points at the wrong entry. Diagnostics-only; the error still fires. What correct looks like: parse with spans (toml_edit) or track offsets during iteration.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/localization/src/catalog.rs:304-308
Code:
```
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry.path()),
            Err(_) => None,
        })
```
What is wrong: an I/O error while iterating `read_dir` (the entry itself) is silently discarded — the file is invisible to the audit instead of landing in `failures`. Every other I/O failure in this file is surfaced; this one is swallowed. What correct looks like: push into `loaded.failures`.
Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/localization/src/catalog.rs:129-131
Code:
```
            let message = Message::parse(text)
                .map_err(|error| catalog_message_error(src, raw_key, text, &error))?;
            messages.insert(key, message);
```
What is wrong: single-error-per-load — the first bad key aborts the whole catalog, unlike settings-schema's "FULL diagnostic list is returned (every problem, one pass)" contract (parse.rs:31-32). Doc 07 §6 does not promise aggregation for catalogs, so this is a design asymmetry, not a contract breach. Also `messages.insert` can never overwrite silently because `toml::Table` already rejected duplicate keys.
Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CatalogError (enum) | 27-68 | REAL | — | typed, thiserror, 7 variants with actionable hints |
| Catalog::from_toml_str | 81-138 | REAL | — | shape-before-key ordering deliberate (L91-94 comment) |
| Catalog::locale / source_name / get / contains_key / keys / len / is_empty | 141-173 | REAL | — | trivial accessors |
| Display for Catalog | 176-186 | REAL | — | |
| value_type_name | 188-198 | REAL | — | |
| catalog_parse_error | 202-220 | REAL | — | span→line/col; duplicate-key text sniff (`message.contains("duplicate key")`) is toml-crate-internal-string coupling — brittle but tested |
| extract_key_at | 224-231 | REAL | — | forward scan to `=`; bounded by span.end fallback |
| catalog_message_error | 235-254 | REAL | S4 | F-001 first-occurrence needle |
| load_catalog_file | 258-276 | REAL | — | file stem must parse as BCP-47 |
| load_locales_dir | 289-329 | REAL | S4 | F-002 swallowed entry errors; deterministic sort by locale |
| tests (9) | 331-461 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_flat_catalog | YES | asserts len, key order, containment |
| duplicate_keys_are_parse_errors_with_position | YES | pins line 3, column 1, extracted key "a.b" |
| non_string_values_are_rejected | YES | pins variant + "integer" |
| nested_tables_are_rejected_with_hint | YES | pins NestedTable{key:"a"} |
| invalid_keys_are_rejected | YES | pins kebab-case reason |
| message_errors_carry_file_position | YES | pins line 1 |
| empty_catalog_is_valid | YES | comment-only file loads, is_empty |
| loads_a_locales_directory | YES | real repo `locales/` dir; asserts 0 failures AND exactly 1 catalog — adding a second shipped locale would fail this test (an honest pin, but it hardcodes today's single-locale state) |
| placeholder_set_shape | YES | pins {name:simple, p:number,percent} map |
Untested: `load_catalog_file`'s InvalidLocaleTag and Io paths directly (covered indirectly at most), `extract_key_at` no-`=` fallback, catalog_message_error's `None` branch (key not found in src).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Duplicate keys are rejected by the TOML parser itself (rule 2's in-file half)" | catalog.rs:5-6 | REAL | baseline `duplicate_keys_are_parse_errors_with_position` ok |
| "nested tables and non-string values are rejected here with actionable diagnostics" | catalog.rs:7-9 | REAL | baseline nested/non-string tests ok |
| doc 07 §6.3 "file name = BCP-47 tag" | catalog.rs:256-257 quoting doc | REAL | load_catalog_file:263-270 parses stem as Locale |
| en-US "shipped first, complete, the source of truth" (locales/en-US.toml:4-6) | locales/en-US.toml | REAL-UNPROVEN | catalog loads it (baseline); "complete" is enforced by audit rules (audit.rs — not my file, UNVERIFIED by me) |

## 7. Dead / duplicated / unreachable code
- No dead code in-file. The whole crate is currently unconsumed by any app (see reachability) — reachable only via `cargo test -p localization` and xtask audit tooling.

## 8. Dependencies & flags
- `toml = "1.1"` (parse), `thiserror = "2.0"` — both used, both declared in the header comment as pre-audited. No feature gates.

## 9. Open questions for the next agent
- `audit.rs` (the five §6.4 rules incl. plural conformance) and `fallback.rs`, `refs.rs`, `message.rs`, `key.rs` are not in my assignment — catalog.rs's correctness leans on Message::parse/MessageKey::new, which I did not audit line-by-line.
- When a UI-facing consumer lands, the first-occurrence needle (F-001) starts mattering for user-facing error files.
