# AUDIT: tools/xtask/src/cmd/gate.rs

| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-16 |
| Lines / functions | 1387 / 28 (18 free fns + 10 impl methods; 17 unit tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (`cargo xtask gate` → `cmd/mod.rs:73-79` → `gate::run`; the `cargo xtask` alias exists in `.cargo/config.toml`) |
| Findings | S0:0 S1:0 S2:0 S3:3 S4:2 |
| Commands run | Read full file (1387 lines); `ls ui/package.json apps/desktop` (both EXIST → gate steps 6-9 runnable), `ls crates/engine-wasm-binding` (DOES NOT EXIST → step 10 permanently SKIP), python3 read of ui/package.json scripts (lint/typecheck/test scripts present, so steps 6-8 invoke real commands); `ls audit/runs/baseline/` + full read of xtask-gates.log (NO `cargo xtask gate` run recorded anywhere); `sed -n '104,113p;426,447p'` quote verification. All read-only; I did NOT run the gate itself (orchestrator serializes builds). |

## 1. One-paragraph verdict (blunt: what it claims vs what it does)

gate.rs claims to run the doc 10 §2.1 ten-command table verbatim and in order via real subprocesses, capture honest pass/fail/skip results, and emit the doc 10 §2.4 report plus a JSON log — and the code genuinely does this: `DOC10_STEPS` (185-278) is a hard-coded COMMAND table (the spec itself, not results), `execute_step` (452-496) spawns real `cargo`/`pnpm`/`wasm-pack` processes with real exit codes and captured output, and skips are honest with reasons. Nothing here fabricates a pass. The hostile question "what does the gate actually gate?" has a concrete answer: on this tree it gates 9 of 10 rows for real (steps 1-5 cargo, steps 6-8 pnpm — `ui/package.json` and its lint/typecheck/test scripts exist; step 9 tauri — `apps/desktop/` exists) while row 10 is a permanent SKIP because its hard-coded crate path `crates/engine-wasm-binding` does not exist in the repository. The three real defects: that baked-in dead path, the bounded 40-line tails that can lose the only `error:` line for long failures, and the absence of any recorded `cargo xtask gate` execution in the baseline evidence — the flagship gate is proven by unit tests that deliberately never run the real table.

## 2. What actually works (verified) — with the command you ran and its result

- **The command table is the doc's table and execution is real**: `DOC10_STEPS` (185-278) encodes the ten verbatim commands; `execute_step` (452-496) does `Command::new(program).args(args).current_dir(cwd).output()` with real exit-code capture (`out.status.code()`), real duration timing, and real stdout/stderr tails. The displayed command string is `argv.join(" ")` — the exact thing that ran (168-173). No result is pre-decided anywhere in the file.
- **Precondition gating verified against the live tree**: steps 6-8 require `ui/package.json` (EXISTS — `ls` verified; scripts lint/typecheck/test present in its JSON), step 9 requires `apps/desktop/` (EXISTS — contains Cargo.toml/src), step 10 requires `crates/engine-wasm-binding` (DOES NOT EXIST — `ls` errors). So a full `cargo xtask gate` on this tree executes rows 1-9 for real and skips row 10 with an honest reason string.
- **Exit-code contract**: `run` (131-160) returns exit 1 iff `has_failures()` (155-158), exit 2 on xtask errors, 0 otherwise — matching main.rs's contract and the module header (78-82).
- **Report machinery is genuine**: `render_block` (598-647) reproduces the doc 10 §2.4 shape (tested byte-exact against a synthetic report at 1198-1213); `render_json` (686-763) writes the `evidence/1` envelope with per-command argv, cwd, status, exit, duration, reason, tails; `write_report` (767-774) puts it in `target/gate-report.json`.
- **Fail-fast/keep-going/dry-run semantics are real and tested**: unit tests at 1051-1126 assert the exact status sequences for fail-fast, keep-going, and dry-run using cheap injected steps; the real table is deliberately never executed in unit tests (917-918, 928-966).
- **Unit tests pin the doc's verbatim commands**: `doc10_commands_are_verbatim` (985-1006) asserts all ten command strings literally; `doc10_preconditions_and_cwd` (1009-1020) pins preconditions and that row 10 embeds `WASM_BINDING_CRATE`.

## 3. Findings

### F-G01 [S3] — Hard-coded wasm-binding path that does not exist: row 10 of the "10-row gate" can never run on this tree
Where: tools/xtask/src/cmd/gate.rs:107-110, 264-277
Code (107-110):
```rust
/// The wasm binding crate of doc 10 §2.1 row 10. The doc leaves the crate
/// name `[PROPOSAL]` (binding layout per doc 05 §5.2 / doc 09); update this
/// single constant when that card lands.
const WASM_BINDING_CRATE: &str = "crates/engine-wasm-binding";
```
What is wrong: `ls crates/engine-wasm-binding` → "No such file or directory". Step 10 (`wasm-pack build crates/engine-wasm-binding --target web --dev`) is skipped on every run with reason "`crates/engine-wasm-binding` not present". The header sells a 10-command gate (9-23); the tree gets 9.
Why it matters: this is a baked-in path driving a permanent SKIP — the "wasm-pack" leg of gate-CI parity is fiction on this repository, while the report still prints `result: SUCCESS (0 of 10 failed, N skipped)` when the other 9 pass. Honest, but the doc-10 parity claim ("what CI runs" and "what the gate runs" are literally the same command strings, lines 5-7) overstates by one row that structurally cannot exist until a future card lands a crate at that exact path.
What correct looks like: either the binding crate exists (out of this file's control — a wave-7 card), or the help/report says "9 of 10 rows applicable on this tree" instead of relying on the SKIP line.
Confidence: HIGH
Verification: ran (`ls` on the path; python3 read of the step table's `requires` logic cross-checked with the filesystem).

### F-G02 [S3] — `error_hint` and the failure-details section can lose the only error line for long command output
Where: tools/xtask/src/cmd/gate.rs:104-105, 386-405, 867-878
Code (386-399):
```rust
    fn error_hint(&self) -> Option<String> {
        let in_stderr = self
            .stderr_tail
            .iter()
            .find(|l| l.trim_start().starts_with("error"));
```
What is wrong: `error_hint` searches only `stdout_tail`/`stderr_tail`, which `tail_lines` (867-878) caps at the LAST 40 lines (`OUTPUT_TAIL_LINES`, 104-105). Cargo/clippy failures frequently emit their first `error:` hundreds of lines before the end; the tail then contains only summary lines like "error: could not compile …" — or, after a long warning cascade, possibly no `error`-prefixed line at all (cargo's summary errors are there, but clippy/`-D warnings` failures can end with a bare warning summary and exit code). The inline FAIL hint and the entire `--- failure details ---` block then either degrade to the summary or show nothing diagnostic.
Why it matters: this degrades the gate's core promise — the doc 10 §2.4 "first error line" hint. It is honest-bounded (documented at 54-59: "bounded tails (≤40 lines each)"), but a FAIL with no visible reason forces the user to re-run the command outside the gate; the JSON log preserves only the same bounded tail, so the evidence log loses the error too.
What correct looks like: capture the first N error-prefixed lines separately (a head-of-errors ring), or raise the tail bound for failed commands.
Confidence: HIGH (mechanism), MEDIUM (frequency)
Verification: read.

### F-G03 [S3] — No recorded execution of `cargo xtask gate` anywhere in the baseline evidence
Where: tools/xtask/src/cmd/gate.rs (whole file) vs /home/z/my-project/audit-workspace/audit/runs/baseline/
Code (test doc comment, 916-918):
```rust
// Tests (C2: unit tests in owned code, incl. error paths; the REAL gate
// table is never executed here — too slow — only injected cheap steps)
```
What is wrong: the baseline directory contains build/clippy/test/fmt/ui logs (`build-lumen-cli-desktop.log`, `clippy-all-targets.log`, `cargo-test-workspace.log`, `ui-*.log`) — those commands were run DIRECTLY, not through the gate — and `xtask-gates.log` records check-architecture/lint-literals/check-identity/fixtures runs, NOT a gate run. There is zero recorded output of `cargo xtask gate` (any flag combination) in the evidence base. The gate's own unit tests never execute the real table (by design), and the CLI integration tests run only `help`/error paths.
Why it matters: the flagship subcommand — the one CI supposedly depends on — is REAL by code reading but UNPROVEN by executed evidence. Its report rendering, root discovery, JSON envelope, and flag handling have never had their output captured in the baseline. Any latent runtime defect (e.g. the documented Windows `pnpm` spawn question at 93-94) would be invisible.
What correct looks like: one `cargo xtask gate --dry-run` (cheap, no subprocess) and one `--fast` run captured into the baseline logs.
Confidence: HIGH (absence verified by listing and reading the baseline logs)
Verification: read (ls + full read of xtask-gates.log and baseline file list).

### F-G04 [S4] — After a failure, excluded/precondition-skip steps are reported as NOT RUN (fail-fast) rather than their skip reasons
Where: tools/xtask/src/cmd/gate.rs:412-425
Code (413-420):
```rust
    for step in steps {
        if halted {
            results.push(StepResult::shell(
                step,
                Status::NotRun,
                Some("fail-fast".into()),
            ));
            continue;
        }
```
What is wrong: the `halted` check precedes the `step_excluded` and `requires` checks, so a `--fast` run that fails at step 2 labels steps 5/9/10 "NOT RUN (fail-fast)" instead of "SKIP (excluded: --fast)" — cosmetic mislabeling of why a step did not run. The doc (36-37) says "steps after a failure report NOT RUN (fail-fast)", so behavior matches its own doc.
Why it matters: report readers see "not run" counts inflated by steps that were never going to run under the active flags.
What correct looks like: evaluate exclusion/preconditions first, then halted.
Confidence: HIGH
Verification: read (order of the three guards is unambiguous).

### F-G05 [S4] — No per-step timeout (documented) and output captured (nothing streams)
Where: tools/xtask/src/cmd/gate.rs:90-96
Code (94-96):
```rust
//! * No per-step timeout (cargo's own behavior governs).
//! * Output is captured, so nothing streams while a long step runs.
```
What is wrong: a hung command hangs the gate indefinitely; CI must enforce the timeout externally. Honestly declared in the "Known v0 limitations" register.
Why it matters: bounded only by external infrastructure; local users get a silent stall with zero output.
What correct looks like: spawn with a configurable per-step deadline; stream to the report file.
Confidence: HIGH (limitation is self-declared)
Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run` | 131-160 | REAL | — | Flags→root→env→steps→report; exit contract honored. |
| `step_excluded` | 283-291 | REAL | — | `--no-ui` 6..=9 first, `--fast` 5/9/10; tested. |
| `parse_flags` | 308-325 | REAL | — | Five flags, else usage error. |
| `Status::json` | 342-351 | REAL | — | Stable lowercase strings. |
| `StepResult::shell` | 369-382 | REAL | — | Non-executed row constructor. |
| `StepResult::error_hint` | 386-405 | REAL | S3 | Tail-bounded hint (F-G02); tested with synthetic tails only. |
| `run_steps` | 410-448 | REAL | S4 | Guard order mislabels skips after halt (F-G04); semantics otherwise tested. |
| `execute_step` | 452-496 | REAL | — | Real spawn/capture/timing; spawn failure → FAIL with reason (490-494). |
| `gather_env` | 516-539 | REAL | — | git/rustc degrade to unknown on failure. |
| `capture` | 542-552 | REAL | — | Read-only helper. |
| `GateReport::count`/`has_failures` | 563-569 | REAL | — | — |
| `GateReport::result_line` | 573-595 | REAL | — | DRY/SUCCESS/FAILURE shapes; tested. |
| `GateReport::render_block` | 598-647 | REAL | — | Doc §2.4 shape; byte-exact test. |
| `GateReport::render_details` | 651-683 | REAL | S3 | Bounded tails only (F-G02). |
| `GateReport::render_json` | 686-763 | REAL | — | `evidence/1` envelope; escapes tested. |
| `write_report` | 767-774 | REAL | — | target/gate-report.json. |
| `find_workspace_root` | 781-795 | REAL | — | Walk-up `[workspace]` match. |
| `manifest_declares_workspace` | 800-807 | REAL | — | Line-start match; adequate. |
| `civil_from_days`/`unix_seconds`/`iso8601_utc` | 815-848 | REAL | — | Hinnant algorithm; tested incl. pre-epoch. |
| `format_duration` | 852-863 | REAL | — | 12s/3m41s/5m02s; tested. |
| `tail_lines` | 867-878 | REAL | S3 | 40-line bound (F-G02). |
| `json_escape`/`jstr`/`jarray`/`round3` | 882-914 | REAL | — | RFC 8259; tested. |
| tests (17) | 921-1387 | HONEST | — | Verbatim-table assertions + injected-step engine tests + rendering byte-tests. Would fail if the table, semantics, or renderers regressed. |

## 5. Test honesty

High. `doc10_commands_are_verbatim` (985-1006) pins the exact ten command strings — any table edit shows up as a failure. The engine tests (1051-1126) run REAL subprocesses (cheap `cargo --version` / `cargo --unknown-flag-a3` / nonexistent binary / missing precondition dir) and assert exact status vectors, exit-code presence, spawn-failure reasons — if `run_steps` mis-sequenced fail-fast or dropped results, they fail. Rendering tests assert byte-exact block output and JSON keys. The honest gap (acknowledged at 916-918): the real 10-command table is never executed by any test, and no baseline log records a gate run (F-G03) — so `execute_step`'s interaction with the real table (pnpm/wasm-pack/tauri spawn behavior, long-output capture) is exercised only in whatever CI runs, which I could not verify from my file scope.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Runs the doc 10 §2.1 command table verbatim and in order" | gate.rs:2-7 | TRUE (code) | `DOC10_STEPS` + verbatim-string unit test 985-1006; execution is real subprocess spawn. |
| "what CI runs and what the gate runs are literally the same command strings" | gate.rs:5-7 | OVERSTATED for row 10 | Row 10's crate path does not exist → permanent SKIP on this tree (F-G01); whether CI runs a wasm-pack step at all UNVERIFIED (ci.yml outside scope). |
| "Steps whose target does not exist yet are reported SKIP (<path> not present)" | gate.rs:24-26 | TRUE | `run_steps` 426-435; step 10 will demonstrate it on this tree. |
| "pnpm on Windows is a `.cmd` shim; Command::new("pnpm") may not spawn it [VERIFY]" | gate.rs:93-94 | HONEST-UNRESOLVED | Declared unresolved by the author; UNVERIFIED (Linux-only environment). |
| Exit codes 0/1/2 as documented | gate.rs:78-82, main.rs:16-23 | TRUE | `run` 155-159 returns 1 iff failures. |
| No `cargo xtask gate` execution evidence | baseline dir | EVIDENCE GAP (F-G03) | Baseline logs list direct command runs only; `xtask-gates.log` contains other subcommands, no gate. |

## 7. Dead / duplicated / unreachable code

- No dead functions. `Json`-style helpers absent here — `jstr`/`jarray`/`json_escape` (880-909) duplicate sbom.rs's `json_escape` (538-556) and check_architecture.rs's `json_escape` (2122-2138) nearly verbatim — a third copy of RFC 8259 escaping in the crate. Consolidation is blocked by the frozen-file ownership rule, so the duplication is systemic, not accidental.
- `StepResult::shell`'s `status` parameter + the struct-update pattern at 480-493 is fine.

## 8. Dependencies & flags

- std only (`Command`, `Path`, `Instant`, `SystemTime`). No workspace crates.
- Flags: `--fast` (skip 5/9/10), `--no-ui` (skip 6-9), `--json`, `--keep-going`, `--dry-run`; unknown → usage exit 2 (317-321).
- Side effects: writes `target/gate-report.json` (creates `target/`); runs git/rustc read-only probes in `gather_env`.

## 9. Open questions for the next agent

- Does CI actually run `cargo xtask gate` (or the equivalent table directly)? If the answer is "CI runs the commands itself," then gate-CI parity (§10.4) rests on string equality, not on shared execution — confirm from `.github/workflows/ci.yml`.
- Capture a `--dry-run` and a `--fast` gate run into the baseline to close F-G03 — cheap and immediate.
- The pnpm-Windows `[VERIFY]` item (93-94) is still open after the UI landed (ui/package.json exists now) — needs a Windows lane or an explicit resolution note.
