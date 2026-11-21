# AUDIT: crates/localization/src/catalog.rs
| Field | Value |
|---|---|
| Auditor | W11-A10 |
| Date | 2026-11-21 |
| Lines / functions | 461 / 30 (19 production items: 1 const, 2 structs, 1 enum, 12 fns/methods, 1 Display impl, 2 free fns + 2 loader fns; 11 test items) |
| File verdict | UNREACHABLE (code is REAL and baseline-test-executed; no shipped entry point links it) |
| Reachable from shipped entry point? | NO — proof: `rg -n "from_toml_str\|load_locales_dir\|load_catalog_file" --glob '!crates/localization/**' --glob '!audit/**'` over the whole repo → the ONLY hit outside the crate is a COMMENT in ui/src/screens/settings/areas/language/locales.ts:4 that names `load_locales_dir` as the resolver while the UI hardcodes `AVAILABLE_LOCALES: readonly string[] = ["en-US"]` (locales.ts:14). No `Cargo.toml` in the repo declares a dependency on `localization` (verified: `rg -n "localization" --glob '**/Cargo.toml'` → only the crate's own manifest + a comment in playlists). In-crate callers: audit.rs:21,309,377-506 and fallback.rs:189-218 (test code). Same crate-orphan status as key.rs (W11-A04 F-001), message.rs#part2 (F-001), fallback.rs (W11-A07). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 4 |
| Commands run | Read catalog.rs 1-461 (complete) · Read lib.rs 1-155 (line_col, re-exports), locales.ts 1-30 (the lying comment), locales/en-US.toml 1-30 · `rg -n "Catalog::from_toml_str\|load_catalog_file\|load_locales_dir\|from_toml_str" crates/localization/src --glob '!catalog.rs'` (in-crate callers: audit.rs, fallback.rs) · `rg -n "from_toml_str\|load_locales_dir\|load_catalog_file"` repo-wide minus crate/audit/docs (→ locales.ts:4 comment only) · `rg -n "localization" --glob '**/Cargo.toml'` (zero dependents) · `rg -n '\.keys()\|contains_key\|source_name\|\.len()' crates/localization/src/audit.rs crates/localization/src/fallback.rs` (accessors live in-crate) · `rg -n '#.*"' locales/en-US.toml` (no comment-embedded quoted dotted keys — needle false-hit is latent) · baseline `orchrun-test-workspace-nofailfast.log.txt:2683-2691` (all 9 catalog::tests `ok`), `:2760-2771` (seed_integrity 3/3 `ok`, incl. `shipped_locales_tree_passes_every_catalog_audit_rule`) · `sed -n` verification of every quote below |

## 1. One-paragraph verdict
The catalog loader is genuinely and carefully built: flat-catalog enforcement (nested tables and non-string values get typed errors with a fix hint), key-grammar and message-format validation per entry, duplicate keys delegated to the TOML parser with span-extracted key names, and a directory scanner with per-file failure collection — all proven by nine green unit tests plus the seed-integrity suite over the real `locales/` tree in the baseline no-fail-fast run. It is also machinery bolted to nothing: no crate in the workspace depends on `localization`, so neither `lumen` nor `lumen-tauri` nor any tool ever calls `load_locales_dir`; the one codebase that names it (the UI's language picker) does so in a comment while hardcoding `["en-US"]` in TypeScript. Within the file itself, the directory scanner silently swallows per-entry `read_dir` errors (inconsistent with its own failure-collection contract), and two diagnostic helpers are fragile: message-format line attribution finds the FIRST quoted occurrence of the key (comments and value substrings can steal it), and duplicate-key extraction string-matches the toml crate's error wording. This is the config-system's declared-vs-consumed disease in its purest form: a complete, tested catalog pipeline whose only runtime consumer is its own test suite.

## 2. What actually works (verified)
— Verified by baseline `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2683-2691` → all nine `catalog::tests::*` `ok` (`parses_flat_catalog`, `duplicate_keys_are_parse_errors_with_position`, `non_string_values_are_rejected`, `nested_tables_are_rejected_with_hint`, `invalid_keys_are_rejected`, `message_errors_carry_file_position`, `empty_catalog_is_valid`, `loads_a_locales_directory`, `placeholder_set_shape`), and `:2765-2771` → `tests/seed_integrity.rs` 3/3 `ok` including `shipped_locales_tree_passes_every_catalog_audit_rule` — i.e. the shipped `locales/en-US.toml` parses and passes all five audit rules through THIS file's loaders.
— Verified by trace: `from_toml_str` (86-131) handles every value shape — String → parse, Table → `NestedTable` with the write-it-flat hint, other → `NonStringValue` with `value_type_name` (exhaustive match, 188-198); `MessageKey::new` errors map 1:1 to typed `CatalogError` variants (109-128). An empty quoted TOML key (`"" = "x"`, legal TOML) lands in `KeyError::Empty` → `InvalidKey { key: "" }` — handled, no panic.
— Verified: `extract_key_at` (224-231) clamps both slice bounds (`span.start.min(src.len())`, `span.end.min(src.len())`) — no panic path on out-of-range spans; `catalog_message_error`'s `key_line + value_line - 1` cannot underflow (`line_col` is 1-based, lib.rs:123-124).
— Verified: duplicate keys ARE rejected by the TOML parser itself (module doc claim, line 5-6) — pinned by `duplicate_keys_are_parse_errors_with_position` asserting exact `line: 3, column: 1, key: "a.b"`, green in baseline.

## 3. Findings

### F-001 [S2]
Where: crates/localization/src/catalog.rs:289-329 (whole-file reachability)
Code:
```
/// Load every `*.toml` file directly inside `dir`. Non-TOML files are
/// skipped (documented: the directory holds catalogs only).
pub fn load_locales_dir(dir: &Path) -> LoadedCatalogs {
```
What is wrong: `load_locales_dir` — the doc-07 §6.3 catalog-loading mechanism — has zero callers outside `crates/localization`'s own audit.rs/tests. The shipped binaries never link this code: no workspace manifest depends on `localization` (`rg` over every Cargo.toml). The only external mention is a comment that MISREPRESENTS the relationship: ui/src/screens/settings/areas/language/locales.ts:3-6 says "the localization crate (crates/localization) resolves catalogs by scanning the repo `locales/` directory (load_locales_dir, catalog.rs)" — but the UI never invokes it; the file hardcodes `AVAILABLE_LOCALES: readonly string[] = ["en-US"]` (locales.ts:14), and the UI's real l10n pipeline is a separate TypeScript stack (`ui/src/i18n/messages.ts`, checked by `npm run l10n:check`, baseline #24 PASS).
Why it matters: this is the wave's hostile question answered — config mechanism declared, tested, documented as load-bearing, and consumed by no shipped artifact. `locales/en-US.toml` (the declared "source of truth for the key set", file header line 4) is parsed by Rust only inside this crate's CI tests. Two grammars now exist (Rust kebab newtype here, reimplemented in TS l10n-check) with no cross-check, free to drift silently.
What correct looks like: either a shipped consumer (CLI/UI) links the crate and derives its locale list/messages from `load_locales_dir`, or the crate is demoted to a CI-only auditor and the locales.ts comment is corrected to say the list is maintained by hand.
Confidence: HIGH — Verification: read + repo-wide caller greps (commands listed above) + baseline #3/#24. Cross-refs: key.rs F-001 (W11-A04), message.rs#part2 F-001, fallback.rs (W11-A07) — same crate-orphan status, independently re-verified here.

### F-002 [S3]
Where: crates/localization/src/catalog.rs:304-308
Code:
```
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| match entry {
            Ok(entry) => Some(entry.path()),
            Err(_) => None,
        })
```
What is wrong: a per-entry `read_dir` iteration error (platform I/O hiccup mid-scan, permission change on the directory) is silently discarded — the affected catalog file simply does not appear in `catalogs` NOR in `failures`. This is inconsistent with the function's own error handling one screen up: a directory-LISTING failure (291-302) is recorded in `failures`, and every per-FILE load failure (322-325) is recorded — only the per-ENTRY failure vanishes.
Why it matters: the function's contract (`LoadedCatalogs { catalogs, failures }`) exists so its caller — the audit gate (audit.rs:309) — can report every problem. A swallowed entry error means a catalog silently drops out of the audit: missing-key/orphan/placeholder violations in that file go unchecked, which is precisely the failure mode the five audit rules exist to prevent. The crate's own lint policy bans swallowed errors as a class (lib.rs:93-97: "All failure paths are typed errors or recorded diagnostics").
What correct looks like: push the `Err(e)` into `loaded.failures` with the directory name as context, exactly like the listing-failure branch.
Confidence: HIGH — Verification: read (both sibling branches traced); not executed (would require injecting a mid-iteration IO fault — no harness, cargo runs forbidden).

### F-003 [S4]
Where: crates/localization/src/catalog.rs:242-243
Code:
```
    let needle = format!("\"{raw_key}\"");
    let file_line = src.find(&needle).map(|byte| line_col(src, byte).0);
```
What is wrong: message-format error line attribution finds the FIRST occurrence of the quoted key string anywhere in the file. A comment mentioning `"a.b"` above the real definition, or an earlier message whose VALUE contains the quoted key, steals the match and the diagnostic points at the wrong line (and only the line is compensated — the column is the message-relative column, already approximate).
Why it matters: diagnostics-only impact (wrong line number in `CatalogError::MessageFormat`), but this file's stated purpose is "actionable diagnostics" (module doc line 7-9). Latent in the shipped tree: `rg -n '#.*"' locales/en-US.toml` shows no comment-embedded quoted dotted keys.
What correct looks like: search for the key in the value position — e.g. `format!("\"{raw_key}\" =")` (keys are always followed by ` =` in these flat catalogs), or scan assignment lines via a tiny index like settings-schema's LineIndex.
Confidence: HIGH — Verification: read + trace of first-match semantics; en-US.toml checked clean of the triggering pattern.

### F-004 [S4]
Where: crates/localization/src/catalog.rs:209-213, 224-231
Code:
```
    let key = if message.contains("duplicate key") {
        span.map(|span| extract_key_at(src, span))
    } else {
        None
    };
```
What is wrong: duplicate-key detection string-matches the toml crate's human-readable error text. If a toml upgrade rewords "duplicate key" (it is not a stable API), the `key` field of `CatalogError::Parse` silently degrades to `None` — no test would fail, since no test pins the None path. Relatedly, `extract_key_at` (224-231) scans forward to the first `=`, so a quoted key CONTAINING `=` (`"a=b" = 1` duplicated) extracts `"a` — mangled key name in the diagnostic.
Why it matters: version-brittle plumbing with silent degradation, in the one code path (rule 2, doc 07 §6.2) the module doc advertises as "parse-level for free".
What correct looks like: parse the key from the span position without assuming wording (e.g. re-lex the token at the span), and pin the degradation path with a test asserting `key.is_some()` for a duplicate.
Confidence: MEDIUM — Verification: read; the toml crate's message stability is an external property I cannot verify in-repo (UNVERIFIED beyond code reading).

### F-005 [S4]
Where: crates/localization/src/catalog.rs:429-443 (test) vs 258-329 (untested error paths)
Code:
```
        let loaded = load_locales_dir(std::path::Path::new(dir));
        assert!(
            loaded.failures.is_empty(),
            "failures: {:?}",
            loaded.failures
        );
```
What is wrong: no test exercises the failure machinery this file exists to provide: `load_catalog_file`'s `InvalidLocaleTag` path (a `xx-!!`.toml file), its `Io` path (missing/unreadable file), `load_locales_dir`'s listing-failure branch (291-302), the swallowed-entry branch (F-002), or the non-TOML skip filter (309-314). `loads_a_locales_directory` asserts only the happy path over the real seed directory. The rule-2 surface through `audit.rs` fixtures (audit_proof_matrix.rs, 14 tests green in baseline) covers parse-level duplicates but not these loader paths.
Why it matters: the untested branches are exactly the ones that can silently misbehave (F-002's swallow would have been caught by a test feeding a failing entry iterator; the InvalidLocaleTag mapping is unpinned).
What correct looks like: tempdir-based tests: one `zz.toml` with a bad tag, one unreadable file, one `.md` file, asserting `failures` content and catalog count.
Confidence: HIGH — Verification: read (test bodies 331-461 enumerated against production branches).

### F-006 [S4]
Where: crates/localization/src/catalog.rs:176-186
Code:
```
impl fmt::Display for Catalog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "catalog {} ({} messages, {})",
```
What is wrong: `Display for Catalog` has no caller — in-crate grep shows `keys/contains_key/len/source_name/get` used by audit.rs and fallback.rs, but no `format!("{}", catalog)` / `.to_string()` on a `Catalog` anywhere (the Display hits in refs.rs/plural.rs are other types). Dead impl inside an already-unlinked crate.
Why it matters: dead code; mild API surface noise for a crate whose entire public API is unconsumed.
What correct looks like: delete it, or use it in `load_locales_dir`'s failure diagnostics where a catalog summary would add context.
Confidence: HIGH — Verification: read + in-crate grep (commands above); external callers impossible (zero dependents, F-001).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `REFERENCE_LOCALE` (const) | 23 | REAL | — | "en-US"; consumed by audit.rs:21 and tests. Matches doc 07 §6.3. |
| `CatalogError` (enum, 7 variants) | 27-68 | REAL | — | thiserror-typed, every variant carries context (key/line/column or source). No lossy conversion. |
| `Catalog` (struct) | 72-76 | REAL | — | BTreeMap keyed by validated `MessageKey` — duplicate insertion impossible post-grammar (and in-file dupes die at TOML parse). |
| `Catalog::from_toml_str` | 81-138 | REAL (UNREACHABLE shipped) | — | Flat enforcement order is deliberate and correct (value shape BEFORE key grammar, comment 91-94, so `[a]` gets the flat hint not a grammar error). First-error-wins per file (no full-list promise — contrast settings-schema). No panics. |
| `Catalog::locale` | 141-143 | REAL | — | Trivial accessor; used by audit.rs sort and fallback.rs. |
| `Catalog::source_name` | 146-148 | REAL | — | Used in every audit.rs issue row (135, 162, …). |
| `Catalog::get` | 151-153 | REAL | — | BTreeMap get; used by fallback.rs translate. |
| `Catalog::contains_key` | 156-158 | REAL | — | Used by audit.rs rules 1/2 (133, 145, 160). |
| `Catalog::keys` | 159-163 | REAL | — | BTreeMap order = deterministic; doc claim "lexicographic order (deterministic diagnostics)" accurate. |
| `Catalog::len` / `is_empty` | 166-173 | REAL | — | Used by audit.rs:149 (coverage totals) / empty-catalog test. |
| `impl Display for Catalog` | 176-186 | REAL (dead) | S4 (F-006) | No caller in repo. |
| `value_type_name` | 188-198 | REAL | — | Exhaustive match over toml::Value; returns the exact names the error text promises ("integer" pinned by test). |
| `catalog_parse_error` | 202-220 | REAL | S4 (F-004) | span→line/col via lib.rs line_col; `unwrap_or((1,1))` fallback unpinned; wording-match fragility F-004. |
| `extract_key_at` | 224-231 | REAL | S4 (F-004) | Bounds clamped (no panic); `=`-in-key mangles extraction; trim_matches('"') handles the normal quoted case (pinned green by duplicate test). |
| `catalog_message_error` | 235-254 | REAL | S4 (F-003) | First-occurrence needle search (F-003); math `key_line + value_line - 1` underflow-safe (line_col 1-based). |
| `load_catalog_file` | 258-276 | REAL (UNREACHABLE shipped) | — | stem→Locale::parse→typed InvalidLocaleTag; read_to_string→typed Io with context; delegates to from_toml_str. Error paths untested (F-005). |
| `LoadedCatalogs` (struct) | 280-285 | REAL | — | pub fields; Default derive; consumed by audit.rs:309. |
| `load_locales_dir` | 289-329 | REAL (UNREACHABLE shipped) | S3 (F-002) | Sorting: paths sorted (316) then catalogs re-sorted by locale (327) — deterministic either way; non-TOML skip documented (287-288); entry-error swallow F-002; is_file() follows symlinks (fine). |
| `tests::en_us` | 338-340 | REAL | — | Helper. |
| `tests::parses_flat_catalog` | 342-356 | REAL | — | Discriminating: len, ordered keys, contains_key. Baseline ok. |
| `tests::duplicate_keys_are_parse_errors_with_position` | 358-375 | REAL | — | Pins exact line 3 / column 1 / key "a.b" — would fail on span or extraction regression. Baseline ok. |
| `tests::non_string_values_are_rejected` | 377-386 | REAL | — | Pins NonStringValue{found:"integer"}. Baseline ok. |
| `tests::nested_tables_are_rejected_with_hint` | 388-397 | REAL | — | Pins NestedTable{key:"a"} — proves the value-shape-first ordering works. Baseline ok. |
| `tests::invalid_keys_are_rejected` | 399-409 | REAL | — | Pins InvalidKey with "kebab-case" reason. Baseline ok. |
| `tests::message_errors_carry_file_position` | 411-420 | REAL | — | Pins MessageFormat line 1. Baseline ok. |
| `tests::empty_catalog_is_valid` | 422-427 | REAL | — | Comment-only file → empty catalog Ok. Baseline ok. |
| `tests::loads_a_locales_directory` | 429-443 | REAL | — | Live-data test on the real `locales/` dir (CARGO_MANIFEST_DIR-relative); asserts failures empty + exactly 1 catalog + en-US. Will (correctly, per its comment) fail when a second locale lands. Baseline ok. |
| `tests::placeholder_set_shape` | 445-460 | REAL | — | Pins placeholder_set BTreeMap equality incl. typed placeholder `number,percent`. Baseline ok. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| parses_flat_catalog | YES | Exact key-vector and len asserts; any parse/insert/order bug breaks it. |
| duplicate_keys_are_parse_errors_with_position | YES | Three exact fields (line 3, column 1, key "a.b") — a broken span mapping, wording match, or extractor each fail a distinct assert. |
| non_string_values_are_rejected | YES | Variant + found-name pinned. |
| nested_tables_are_rejected_with_hint | YES | If the value-shape check were removed, `[a]` would surface as InvalidKey (different variant) → matches! fails. |
| invalid_keys_are_rejected | YES | Variant + reason substring pinned. |
| message_errors_carry_file_position | YES | line 1 pinned; a broken needle search returning None still yields line 1 here (single-line file) — weak but the column/variant still discriminate. Partial. |
| empty_catalog_is_valid | YES | A regression rejecting empty tables fails it. |
| loads_a_locales_directory | YES | Any loader regression on the real tree (or a corrupt en-US.toml) flips failures/len. Also functions as a tree-state tripwire. |
| placeholder_set_shape | YES | Exact BTreeMap equality incl. placeholder type tags. |
All nine green in baseline no-fail-fast (orchrun log 2683-2691). Honest suite; gaps are the loader error paths (F-005), not tautologies.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Duplicate keys are rejected by the TOML parser itself (rule 2's in-file half, 'parse-level for free')" | catalog.rs:5-6; doc 07 §6.2 | REAL | duplicate-keys test green in baseline (orchrun 2683) with exact position/key extraction. |
| "nested tables and non-string values are rejected here with actionable diagnostics" | catalog.rs:7-9 | REAL | NestedTable/NonStringValue variants + two green tests (orchrun 2688-2689). |
| "the localization crate resolves catalogs by scanning the repo `locales/` directory (load_locales_dir, catalog.rs)" | ui/src/screens/settings/areas/language/locales.ts:3-4 | FALSE (as an account of the shipped system) | No caller of `load_locales_dir` exists outside the crate's tests (repo grep, F-001); the UI hardcodes `["en-US"]` (locales.ts:14). The scan runs only in `cargo test -p localization` (baseline #3). |
| "file name = BCP-47 tag" (doc 07 §6.3) | catalog.rs:256-257 doc comment | REAL | load_catalog_file:263-270 parses the stem via Locale::parse with typed InvalidLocaleTag. |

## 7. Dead / duplicated / unreachable code
- Whole file UNREACHABLE from shipped entry points (F-001) — executed only by `cargo test --workspace` / `-p localization` (baseline #3, no-fail-fast log 2683-2691, 2740-2771).
- `Display for Catalog` dead even in-crate (F-006).
- Duplicated mechanism: the TS l10n-check reimplements the key grammar (see key.rs audit F-001, W11-A04) — this file's flat-catalog rules have no TS counterpart to drift against because nothing else parses `locales/*.toml` at all.

## 8. Dependencies & flags
`toml` 1.1 + `thiserror` 2.0 — matches the crate's declared pre-audited set (Cargo.toml:8-12). No features, no cfg gates except `#[cfg(test)]`. Workspace lints inherited; no `unsafe`, no unwrap/expect in this file (verified by read). Baseline clippy CLEAN (INDEX #5).

## 9. Open questions for the next agent
- F-002's swallow could be tested only with a fault-injecting DirEntry iterator — if the orchestrator ever runs a mutation pass, `load_locales_dir` entry-error handling is the highest-value probe in this file.
- `catalog_gen` (crates/commands/src/bin/catalog_gen.rs) — despite the name it is NOT a localization catalog generator (it lives in `commands`); whoever audits it should confirm it doesn't shadow this crate's role. (Not verified here — out of scope.)
- locales/en-US.toml (W11-A0? file assignment): its header claims grammar "enforced by `localization::MessageKey`" — enforcement exists only in this crate's tests; the header's honesty depends on F-001's resolution.
