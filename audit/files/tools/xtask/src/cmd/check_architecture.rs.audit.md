# AUDIT: tools/xtask/src/cmd/check_architecture.rs
| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-23 |
| Lines / functions | 2548 / 66 fns+methods, 12 in-file unit tests, 12 integration tests (tools/xtask/tests/check_architecture.rs) |
| File verdict | REAL (executed on the real workspace and on 12 hostile scratch fixtures; 24 tests run green; one executed S2 blind spot) |
| Reachable from shipped entry point? | NO for the release lane (not linked into lumen-cli/desktop). YES from the CI gate: `.github/workflows/ci.yml:238 cargo run -p xtask --locked -- check-architecture` and `scripts/full_gate.sh:29 cargo xtask check-architecture` → `.cargo/config.toml` alias → `tools/xtask/src/main.rs dispatch` → `tools/xtask/src/cmd/mod.rs:102-106 run: check_architecture::run`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 5 |
| Commands run | `cargo xtask check-architecture` → exit 1 (reproduced baseline arch failure byte-for-byte); `cargo test -p xtask --test check_architecture` → 12 passed/exit 0; `cargo test -p xtask --bin xtask check_architecture` → 12 passed/exit 0; 12 scratch-fixture invocations of `target/debug/xtask check-architecture --root audit/runs/scratch/<case>` (exit codes in findings below); sed/rg verification of every quote. |

## 1. One-paragraph verdict
This is a genuine, working architecture gate, not a paper gate: it reads every member manifest with a hand-rolled std-only TOML-subset parser, enforces five real checks (edge allowlist + layer ordering, cycle scan over normal+dev edges, lint inheritance, `static mut` text scan, dev-only direction), and I executed it — it reproduces the baseline failure exactly (`edges` FAIL on the undeclared `lumen-cli`→`streaming` edge, exit 1) and all 24 tests pass. Its teeth are real: my inline-form control fixture and all seeded violations are caught. But the gate can be fooled with valid Cargo.toml syntax: dependencies declared in table form (`[dependencies.foo]`) are invisible to ALL five checks — executed proof below (S2), a hole the test suite never probes because every fixture uses inline `key = { ... }` form. Secondary executed defects: CRLF manifests make the tool exit 2 (gate cannot run on Windows autocrlf checkouts), crate rules without `layer` silently skip layer-ordering enforcement, duplicate package names are only detected when their directories sort adjacently, and `unsafe_code = "forbid"` (stronger than deny) fails check 3. The baseline "1 of 5 checks failed" is the gate working correctly: the repo, not the tool, is in violation.

## 2. What actually works (verified)
- `cargo xtask check-architecture` → exit 1, `result: FAILURE (1 of 5 checks failed)`, `edges=fail cycles=pass unsafe-lints=pass static-mut=pass testkit-direction=pass`, `39 nodes, 143 internal edges`, `scanned 632 files across 39 src trees` — identical to audit/runs/baseline/full_gate.txt:41-52.
- Baseline failure identified (hostile-focus question): check 1 `edges`, single violation. `apps/cli/Cargo.toml:62` declares `streaming = { path = "../../crates/streaming" }`; `boundaries.toml:514` `[crates.lumen-cli].internal_deps` lists 15 crates and `streaming` is not among them. Cause: the CLI grew a streaming dependency without the Lead-reviewed boundaries.toml update the gate demands (fix hint at check_architecture.rs:1447-1449). Not a tool bug — a real catch. `streaming` is layer 2 (boundaries.toml:214), `lumen-cli` layer 6, so the hinted fix would also satisfy the layer check.
- `cargo test -p xtask --test check_architecture` → 12 passed, exit 0 (fixture acceptance proof: real binary, real scratch workspaces).
- `cargo test -p xtask --bin xtask check_architecture` → 12 unit tests passed, exit 0.
- Fixture runs proving the teeth: inline-form undeclared dep → exit 1 `[EDGE]`; same-layer non-whitelisted edge → exit 1 `[LAYER]`; `[target.'cfg(windows)'.dependencies]` undeclared dep → caught (exit 1, target tables ARE counted).
- Usage errors exit 2 (verified by test `usage_errors_exit_two` and my CRLF fixture: exit 2).

