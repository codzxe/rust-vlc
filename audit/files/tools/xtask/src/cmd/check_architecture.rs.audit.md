# AUDIT: tools/xtask/src/cmd/check_architecture.rs

| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-16 |
| Lines / functions | 2548 / 78 `fn` items (5 checks + loaders + a char-level TOML parser + a source scanner + report renderers; 12 in-file unit tests, integration tests in `tools/xtask/tests/check_architecture.rs`) — audited per instruction at structure/orchestration level, not line-by-line |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (`cargo xtask check-architecture` → `cmd/mod.rs:100-107` → `check_architecture::run`; executed in baseline: `audit/runs/baseline/xtask-gates.log` and the artifact `target/architecture-report.json` with `generated_unix` 1795196403) |
| Findings | S0:0 S1:0 S2:1 S3:2 S4:2 |
| Commands run | Read orchestration/structure (~900 of 2548 lines incl. every check function, loaders, scanner, reporter; parser cluster read at header level); read `target/architecture-report.json` (members=39 declared=39, edges FAIL with 1 violation); verified the violation for real: `grep -n 'streaming' apps/cli/Cargo.toml` (line 62: `streaming = { path = "../../crates/streaming" }`) vs `grep -A8 '^\[crates.lumen-cli\]' boundaries.toml` (internal_deps list omits `streaming`); member-set arithmetic: 31 crates + 4 apps (desktop-tauri excluded per ADR-004) + 4 explicit members = 39 = report's members; `grep -c '^\[\[package\]\]' Cargo.lock`; `sed -n` quote verification. All read-only. |

## 1. One-paragraph verdict (blunt: what it claims vs what it does)

check_architecture.rs claims to enforce doc 01 §5 boundary rules — edge allowlist + layer ordering, cycles over normal AND dev edges, lint inheritance, `static mut` scan, testkit direction — against the real workspace, reading every Cargo.toml directly with a hand-rolled std-only parser. It does real analysis and I verified its current FAILURE is genuine: the committed `target/architecture-report.json` records one EDGE violation (`apps/cli` depends on `streaming`, which `boundaries.toml [crates.lumen-cli].internal_deps` does not list), and I independently confirmed both halves of that violation in the source files — the tool is not failing cosmetically, and it is not passing cosmetically either; it caught a stale allowlist. No hard-coded expected tables drive results: the layer stack, allowlists, same-layer pairs, dev-only set, and static-mut exemptions all come from parsing `boundaries.toml`; the member set comes from expanding `[workspace] members` globs in the root manifest (verified arithmetically: 31 crates + 4 non-excluded apps + 4 explicit paths = 39 = report's members = declared). The one structural defect worth its salt: duplicate package-name detection is order-dependent (`windows(2)` on path-sorted members) and misses non-adjacent same-name pairs, which matters because the tool's stated purpose is to verify broken workspaces cargo itself cannot build. A second finding is that the baseline log capturing this tool's run is mangled relative to what the code prints (and records EXIT=0 against the code's exit-1 contract), which is an evidence-integrity problem pinned on the log, not the tool.

## 2. What actually works (verified) — with the command you ran and its result

