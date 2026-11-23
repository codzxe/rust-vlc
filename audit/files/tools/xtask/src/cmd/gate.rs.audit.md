# AUDIT: tools/xtask/src/cmd/gate.rs
| Field | Value |
|---|---|
| Auditor | W01-A06 |
| Date | 2026-11-23 |
| Lines / functions | 1387 / 26 non-test (+23 tests, +2 test helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (dev tool; xtask is not in the release lane `cargo build --release -p lumen-cli -p desktop`). Reachable from the dev entry point `cargo xtask gate`: .cargo/config.toml `[alias] xtask` -> tools/xtask/src/main.rs -> tools/xtask/src/cmd/mod.rs:74-78 (name "gate", run: gate::run) -> gate::run (gate.rs:131). |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 3 |
| Commands run | `cargo test -p xtask cmd::gate` -> 23 passed, 0 failed (exit 0); `sed -n` quote verification (all findings); `ls apps/desktop apps/desktop-tauri`; `rg -c "xtask gate" .github/workflows/*.yml scripts/*.sh` -> no matches; `rg -n "pnpm\|wasm-pack\|cargo tauri" .github/workflows/*.yml`; `command -v pnpm wasm-pack` -> not found; read target/gate-report.json |

## 1. One-paragraph verdict
The gate engine is real, well-tested, and honestly reported: fail-fast/keep-going/skip/dry-run semantics, bounded tails, hand-rolled JSON, and the std-only date/duration helpers all work and I executed all 23 embedded unit tests (pass). But the tool's payload — the hard-coded doc 10 §2.1 command table — has rotted against the repo it gates: row 9 runs `cargo tauri build` in `apps/desktop`, which is a std-only non-tauri skeleton (the real tauri crate is the workspace-excluded `apps/desktop-tauri`), so a real gate run can never pass step 9; rows 6–8 invoke `pnpm` against an npm-managed `ui/` (only `package-lock.json` exists, no pnpm lockfile, and CI runs `npm run ...`); row 10 points at `crates/engine-wasm-binding` which does not exist (the wasm crate is `apps/web`/`lumen-web`), making that check a permanent SKIP. The module header's core selling point — gate-CI command parity — is false: no workflow or script ever invokes `cargo xtask gate`, and CI/full_gate.sh enforce 7+ checks (check-architecture, fixtures, lint-literals, check-identity, sbom, web-budget, packaging) the gate does not run. The only surviving execution artifact (target/gate-report.json) is a `--dry-run`; a real run in this sandbox would FAIL at rows 6–8 (pnpm not installed) at the latest.

## 2. What actually works (verified)
— verified by `cargo test -p xtask cmd::gate` -> `test result: ok. 23 passed; 0 failed` (all 17+ behavioral tests incl. real subprocess pass/fail/spawn-fail/skip semantics, exact report-block shape, JSON schema/escaping, pre-epoch ISO-8601, leap-year dates).
— verified by reading target/gate-report.json (a real `--dry-run` artifact, run_id local-1795456861, commit 92e3da7, 2026-11-23T18:01:01Z): render_json's exact shape matches the code, step 10 honestly SKIPs with reason "crates/engine-wasm-binding not present".

## 3. Findings
F-001 [S2]
Where: tools/xtask/src/cmd/gate.rs:257-263
Code:
```
    Step {
        idx: 9,
        name: "tauri",
        argv: &["cargo", "tauri", "build", "--debug", "--no-bundle"],
        cwd_rel: "apps/desktop",
        requires: Some("apps/desktop/"),
    },
```
What is wrong: `apps/desktop` is NOT a tauri project — its Cargo.toml says "NO tauri/wry import" and the directory contains no `tauri.conf.json` (verified: `ls apps/desktop/` -> Cargo.toml, README.md, examples, prototype, src, tests). The tauri shell is `apps/desktop-tauri` (has `tauri.conf.json`, workspace-`exclude`d per root Cargo.toml:51). Because the precondition `apps/desktop/` IS satisfied, a real gate run executes `cargo tauri build` inside a non-tauri crate: guaranteed FAIL (`cargo-tauri` is not even installed here; and per baseline cargo-build-desktop-tauri.txt the tauri build fails at gdk-sys in this sandbox regardless). Why it matters: the gate can never exit 0 on this tree; every real run reports FAILURE for an environmental/staleness reason unrelated to tree quality — the definition of a noisy, untrustworthy gate. What correct looks like: point row 9 at `apps/desktop-tauri` (its own lane, per tauri-launch.yml) or SKIP when the crate is workspace-excluded.
Confidence: HIGH — Verification: read (directory listings, Cargo.tomls) + baseline citation.

F-002 [S2]
Where: tools/xtask/src/cmd/gate.rs:3-7
Code:
```
//! The xtask shells out to cargo/pnpm/wasm-pack instead of reimplementing
//! checks, so "what CI runs" and "what the gate runs" are literally the same
//! command strings (doc 10 §2.2/§10.4 gate-CI parity).
```
What is wrong: the parity claim is false in both directions. (a) Nothing ever runs the gate: `rg -c "xtask gate" .github/workflows/*.yml scripts/*.sh` -> no matches; the orchestrator baseline contains no gate run (only target/gate-report.json, a dry-run). (b) CI runs checks the gate omits — scripts/full_gate.sh:29-82 runs check-architecture, fixtures --check, fuzz-registry, lint-literals, check-identity, sbom --check, web-budget --check, packaging gate; none of these are gate rows. (c) The UI rows differ: .github/workflows/ui.yml:100-128 runs `npm run typecheck` / `npm run lint` / `npm run test`, not `pnpm --dir ui ...`. Why it matters: a developer trusting "gate PASS == CI PASS" (doc 10 §2.4: "CI runs the identical command set (§10.4)", docs/10-testing-ci-quality.md:171) gets a false security guarantee; §10.4 is actually the runner-minutes budget section, so the cross-reference is stale too. What correct looks like: either wire CI to the gate or delete/annotate the parity claim; add the full_gate.sh checks as gate rows or document the divergence.
Confidence: HIGH — Verification: read + rg (no matches) + baseline full_gate.txt sections (fmt, test, arch, lint-literals, identity, sbom, web-budget red).

F-003 [S3]
Where: tools/xtask/src/cmd/gate.rs:107-110
Code:
```
/// name `[PROPOSAL]` (binding layout per doc 05 §5.2 / doc 09); update this
/// single constant when that card lands.
const WASM_BINDING_CRATE: &str = "crates/engine-wasm-binding";
```
What is wrong: `crates/engine-wasm-binding` does not exist (verified `ls crates/`); the wasm lane landed as `apps/web` (package `lumen-web`, cdylib) and is built with `cargo build -p lumen-web --target wasm32-unknown-unknown --locked` (baseline rc-smoke.txt:134), not with `wasm-pack` (not installed in this environment). The card the comment waits for has landed under a different name and the constant was never updated, so row 10 is a permanent, load-bearing SKIP: the "wasm32 still compiles" check the doc promises (doc 10 §2.1 row 10) is dead weight in the gate — that coverage actually lives only in rc-smoke.sh. Why it matters: a SKIP row looks honest but hides that the check can never fire. What correct looks like: WASM_BINDING_CRATE = "apps/web" plus the real build command, or an explicit "lane moved to rc-smoke/ci" note.
Confidence: HIGH — Verification: read (ls, rg on rc-smoke.txt) + dry-run artifact skip reason.

F-004 [S3]
Where: tools/xtask/src/cmd/gate.rs:236-256 (rows 6-8)
Code:
```
        argv: &["pnpm", "--dir", "ui", "lint"],
        cwd_rel: "",
        requires: Some("ui/package.json"),
```
What is wrong: `ui/` is an npm project — it commits `package-lock.json` (verified: `ls ui/` -> package-lock.json, package.json; no pnpm lockfile) and CI pins npm caches to it (ui.yml). The gate's pnpm rows (a doc-era `[PROPOSAL: pnpm]` choice) were never reconciled with the npm toolchain: on hosts without pnpm they FAIL as spawn errors (pnpm not installed here — `command -v pnpm` -> nothing); on hosts with pnpm they run a different, lockfile-less resolution than CI's `npm ci`. Why it matters: rows 6-8 either always fail or test a dependency graph that is not the one shipped — either way the "UI gate" is not the UI gate CI runs. What correct looks like: `npm --prefix ui run lint` etc. (matching ui.yml), or commit a pnpm lockfile and switch CI.
Confidence: HIGH — Verification: read + command -v.

F-005 [S4]
Where: tools/xtask/src/cmd/gate.rs:800-807
Code:
```
fn manifest_declares_workspace(path: &Path) -> bool {
    match std::fs::read_to_string(path) {
        Ok(contents) => contents
            .lines()
            .any(|line| line.trim_start().starts_with("[workspace]")),
```
What is wrong: cargo's standard member-exclusion idiom is an EMPTY `[workspace]` table in the member manifest; this predicate would treat such a member as the workspace root and gate the wrong tree. Latent only: no crate in this repo uses the idiom (apps/desktop-tauri is excluded via the root's `exclude = ["apps/desktop-tauri"]`, root Cargo.toml:51, and its own manifest has no `[workspace]` line — verified by rg). What correct looks like: require a non-empty `[workspace]` section (or check for `members`/`exclude` keys).
Confidence: HIGH — Verification: read + rg on all apps/*/Cargo.toml.

F-006 [S4]
Where: tools/xtask/src/cmd/gate.rs:711-714
Code:
```
        s.push_str(&format!(
            "  \"flags\": {{\"fast\": {}, \"no_ui\": {}, \"keep_going\": {}, \"dry_run\": {}}},\n",
            self.flags.fast, self.flags.no_ui, self.flags.keep_going, self.flags.dry_run
```
What is wrong: the JSON envelope's `flags` object omits the `--json` flag that the run accepted, so the recorded run is not fully reconstructible from its own log. Trivial, but this file's whole ethos is honest evidence envelopes.
Confidence: HIGH — Verification: read.
F-007 [S4]
Where: tools/xtask/src/cmd/gate.rs:386-397
Code:
```
    fn error_hint(&self) -> Option<String> {
        let in_stderr = self
            .stderr_tail
            .iter()
            .find(|l| l.trim_start().starts_with("error"));
```
What is wrong: prefix match `starts_with("error")` also matches non-error lines ("errors were encountered...", "errornote") and cannot see an error older than the last 40 tail lines. Bounded, documented heuristic — acceptable, noted for the record.
Confidence: HIGH — Verification: read (behavior is test-verified for the simple cases).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run | 131-160 | REAL | — | entry; help fast-path; honest exit 0/1; writes report before printing |
| Step (struct) | 169-181 | data | — | argv/cwd/precondition model |
| DOC10_STEPS (const) | 185-278 | REAL (content stale) | S2/S3 | F-001 (row 9), F-003 (row 10), F-004 (rows 6-8) |
| step_excluded | 283-291 | REAL | — | --fast excludes 5,9,10; --no-ui excludes 6-9; test-verified |
| GateFlags / parse_flags | 299-325 | REAL | — | unknown flag -> Usage error |
| Status::json | 341-351 | REAL | — | 5-state mapping |
| StepResult::shell | 368-382 | REAL | — | skeleton constructor |
| StepResult::error_hint | 386-404 | REAL | S4 | F-007 |
| run_steps | 410-496 | REAL | — | fail-fast / skip / dry logic; verified by executing tests with real subprocesses |
| execute_step | 452-496 | REAL | — | spawn failure => FAIL + reason; exit code + bounded tails captured |
| gather_env | 516-539 | REAL | — | git/rustc degrade to null on failure (documented) |
| capture | 542-552 | REAL | — | read-only helper |
| GateReport::count / has_failures | 563-569 | REAL | — | |
| result_line | 573-595 | REAL | — | partial runs reported as partial (dry-run, skipped, not-run counts) |
| render_block | 598-647 | REAL | — | exact doc 10 §2.4 shape; test pins it byte-for-byte |
| render_details | 651-683 | REAL | — | failure tails section; test-verified |
| render_json | 686-763 | REAL | S4 | F-006; output matches real artifact target/gate-report.json |
| write_report | 767-774 | REAL | — | creates target/; artifact exists on disk |
| find_workspace_root | 781-795 | REAL | — | walk-up; test-verified |
| manifest_declares_workspace | 800-807 | REAL | S4 | F-005 latent idiom bug |
| civil_from_days | 815-826 | REAL | — | Hinnant algorithm; leap-year + pre-epoch tests pass |
| unix_seconds | 829-834 | REAL | — | negative epoch handled |
| iso8601_utc | 837-848 | REAL | — | verified against 5 known instants |
| format_duration | 852-863 | REAL | — | 12s/3m41s/5m02s; test-verified |
| tail_lines | 867-878 | REAL | — | bound + blank trimming; test-verified |
| json_escape / jstr / jarray | 882-909 | REAL | — | RFC 8259 subset; test-verified |
| round3 | 912-914 | REAL | — | millisecond rounding; test-verified |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| doc10_table_has_ten_ordered_steps | YES (weakly) | pins ordering/numbering; same-file constants |
| doc10_commands_are_verbatim | YES (pin only) | re-encodes the doc strings in the test — catches accidental table edits, but cannot detect drift between the table and the actual doc 10 file or the actual repo (which is exactly what happened: rows 6-10 rotted while this test stayed green) |
| doc10_preconditions_and_cwd | YES | pins requires/cwd wiring incl. row-9/row-10 constants |
| flag_exclusions_match_doc10 | YES | real logic coverage of step_excluded |
| fail_fast_marks_remaining_steps_not_run | YES | executes real subprocesses (cargo --version / bad flag) |
| keep_going_collects_every_failure | YES | real exit codes, spawn failure reason, precondition reason |
| dry_run_executes_nothing_but_reports_preconditions | YES | no exits/durations recorded |
| pass_captures_exit_code_and_bounded_output | YES | real capture of cargo --version stdout |
| block_matches_doc10_shape | YES | byte-exact render pin |
| success_result_line_omits_zero_counts / dry_run_result_line | YES | format pins |
| details_section_* | YES | render coverage |
| json_report_has_stable_schema_and_escapes_strings | YES | escaping + schema pins |
| json_result_dry_run | YES | |
| finds_repo_root_from_crate_dir / no_workspace_root_above_temp_dir | YES | real FS walk (second one mildly env-dependent on /tmp cleanliness) |
| iso8601_known_instants / duration_formatting / tail_lines / json_escape / round3 / hint_is_first_error_line_bounded | YES | genuine unit coverage |
Overall: honest suite; no tautologies. Gap: `run()` itself and the real DOC10_STEPS are never executed by any test or by CI (documented as too slow — legitimate), which is precisely how the stale table went undetected.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Runs the doc 10 §2.1 command table verbatim and in order" | gate.rs:3-4 | REAL | table matches docs/10-testing-ci-quality.md §2.1 rows 1-10 string-for-string (compared manually; doc 10:69-84) |
| "'what CI runs' and 'what the gate runs' are literally the same command strings" | gate.rs:5-7; doc 10:114-115, 171 | FALSE | F-002: no CI/script invokes the gate; full_gate.sh runs 8 checks the gate lacks; ui.yml uses npm not pnpm |
| "One entry point: cargo xtask gate" | doc 10 §2.2 (line ~101) | FALSE (in practice) | scripts/full_gate.sh is the de-facto gate and never calls `cargo xtask gate`; ci.yml expands `cargo run -p xtask --locked --` per-command |
| "Steps whose target does not exist yet are reported SKIP ... and do not fail the gate" | gate.rs:24-27 | REAL | run_steps:426-435 + dry-run artifact row 10 SKIP |
| "Spawn failures are FAILs with the reason recorded" | gate.rs:27, 450-451 | REAL | execute_step:490-494 + keep_going test (reason "failed to spawn") |
| JSON log always written to target/gate-report.json | gate.rs:61-65 | REAL | artifact exists (dry-run run local-1795456861) |

## 7. Dead / duplicated / unreachable code
- Row 10 (`wasm-pack`) is unreachable-by-construction: precondition path never exists -> permanent SKIP (F-003).
- Row 9 is live but misdirected (F-001) — it can only ever contribute a FAIL.
- No dead functions found; all helpers are used and tested.

## 8. Dependencies & flags
- xtask is std-only (tools/xtask/Cargo.toml: zero dependencies — verified), so gate.rs has no feature-flag or dependency surface. No unsafe, no unwrap on parsed data; the only `unwrap_or` uses are saturating defaults in helpers.
- The gate itself invokes external tools: cargo (rows 1-5, 9), pnpm (6-8, not installed here), wasm-pack (10, not installed here), git/rustc (env capture, degrade to null).

## 9. Open questions for the next agent
- Who owns doc 10 §2.2's "one entry point" promise now that scripts/full_gate.sh is the real gate? The gate.rs ownership header (lines 84-88) says the dispatch table is frozen — does that freeze also block fixing the stale rows 6-10?
- Is there any host where `cargo xtask gate` (non-dry) has ever been run to completion? Only a dry-run artifact survives; the orchestrator baseline has none.
- full_gate.sh's `set -uo pipefail` (line 4) lacks `-e` (deliberate: manual per-section if/fail accumulation) — its audit belongs to another agent's file list, but note sections 6/7 there (fuzz-registry, packaging) also have no gate.rs counterpart.