## 3. Findings
F-001 [S2]
Where: tools/xtask/src/cmd/check_architecture.rs:957-963
Code:
```rust
            let Some(last) = table.path.last() else {
                continue;
            };
            let table_kind = match last.as_str() {
                "dependencies" | "build-dependencies" | "dev-dependencies" => last.as_str(),
                _ => continue,
            };
```
What is wrong: a dependency declared in Cargo's table form — `[dependencies.beta]` / `path = "../beta"` — produces a table whose LAST header segment is `beta`, not `dependencies`, so it falls to `_ => continue` and is dropped from `Member.deps`. The edge is invisible to check 1 (allowlist + layers), check 2 (cycles), and check 5 (testkit-direction). The module doc (L12) claims "`edges` | every actual dependency is allowed by `boundaries.toml`" — false for this form. Why it matters: the gate is the CI-enforced ARCH-2/ARCH-7 barrier; table form is valid, common Cargo.toml style for multi-key dep specs, so an unreviewed dependency can slip past CI silently. What correct looks like: treat any table whose path CONTAINS a dep-table segment (or first segment is one of the three kinds, incl. `[dependencies.foo]`) as a dependency declaration. Confidence: HIGH — Verification: RAN. Fixture `scratch/bypass1` (table-form dep, `internal_deps = []`): **exit 0, `result: PASS (5 of 5 checks)`**. Control `scratch/control1` (same dep inline, everything else identical): exit 1 `[EDGE] ... not in [crates.alpha].internal_deps`. The gate is fooled by syntax alone. No manifest in the current tree uses table form (rg `^\[dependencies\.` over all Cargo.tomls: 0 hits), so this is a hole, not an active violation.

F-002 [S3]
Where: tools/xtask/src/cmd/check_architecture.rs:438-441
Code:
```rust
        match self.peek() {
            None | Some('\n') => Ok(()),
            Some(c) => Err(self.error(format!("unexpected `{c}` after value"))),
        }
```
What is wrong: `finish_line` accepts only `\n` as a line terminator; a `\r\n`-ended manifest leaves a `\r` after every value/header and errors. CRLF Cargo.toml is exactly what Windows checkouts with `core.autocrlf=true` produce. The gate then exits 2 ("tool cannot verify") on every member manifest — the architecture gate is unrunnable on that platform config, inviting contributors to bypass it. What correct looks like: accept `\r` before `\n` (or normalize line endings on read). Confidence: HIGH — Verification: RAN. Fixture `scratch/crlf` (control1 manifest + CRLF endings): exit 2, `internal error: parsing .../alpha/Cargo.toml (a member manifest): line 1: unexpected `` after value` (the `\r` prints invisibly). No CRLF manifests exist in the tree today (rg check: 0 files).

F-003 [S3]
Where: tools/xtask/src/cmd/check_architecture.rs:1454-1456
Code:
```rust
            let target_layer = b.crates.get(&d.name).and_then(|target| target.layer);
            if let (Some(from_layer), Some(to_layer)) = (rule.layer, target_layer) {
                if from_layer < to_layer {
```
What is wrong: layer ordering is enforced only when BOTH crate rules declare `layer`. Nothing (neither `load_boundaries` nor any check) requires a non-dev crate to have a layer, so omitting `layer = N` from a `[crates.<name>]` section silently exempts that crate from both upward-edge and same-layer rules. Production `boundaries.toml` currently has 5 layerless entries (`first-vertical`, `testkit`, `lumen-mem-audit`, `lumen-sweep`, `template` — all dev/tool crates today), so the hole is unused but unguarded. What correct looks like: reject layerless rules for crates not in `dev_only`, or require Lead sign-off equivalent. Confidence: HIGH — Verification: RAN. Fixture `scratch/layerless` (alpha L1 → beta, beta rule has NO layer, edge allowlisted, same-layer pair NOT whitelisted): exit 0 PASS. Control `scratch/layerctl` (identical but beta `layer = 1`): exit 1 `[LAYER] internal edge ... same-layer (1 -> 1) but the pair is not whitelisted`.