- **The five checks run real data, no baked results**: `run` (140-201) → `load_workspace` (888-1009, reads root + every member manifest), `load_boundaries` (1136-1268, parses boundaries.toml with unknown-table/unknown-key rejection), `run_checks` (1330-1350) → the five check functions. The only constants are names and report paths (`CHECK_NAMES` 127-133, `REPORT_REL` 124).
- **Verified genuine FAILURE**: `target/architecture-report.json` (read) says `edges: fail` with the violation `apps/cli/Cargo.toml [dependencies]: 'lumen-cli' depends on 'streaming', which is not in [crates.lumen-cli].internal_deps`. I verified `apps/cli/Cargo.toml:62` has `streaming = { path = "../../crates/streaming" }` and `boundaries.toml`'s `[crates.lumen-cli] internal_deps = [...]` (15 entries, no `streaming`). The note line "98 internal edges, 46 dev edges, 39 external deps" comes from counting real manifest entries.
- **Member discovery is real**: `expand_member_patterns` (1036-1129) expands `crates/*`, `apps/*` from the root manifest's single-line members array, subtracts the `exclude` list, and hard-errors on a member dir without Cargo.toml. Arithmetic check: 31 `crates/` dirs + apps/{cli,control-server,desktop,web} (desktop-tauri excluded per the ADR-004 note in Cargo.toml) + tools/xtask + tests/integration/first-vertical + tools/mem-audit + tools/sweep = 39 = `members: 39` in the report = `declared: 39` in boundaries.toml.
- **Dependency extraction covers the shapes that matter**: `load_workspace` (946-972) takes keys from any table whose last segment is `dependencies`/`build-dependencies`/`dev-dependencies` — which includes `[target.'cfg(...)'.dependencies]` (the parser's quoted header segments at 446-466 support it) — keyed by local name for renames (documented 76-77). `[lints] workspace = true` detection (974-985) and root `[workspace.lints.rust] unsafe_code = "deny"` (1002-1003) feed check 3, which the report shows as `39/39 crates inherit; root denies unsafe_code`.
- **Cycle detection includes dev edges and self-loops**: `check_cycles` (1515-1569) builds adjacency over ALL internal edges (normal + dev; self-edge included at 1522-1525 since member_names contains the name), then the three-color DFS (1573-1607) records back-edge cycles deduplicated by node set; in-file tests (2364-2430) exercise dev-edge cycles and self-loops on fixtures. Report: 39 nodes, 144 internal edges, 0 cycles — 144 = 98 normal + 46 dev edges counted in check 1, consistent.
- **`static mut` scanner is a real string/comment-aware scan**: `check_static_mut` (1663-1710) walks every member `src/` tree (`collect_rs_files` 1747-1762), strips comments/strings/raw strings preserving line numbers (`strip_strings_and_comments` 1769-1889), and matches `static`+whitespace+`mut` with word boundaries (`static_mut_lines` 1899-1937). Report: "scanned 632 files across 39 src trees".

## 3. Findings

### F-A01 [S2] — Duplicate-package-name detection is order-dependent and misses non-adjacent duplicates
Where: tools/xtask/src/cmd/check_architecture.rs:990-1000
Code (990-1000):
```rust
    // Duplicate package names would confuse every check that keys by name.
    for pair in members.windows(2) {
        if let [a, b] = pair
            && a.name == b.name
        {
            return Err(XtaskError::internal(format!(
                "duplicate package name `{}` ({} and {})",
                a.name, a.manifest_rel, b.manifest_rel
            )));
        }
    }
```
What is wrong: `members` is built in `member_dirs` order — the path-sorted output of a `BTreeSet<PathBuf>` (1115-1127). `windows(2)` only compares ADJACENT pairs. Two crates with the same package name whose directories are not path-adjacent (e.g. `apps/aaa` and `crates/zzz` both named `dup`, with any third member's path sorting between them) are not detected, despite the comment's own warning that duplicates "would confuse every check that keys by name" — and everything downstream (member_names sets, boundaries lookups keyed by name, cycle adjacency) does key by name.
Why it matters: the module's stated design goal (20-27) is to check "deliberately-broken fixture workspaces … even though cargo could not build them" — precisely the trees where duplicate names appear. A hermetic checker with an order-dependent duplicate detector fails its own mission: a fixture with non-adjacent duplicate names silently produces garbage verdicts (members keyed by name collide in `b.crates.get(&m.name)` lookups and adjacency maps) instead of the loud `XtaskError` this code promises.
What correct looks like: collect `members.iter().map(|m| m.name)` into a `BTreeSet`/`BTreeMap` and detect any duplicate regardless of adjacency — three lines.
Confidence: HIGH (logic gap is unambiguous)
Verification: read.

### F-A02 [S3] — Baseline log's check-architecture entry is mangled relative to what the code prints, and records EXIT=0 against the code's exit-1 contract
Where: tools/xtask/src/cmd/check_architecture.rs:196-200 (exit contract), 1977-1984 (unconditional report header) vs /home/z/my-project/audit-workspace/audit/runs/baseline/xtask-gates.log (whole file)
Code (196-200):
```rust
    if report.result().is_pass() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
```
and (1978-1979):
```rust
        println!("=== LUMEN ARCHITECTURE REPORT ===");
        println!("root:       {}", self.root);
```
What is wrong: `xtask-gates.log`'s check-architecture block contains NO `=== LUMEN ARCHITECTURE REPORT ===` header, no `root:`/`members:`/`spec:` lines, and none of the five `N name ... PASS/FAIL` lines that `print_human` unconditionally emits — it jumps straight to `result: FAILURE …` — and then records `EXIT=0`. The orchestrator's own context states exit 1 for this run, and the code above returns 1 on a failed verdict; a genuine captured run would show the header lines AND exit 1.
Why it matters: the recorded evidence for this tool is not a faithful transcript of the tool's output. Either the baseline capture filtered/truncated lines and mis-recorded the exit code, or the log was assembled rather than captured. For an evidence-only audit, a baseline log that cannot be reproduced from the code it documents poisons every conclusion drawn from it (including mine — I could not use the log to verify print_human's rendering).
What correct looks like: re-capture with `cargo xtask check-architecture 2>&1 | tee …; echo EXIT=$?` and byte-compare against the emitted report.
Confidence: HIGH that the log is inconsistent with the code (the missing lines and EXIT=0 are both flat contradictions); UNVERIFIED which side (capture pipeline vs hand assembly) caused it.
Verification: read (code + log side by side).

### F-A03 [S3] — `static mut` scan never looks at build scripts, examples, or non-`src/` trees
Where: tools/xtask/src/cmd/check_architecture.rs:1663-1674, module header 14-15
Code (1667-1674):
```rust
    for m in &ws.members {
        if b.static_mut_allow.contains(&m.name) {
            continue;
        }
        let src = m.dir.join("src");
        if !src.is_dir() {
            continue;
        }
```
What is wrong: only `<member>/src` is scanned. `build.rs` files (present in this workspace — members carry them), `examples/`, `benches/`, and `apps/desktop/prototype` style side trees are outside the scan. The module header scopes the claim honestly ("no `static mut` in any member `src/` tree", line 15), so the printed PASS is truthful about scope — but gate G5's protection has holes a hostile crate can deliberately exploit (declare `static mut` in build.rs; the generator runs with full build-time power).
Why it matters: the workspace-level `deny(unsafe_code)` lint does apply to build scripts compiled as part of the package, which mitigates use-of-unsafe — but the checker's own belt-and-braces promise ("catches the declaration itself", 33-35) is narrower than the gate's advertised coverage.
What correct looks like: scan `src/` + `build.rs` + `examples/` + `benches/` per member, or state the scope in the report note (it currently says only "scanned N files across N src trees", which is honest).
Confidence: HIGH (scope is explicit in code)
Verification: read.

### F-A04 [S4] — Scanner's documented false negatives: `'"'` char literal, macro-generated `static mut`
Where: tools/xtask/src/cmd/check_architecture.rs:29-35, 1764-1768
Code (1766-1768):
```rust
/// region (a `'` never opens a stripped
/// region (documented false-negative: a `'"'` char literal).
```
What is wrong: a `'"'` char literal flips the string stripper's state so a following `static mut` inside a string/comment region can be missed or misplaced by a line; macro-generated statics are invisible in source. Both are declared in the honest register rather than hidden.
Why it matters: known, bounded, honest — the `deny(unsafe_code)` lint is the primary control; this scan is the secondary. Acceptable, recorded here because an auditor must know the scan is not exhaustive.
Confidence: HIGH
Verification: read.

### F-A05 [S4] — Repo comment contradicts the reader's capability (doc rot touching this file's trust)
Where: tools/xtask/Cargo.toml root manifest comment (workspace members array) vs this file's parser
Code (root Cargo.toml, comment above `members = [...]`):
```toml
# Single-line members array: the xtask TOML reader (check-architecture)
# does not support multi-line arrays — the salvage's multi-line form
# broke the arch gate; restored to the reader's shape with the same
# membership (the 2.E1 pre-announced edit, unchanged in content).
```
What is wrong: the current reader DOES support multi-line arrays — `parse_array` (625-653) skips newlines/comments between elements and `parse_opaque_array` (658+) is the W25 CI-repair addition documented in this file's own header ("Multi-line ARRAYS parse (W25 CI-repair: member manifests ship them)", lines 73-75). The root manifest's comment is stale: it describes the pre-W25 reader and forces the workspace to carry a single-line members array for a limitation that no longer exists.
Why it matters: harmless constraint today, but the comment teaches future editors a false fact about this tool — the exact kind of stale "expected behavior" table a hostile audit flags. The constraint also costs diff-noise on every members edit.
What correct looks like: delete the stale comment (or rewrite it to cite the W25 repair), and optionally move members back to multi-line if style prefers.
Confidence: HIGH
Verification: read (parser code lines 625-657 + header 73-75 + root manifest comment).

## 4. Per-function table (structure level — parser internals grouped)

| Function group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 140-201 | REAL | — | Flags → root → load → checks → report → exit 0/1/2. Executed in baseline. |
| `parse_toml` + `Parser` cluster (eof/peek/bump/error/skip_blank/skip_inline_ws/finish_line/parse_header_path/parse_key/parse_bare_key/parse_basic_string/parse_literal_string/parse_value/parse_bool/starts_with_word/parse_number/parse_array/parse_opaque_array/parse_inline_table/parse_opaque_token) | 286-748 | REAL | — | Char-level std-only reader; duplicate tables/keys rejected loudly (332-334, 362-367); dotted keys rejected (343-347); BOM stripped (288-294); multiline arrays supported (625-653); quoted header segments (446-466). In-file tests 2168-2359 cover the shapes. |
| `TomlDoc::entries_at`/`get`/`has_table`/`table_path_matches` | 749-785 | REAL | — | Lookup helpers. |
| `DepEntry::is_dev` | 795-799 | REAL | — | — |
| `find_workspace_root` / `manifest_declares_workspace` | 855-881 | REAL | — | Duplicated from gate.rs (874-881 ≡ gate.rs 800-807) — frozen-file ownership forces the copy. |
| `load_workspace` | 888-1009 | REAL | S2 | Real member/manifest loading; order-dependent duplicate detection (F-A01). |
| `parse_manifest` / `root_bool` | 1013-1028 | REAL | — | Parse-error context wrapping. |
| `expand_member_patterns` | 1036-1129 | REAL | — | Single-`*`-in-last-component globs; louder failure on fancy patterns; verified against the real 39-member workspace. |
| `load_boundaries` | 1136-1268 | REAL | — | Unknown tables/keys rejected loudly (1194-1258); rules parsed (same_layer_pairs "from->to", dev_only, static_mut_allow). |
| `str_array` / `str_array_value` | 1271-1290 | REAL | — | — |
| `run_checks` | 1330-1350 | REAL | — | Five checks wired in order; static-mut scan errors degrade to a violation-with-fix, not a crash. |
| `check_edges` | 1353-1512 | REAL | — | Allowlist semantics (unknown crate = MANIFEST violation both directions 1361-1373, 1490-1501); layer ordering downward-only 1455-1486; dev edges exempt from layers; delegation to checks 2/5 for self/dev-only edges (1411-1418). Caught the real streaming violation. |
| `check_cycles` / `dfs_cycles` | 1515-1607 | REAL | — | All-edges adjacency + 3-color DFS, dedup by node set; recursion depth = member count (39 today; unbounded recursion is a theoretical risk on huge workspaces). |
| `check_lints` | 1610-1659 | REAL | — | Root deny + per-member `[lints] workspace = true`, overrides flagged. Report: 39/39. |
| `check_static_mut` | 1663-1710 | REAL | S3 | Real scan; src/-only scope (F-A03). |
| `check_testkit_direction` | 1713-1740 | REAL | — | Normal-edge-into-dev_only detection. |
| `collect_rs_files` / `strip_strings_and_comments` / `is_ident_char` / `static_mut_lines` | 1747-1937 | REAL | S4 | Deterministic walk; comment/string/raw-string stripper with line preservation; word-boundary matcher. Documented false negatives (F-A04). |
| `Report::failed_checks` / `result` / `machine_line` / `print_human` / `to_json` | 1953-2079 | REAL | — | Human + machine + JSON artifact (schema architecture-report/1); the committed target/architecture-report.json matches this renderer's shape. |
| `Verdict` / `check_line` / `write_json_report` / `json_escape` / `unix_now` / `rel_path` | 2083-2155 | REAL | — | Support; `json_escape` is the third copy in the crate (gate.rs, sbom.rs). |
| tests (12 in-file) + integration | 2161-2548; tests/check_architecture.rs | HONEST | — | Parser shape/reject tests, stripper/matcher tests, cycle tests on fixtures, machine-line/JSON rendering tests. Would fail if the parser or checks regressed. |

## 5. Test honesty

Yes — the in-file tests assert concrete parses (boundaries shape, manifest shape, duplicate rejection, multiline-array acceptance, quoted segments), concrete scanner behavior (stripper keeps code/removes comments; matcher requires word boundaries), concrete cycle findings (dev-edge cycles and self-loops on fixture graphs), and exact machine-line/JSON output. The integration test file `tools/xtask/tests/check_architecture.rs` (referenced at 59-61 for the `--root` flag's fixture-based acceptance proof) runs the real binary against fixture workspaces — I did not read it in depth (out of the structure-level scope) and did not execute anything; UNVERIFIED whether the fixtures include a duplicate-name case that would expose F-A01.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Implements doc 01 §5 … CI gate G5" with five checks | check_architecture.rs:1-17 | TRUE (executed) | target/architecture-report.json carries the five checks with real per-check stats; baseline log confirms a FAILURE verdict. |
| "Manifests are read directly, not via cargo metadata … deliberately-broken fixture workspaces … are checkable even though cargo could not build them" | 20-27 | TRUE with the F-A01 caveat | No subprocess anywhere in the file (grep for Command::new: zero matches in this module); duplicate-name blind spot undercuts the broken-fixture promise. |
| "Deliberately NOT supported … multi-line ARRAYS parse (W25 CI-repair: member manifests ship them)" | 73-75 | TRUE | `parse_array` 625-653 + test 2264+; root manifest's contrary comment is the stale side (F-A05). |
| Orchestrator context: "check-architecture → FAILURE (edges=fail), exit 1" | context | TRUE (code) | Code returns ExitCode::from(1) (196-200); the baseline log's `EXIT=0` line contradicts both — see F-A02. |
| Baseline xtask-gates.log transcript of this tool | audit/runs/baseline/xtask-gates.log | UNFAITHFUL | Missing unconditional print_human lines; EXIT=0 vs contract (F-A02). |

## 7. Dead / duplicated / unreachable code

- `find_workspace_root` + `manifest_declares_workspace` (855-881) duplicate gate.rs (781-807) — explicitly acknowledged at 851 ("mirrors gate.rs — its helpers are private"); frozen-interface ownership prevents sharing. Third `json_escape` copy likewise (2122-2138).
- `TValue::as_bool`/`as_int`/`as_str_array` all used; `Opaque` values are stored-and-ignored by design (210-217) — that is the loud-subset escape hatch for unconsumed keys, not dead code.
- `Boundaries.same_layer_pairs` is consumed (1469-1473); `dev_only`/`static_mut_allow` consumed (1411, 1668). No dead rules found.

## 8. Dependencies & flags

- std only (`BTreeMap`/`BTreeSet`, `Path`, `ExitCode`, `SystemTime`). No subprocess (hermetic — verified by grep). No workspace crates.
- Flags: `--root <dir>` (fixture acceptance proof), `--json`, `--help`; unknown → usage exit 2 (160-164).
- Side effects: writes `target/architecture-report.json`.

## 9. Open questions for the next agent

- Does `tools/xtask/tests/check_architecture.rs` include a duplicate-package-name fixture with non-path-adjacent directories? If yes, F-A01 is disproven by an executed test — read it before treating this as confirmed.
- Fix F-A01 (three lines, name-set dedup) — it is the only S2 in my file set.
- Re-capture the baseline check-architecture run with full output + exit code (F-A02) — the current log cannot be reproduced from the code and weakens every downstream citation of it.
- The edges FAILURE (streaming edge) is a REAL tree violation: either `boundaries.toml [crates.lumen-cli].internal_deps` needs `streaming`, or `apps/cli` should drop the dependency — a one-line fix owned by whoever owns boundaries.toml, not by this tool.
- Recursion in `dfs_cycles` is fine at 39 nodes; if the workspace grows past a few hundred members, convert to an iterative DFS.