F-004 [S3]
Where: tools/xtask/src/cmd/check_architecture.rs:990-1000
Code:
```rust
    // Duplicate package names would confuse every check that keys by name.
    for pair in members.windows(2) {
        if let [a, b] = pair
            && a.name == b.name
        {
```
What is wrong: `members` is built in path order (`BTreeSet<PathBuf>` in `expand_member_patterns`), not name order, and `windows(2)` only compares ADJACENT members. Two crates with the same package name whose directories do not sort adjacently (e.g. `crates/a` name=`dup`, `crates/b` name=`mid`, `crates/c` name=`dup`) are not detected; the comment's stated protection ("would confuse every check that keys by name") silently fails. Cargo itself rejects duplicate names, so real-tree impact is limited to the checker's hermetic/fixture use. What correct looks like: collect names into a set and compare against it. Confidence: HIGH — Verification: RAN. Fixture `scratch/dupname` (3 crates, two named `dup` in non-adjacent dirs): exit 0, `result: PASS (5 of 5 checks)`, `members: 3 workspace crates, 2 declared` — no duplicate error.

F-005 [S4]
Where: tools/xtask/src/cmd/check_architecture.rs:1002-1003
Code:
```rust
    let root_denies_unsafe = root_doc.get(&["workspace", "lints", "rust"], "unsafe_code")
        == Some(&TValue::Str("deny".to_string()));
```
What is wrong: exact string match on `"deny"`. `unsafe_code = "forbid"` — strictly stronger — fails check 3 with "the root manifest does not deny unsafe_code", as does the valid `{ level = "deny" }` table form (parsed as Opaque). False rejection of an equal-or-stricter posture. What correct looks like: accept `deny` or `forbid` (and the table form). Confidence: HIGH — Verification: RAN. Fixture `scratch/forbid` (root `unsafe_code = "forbid"`): `[UNSAFE] the root manifest does not deny unsafe_code` → check 3 fails.

F-006 [S4]
Where: tools/xtask/src/cmd/check_architecture.rs:646-650
Code:
```rust
                Some(_) => {
                    // Non-string element: rewind and take the array opaque.
                    self.pos = start;
                    return self.parse_opaque_array();
                }
```
What is wrong: `self.pos` is rewound but `self.line` is not, so any newline consumed by `skip_blank` before the rewind is counted again when `parse_opaque_array` re-walks the text. Every multi-line array whose first element sits on a later line inflates subsequent parse-error line numbers by +1. Cosmetic (diagnostics only; failure mode stays loud exit 2). Confidence: HIGH — Verification: RAN. `scratch/ld4` (5-line manifest, bad `\q` escape on line 5, array opens with `[\n 1,`): reports `line 6`; `scratch/ld5` (6 lines, error on line 6): reports `line 7`. Control without the leading newline inside the array reports the correct line.

F-007 [S4]
Where: tools/xtask/src/cmd/check_architecture.rs:703-708
Code:
```rust
            match self.peek() {
                None | Some('\n') => {
                    return Err(
                        self.error("multi-line inline tables are not supported by this reader")
                    );
                }
```
What is wrong: `parse_inline_table` (and `parse_opaque_array`) recognize only `"` basic strings; a `'...'` literal string is consumed as raw characters, so a literal containing `}`/`]` decrements bracket depth early and misparses. `{ s = 'a}b' }` returns `Opaque("{ s = 'a}")` and the trailing text then fails `finish_line`. Valid TOML rejected loudly (safe direction, but a false rejection). Blast radius: dep inline tables with literal-string values are rare. What correct looks like: route `'` through `parse_literal_string` in both opaque scanners. Confidence: HIGH — Verification: RAN. Fixture `scratch/litstr` (`foo = { s = 'a}b' }`): exit 2, `line 2: unexpected `b` after value`.

F-008 [S4]
Where: tools/xtask/src/cmd/check_architecture.rs:1442-1446
Code:
```rust
                    message: format!(
                        "{} [{}]: `{}` depends on `{}`, which is not in \
                         [crates.{}].internal_deps (doc 01 §5.2 step 3, ARCH-2)",
                        m.manifest_rel, d.table, m.name, d.name, m.name
```
What is wrong: `d.table` stores only the LAST header segment ("dependencies"), so a violation from `[target.'cfg(windows)'.dependencies]` is reported as `[dependencies]` — the diagnostic names a table the dep is not in. Misleading when chasing the fix. Confidence: HIGH — Verification: RAN. Fixture `scratch/targetdep` (dep only in `[target.'cfg(windows)'.dependencies]`): output `[EDGE] crates/alpha/Cargo.toml [dependencies]: ...` (note: the edge itself IS correctly caught — this is a labeling defect only).

F-009 [S4]
Where: tools/xtask/src/cmd/check_architecture.rs:28-35
Code:
```rust
//! * **`static mut` scan posture (honest):** a comment/string-aware text scan
//!   of source files. False negatives: `static mut` synthesized by external
//!   proc-macros is invisible (macro-generated code is not in our sources);
//!   a `'"'` char literal can temporarily confuse the string tracker.
//!   False positives: none known — comments, doc comments, string literals
```
What is wrong: "False positives: none known" is no longer true: a `macro_rules!` body containing the literal words `static mut` (a macro that could never be invoked under `deny(unsafe_code)`) is flagged. Also the scan covers `src/` trees only — `build.rs`, `tests/`, `benches/`, `examples/` are unscanned (the `[lints]` inheritance check plus root deny is the actual compile-time barrier; this is belt-and-braces and the doc admits the scope at L15). Confidence: HIGH for the false positive — Verification: RAN. Fixture `scratch/macrofp` (`macro_rules! m { () => { static mut X: u8 = 0; } }` + never invoked): `[STATIC-MUT] crates/alpha/src/lib.rs:1: ...` → check 4 fails. Severity S4 because the flagged text is a genuine `static mut` producer, merely a dead one.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run | 140-201 | REAL | — | arg parse (--help/--json/--root), root discovery+canonicalize, orchestration; exit 0/1 verified by execution |
| TValue::as_str / as_bool / as_int / as_str_array | 220-245 | REAL | — | plain accessors |
| parse_toml | 286-375 | REAL w/ holes | S2/S4 | BOM strip; dup table/key rejection; holes via F-001-adjacent table-shape blindness, F-006 |
| Parser::eof / peek / bump / error | 385-405 | REAL | — | line counting on \n |
| Parser::skip_blank | 408-420 | REAL | — | ws + whole-line comments |
| Parser::skip_inline_ws | 423-427 | REAL | S3 | handles space/tab only — `\r` left for finish_line → F-002 |
| Parser::finish_line | 429-442 | REAL w/ defect | S3 | F-002 CRLF rejection (executed) |
| Parser::parse_header_path | 446-466 | REAL | — | bare + quoted segments; verified by unit test toml_handles_quoted_header_segments_and_arrays_of_tables |
| Parser::parse_key | 469-481 | REAL | — | |
| Parser::parse_bare_key | 484-495 | REAL | — | [A-Za-z0-9_-] |
| Parser::parse_basic_string | 498-536 | REAL | — | 5 escapes; unsupported escapes fail loudly (unit-tested) |
| Parser::parse_literal_string | 539-555 | REAL | — | |
| Parser::parse_value | 558-569 | REAL | — | dispatch by first char |
| Parser::parse_bool | 573-585 | REAL | — | word-boundary check avoids bareword confusion |
| Parser::starts_with_word | 589-601 | REAL | — | |
| Parser::parse_number | 605-620 | REAL | — | ints; floats/dates opaque, never fail |
| Parser::parse_array | 625-653 | REAL w/ defect | S4 | string arrays incl. multi-line + comments; F-006 line drift on rewind |
| Parser::parse_opaque_array | 658-694 | REAL w/ defect | S4 | bracket-depth scan; literal strings blind → F-007 |
| Parser::parse_inline_table | 698-732 | REAL w/ defect | S4 | single-line only, loud reject; F-007 |
| Parser::parse_opaque_token | 736-746 | REAL | — | |
| TomlDoc::entries_at / get | 749-765 | REAL | — | merges entries across repeated (array-of-tables) paths; get takes first |
| TomlDoc::has_table | 768-773 | REAL | — | #[cfg(test)] only |
| table_path_matches | 777-779 | REAL | — | exact-length path compare |
| DepEntry::is_dev | 795-799 | REAL | — | build-dependencies count as normal (non-dev) — reasonable |
| find_workspace_root | 855-869 | REAL | — | walk-up to [workspace]; mirrors gate.rs (documented duplication) |
| manifest_declares_workspace | 874-881 | REAL | — | line-start `[workspace]` match; read errors → false (treated as non-root) |
| load_workspace | 888-1009 | REAL w/ defects | S2/S3/S4 | dep collection filter F-001 (946-972), dup detection F-004 (990-1000), exact-deny F-005 (1002-1003); target-scoped dep tables correctly collected (verified: scratch/targetdep) |
| parse_manifest | 1013-1023 | REAL | — | wraps parse errors with file+line |
| root_bool | 1026-1028 | REAL | — | |
| expand_member_patterns | 1036-1129 | REAL | — | literal paths + single `*` in last component; fancier globs fail loudly (exit 2), never silently miss — but NOT "exactly like cargo" (doc L26-27 overstates) |
| load_boundaries | 1136-1268 | REAL | — | strict: unknown [rules]/[crates] keys and unknown tables are tool errors |
| str_array / str_array_value | 1271-1290 | REAL | — | |
| CheckOutcome::is_pass / status_str | 1320-1327 | REAL | — | |
| run_checks | 1330-1350 | REAL | — | static-mut scan IO errors become a FAIL violation (exit 1), not exit 2 — defensible, undocumented deviation from "parse errors exit 2" |
| check_edges | 1353-1512 | REAL w/ defects | S2/S3/S4 | allowlist + layer logic verified by fixtures; F-001 (input), F-003 (1454-1486), F-008 (label); dev-only/self-edge ownership skips at 1411-1418 verified by dedicated tests |
| check_cycles | 1515-1569 | REAL | — | 3-color DFS over normal+dev edges; dedup by node-set (two distinct cycles on the same node set report once — undercount, still fails) |
| dfs_cycles | 1573-1607 | REAL | — | unit-tested incl. self-loops |
| check_lints | 1610-1659 | REAL w/ defect | S4 | F-005; override tables correctly rejected |
| check_static_mut | 1663-1710 | REAL | S4 | src/-trees only, static_mut_allow exempts whole crates (currently `[]`, boundaries.toml:54); F-009 |
| check_testkit_direction | 1713-1740 | REAL | — | verified by testkit_in_dependencies_is_caught |
| collect_rs_files | 1747-1762 | REAL | — | recursive, sorted; follows symlinked dirs (loop hazard is theoretical; none in tree) |
| strip_strings_and_comments | 1769-1889 | REAL | — | //, nested /* */, "...", r#"..."#; newline-preserving; char literals documented blind spot |
| is_ident_char | 1892-1894 | REAL | — | |
| static_mut_lines | 1899-1937 | REAL | S4 | word boundaries + whitespace (incl. newline) between keywords; F-009 macro FP |
| Report::failed_checks / result / machine_line | 1952-1975 | REAL | — | machine line format unit-tested; matches baseline full_gate.txt:51 |
| Report::print_human | 1977-2015 | REAL | — | output matches my run + baseline |
| Report::to_json | 2018-2079 | REAL | — | hand-rolled JSON; verified by json_report_escapes_and_carries_violations test + artifact test |
| Verdict::as_str / is_pass | 2088-2098 | REAL | — | |
| check_line | 2102-2108 | REAL | — | dot-padding to 24 cols |
| write_json_report | 2111-2118 | REAL | — | creates root/target/ (report path verified by test) |
| json_escape | 2122-2138 | REAL | — | RFC 8259 control-char escapes; non-ASCII passthrough is valid UTF-8 JSON |
| unix_now | 2141-2146 | REAL | — | pre-1970 clock → 0 |
| rel_path | 2150-2155 | REAL | — | forward-slash normalize; absolute fallback |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| clean_scratch_workspace_passes_all_five_checks (+ decoys) | YES | runs real binary on real scratch workspace; asserts exit 0 + exact machine line; decoys prove no FP from comment/string text |
| clean_run_writes_the_json_report_artifact | YES | asserts the on-disk JSON + --json echo |
| forbidden_internal_edge_is_caught | YES | my control1 fixture reproduces it live |
| upward_layer_edge_is_caught / unlisted_same_layer_edge_is_caught | YES | exit 1 + exact message |
| dev_edge_cycle_is_caught | YES | asserts CYCLE + "only 1 of 5 failed" |
| missing_lints_inheritance_is_caught | YES | member + root violations both asserted |
| static_mut_declaration_is_caught | YES | asserts exact file:line |
| testkit_in_dependencies_is_caught | YES | allowlist permits the edge, only check 5 can catch it — and does |
| undeclared_and_stale_manifest_entries_are_caught | YES | both directions |
| unapproved_external_dependency_is_caught | YES | |
| usage_errors_exit_two | YES | exit-code contract |
| 12 unit tests (toml/stripper/cycle/report) | YES | incl. rejection cases; but NO unit or fixture test uses table-form `[dependencies.x]` deps, CRLF endings, layerless crate rules, or duplicate names — which is exactly why F-001..F-004 survived |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every actual dependency is allowed by boundaries.toml" | check_architecture.rs:12 | FALSE (for table-form deps) | F-001 executed: scratch/bypass1 exit 0 PASS with undeclared dep |
| "False positives: none known" (static-mut scan) | check_architecture.rs:32 | FALSE (now one known) | F-009 executed: macro_rules body flagged |
| "Parse errors are tool errors (exit 2), distinct from violations (exit 1)" | check_architecture.rs:42-44 | REAL | usage_errors_exit_two + scratch/crlf exit 2; caveat: static-mut scan IO errors become violations/exit 1 (run_checks 1335-1347) |
| "workspace member set is discovered ... exactly like cargo does" | check_architecture.rs:26-27 | SHALLOW | only literal + single trailing `*` supported; fancier globs error loudly (1062-1071) — hermetic, not cargo-equivalent |
| "Multi-line arrays and other TOML shapes are rejected loudly" | boundaries.toml:24-25 (comment) | STALE/CONTRADICTED | code supports multi-line string arrays since W25 (check_architecture.rs:283-285, unit test at 2264-2278); boundaries.toml's own header comment predates it |
| "(proven locally: arch/schema-gen/codegen/fixtures/rebrand all exit 0)" | .github/workflows/ci.yml:78 | FALSE today | arch exits 1 on the committed tree (my run + baseline full_gate.txt:42) |

## 7. Dead / duplicated / unreachable code
None dead. `TomlDoc::has_table` is test-only (declared). `find_workspace_root`/`manifest_declares_workspace` duplicate gate.rs private helpers — documented at L850-852, accepted. Dependency renames (`foo = { package = "bar" }`) are keyed by local name (L76-77): a renamed INTERNAL dep is misclassified as external (must be in external_allowlist, not internal_deps) — loud, not a hole, but the diagnostic category is wrong.

## 8. Dependencies & flags
xtask has zero external dependencies (tools/xtask/Cargo.toml, "external dependencies (C5 — nothing to approve, nothing to audit)" — verified: no [dependencies] table beyond workspace-inherited none). No feature gates. Not part of the release lane; reached only via cargo alias + CI (ci.yml:236-238, full_gate.sh:28-33). Exit contract 0/1/2 verified.

## 9. Open questions for the next agent
1. The `lumen-cli`→`streaming` edge violation is a LIVE repo defect (baseline + my re-run): was the boundaries.toml update ever Lead-reviewed (worklog/NEXT-STEPS.md)? The fix hint (add `streaming` to `[crates.lumen-cli].internal_deps`, layer 6→2 downward) would make the gate pass — next agent should check whether that lands or the dependency is reverted.
2. ci.yml:78 claims arch exits 0 locally — stale; whoever audits .github/workflows should date-stamp such claims.
3. boundaries.toml header comment (L24-25) contradicts the parser (multi-line arrays now supported) — boundaries.toml is in this domain's file set; its own auditor should carry the contradiction.
4. Should check 2's node-set cycle dedup distinguish distinct cycles on the same node set? Cosmetic undercount today.
5. F-001 fix will need new fixture coverage for `[dependencies.x]` and `[dev-dependencies.testkit]` table forms — the current 12-fixture suite has a syntax blind spot.
