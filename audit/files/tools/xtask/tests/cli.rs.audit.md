# AUDIT: tools/xtask/tests/cli.rs
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-23 |
| Lines / functions | 176 / 3 helpers (`run_xtask`, `repo_root`, `RunOutcome`) + 7 tests + `#[path]`-included `rebrand_skeleton` module (8 more tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — integration tests of the dev tool; run by `cargo test -p xtask` (baseline cargo-test-workspace.txt:6778-6797: 12 passed / 3 ignored). Not in shipped binaries. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/cli-a5726a36b7f29fde` (bare) -> 11 passed / 1 FAILED (`codegen_bridge`: env — no `cargo` on my shell PATH) / 3 ignored; same binary with `PATH="$HOME/.cargo/bin:$PATH"` -> **12 passed / 0 failed / 3 ignored, exit 0** (matches baseline); `./target/debug/xtask help` -> lists all 17 subcommands; `ls ui/package.json` -> exists |

## 1. One-paragraph verdict
These are real, honest binary-surface tests — every one asserts exit codes and output content, and I ran the suite live to the same result as the orchestrator baseline (12/12 pass with cargo on PATH; my one initial failure was my sandbox lacking `cargo` on PATH, not a repo defect). But the suite is frozen in the card-1.A3 era: the dispatch table has 17 subcommands and these tests exercise the interfaces of 6 of them (plus a codegen subprocess probe). None of the graduated gates that CI actually branches on — `sbom --check`, `feature-audit`, `config-validator`, `parity-report`, `lint-literals`, `check-identity`, `web-budget`, `compat-matrix`, `fixtures`, `check-panics`, `visual` — has a single binary-surface test, which is exactly why the exit-code-contract gaps I found in sbom.rs and parity_report.rs are invisible to the test suite. The file's own doc ("the frozen interface other cards rely on") describes an interface that has since tripled without this file noticing.

## 2. What actually works (verified)
- Full suite live: `PATH="$HOME/.cargo/bin:$PATH" ./target/debug/deps/cli-a5726a36b7f29fde` -> `test result: ok. 12 passed; 0 failed; 3 ignored` — identical to baseline cargo-test-workspace.txt:6795-6797.
- The 3 ignored tests are the `rebrand_skeleton` boot-tier placeholders, each with a written reason pointing at the real proof location (test output: "the unit-tier proof lives in tests/rebrand/rebrand_acceptance.rs ... doc 07 §8.3 step 4") — honest ignores, not silences.
- `./target/debug/xtask help` prints all 17 registered subcommands with card references — the frozen-table claim is checkable and the help surface works.

## 3. Findings
F-001 [S3]
Where: tools/xtask/tests/cli.rs:43-50 (vs tools/xtask/src/cmd/mod.rs:74-187 registering 17 subcommands)
Code:
```
    for name in [
        "gate",
        "schema-gen",
        "tokens",
        "codegen",
        "check-architecture",
        "rebrand",
    ] {
```
What is wrong: The suite covers the binary interface of 6 of 17 subcommands. The 11 uncovered ones include every gate CI and full_gate.sh branch on: `sbom --check` (ci.yml:528), `feature-audit`, `config-validator`, `parity-report`, `lint-literals`, `check-identity`, `web-budget`, `compat-matrix`. Their exit-code contracts (e.g. sbom's documented 0/1/2, parity's 0/1/2) are asserted nowhere in the tree — confirmed by rg (no sbom/parity cases in this file). Why it matters: a regression in any graduated subcommand's CLI contract passes the entire test suite; the "frozen interface" the header promises to protect stopped being frozen in the 1.A3 era. What correct looks like: one cheap `--help`-level exit-code probe per subcommand (the pattern this file already uses for codegen), plus drift-path probes for the two `--check` gates.
Confidence: HIGH — Verification: ran + rg + mod.rs read (17 `name:` entries).
F-002 [S4]
Where: tools/xtask/tests/cli.rs:39-58
Code:
```
fn help_exits_zero_and_lists_the_frozen_subcommand_table() {
```
What is wrong: The test name promises "the frozen subcommand table"; the body checks 6 names of 17 plus the string "card 1.A3". Deleting `sbom` or `parity-report` from the dispatch table would not fail it. Why it matters: names that lie (checklist 6.5). What correct looks like: derive the expected list from the same source mod.rs registers, or assert all 17 literals.
Confidence: HIGH — Verification: ran help (lists 17) + read.
F-003 [S4]
Where: tools/xtask/tests/cli.rs:155-159
Code:
```
    // The JSON log is always written.
    assert!(
        repo_root().join("target/gate-report.json").is_file(),
        "gate-report.json was not written"
    );
```
What is wrong: The assertion checks a file EXISTS in target/, not that this invocation wrote it — a stale gate-report.json from any earlier run satisfies it, and a concurrent gate run races it. Why it matters: assertion proves less than its message claims. What correct looks like: stat mtime before/after, or delete the file first.
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: tools/xtask/tests/cli.rs:145-154
Code:
```
    if !repo_root().join("ui/package.json").is_file() {
        assert!(
            out.stdout.contains("SKIP (ui/package.json not present)"),
```
What is wrong: In this tree ui/package.json exists (verified), so the branch never executes — the claimed protection ("config stubs from card 1.B3 must not flip the gate") is dead coverage here. Defensive conditional, but the comment implies protection that is not currently tested. What correct looks like: an unconditional assertion of the ACTIVE ui path (the gate should be running ui steps), or drop the branch.
Confidence: HIGH — Verification: ran (`ls ui/package.json` -> exists) + read.
F-005 [S4]
Where: tools/xtask/tests/cli.rs:3-4 vs 90-97
Code:
```
//! These run the compiled xtask binary with cheap invocations only. They
//! NEVER execute the real gate table (too slow — doc 10 §2; the full gate is
```
What is wrong: `codegen_bridge_dispatches_to_catalog_gen` shells out to `cargo run -p commands --bin catalog-gen` (codegen.rs:67-78), which compiles on a cold cache — my cold run took 12.18s for the suite vs the baseline's warm 0.18s. "Cheap invocations only" is cache-dependent, not structural. Not a correctness issue; the doc overstates the suite's cheapness guarantee on fresh clones (CI pays the compile). What correct looks like: state the warm-cache assumption, or probe the bridge via a pre-built binary path.
Confidence: HIGH — Verification: ran (cold 12.18s vs baseline 0.18s) + read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `run_xtask` | 17-32 | REAL | — | Spawns CARGO_BIN_EXE_xtask, captures code/stdout/stderr; spawn-failure surfaced, not swallowed |
| `repo_root` | 35-37 | REAL | — | CARGO_MANIFEST_DIR/../.. |
| `RunOutcome` | 11-15 | REAL | — | Plain struct |
| `help_exits_zero...` | 39-58 | REAL | — | Passes live; under-asserts (F-002) |
| `no_args_is_a_usage_error` | 60-65 | REAL | — | exit 2 + Usage |
| `unknown_subcommand...` | 68-78 | REAL | — | exit 2 + exact message |
| `codegen_bridge...` | 80-98 | REAL | — | Passes with cargo on PATH (F-005 cold-cost) |
| `gate_bad_flag...` | 107-116 | REAL | — | exit 2 + message |
| `gate_dry_run...` | 118-160 | REAL | — | Passes; dead branch (F-004), weak file assert (F-003) |
| `gate_dry_run_json...` | 162-176 | REAL | — | Envelope fields asserted |
| `mod rebrand_skeleton` (include) | 104-105 | REAL | — | Compiles tests/rebrand/rebrand_skeleton.rs into this binary; 8 tests, 3 honestly-ignored with reasons |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| help_exits_zero_and_lists_the_frozen_subcommand_table | PARTIAL | Fails on exit code or missing one of 6 names; blind to the other 11 (F-001/F-002) |
| no_args_is_a_usage_error | YES | Exit code + stderr content |
| unknown_subcommand_is_a_usage_error_listing_known_ones | YES | Exit code + exact message + known-name presence |
| codegen_bridge_dispatches_to_catalog_gen | YES | Exit 0 + catalog-gen + --check in output; breaks if bridge or binary regresses (verified: failed when spawn target unavailable) |
| gate_bad_flag_is_a_usage_error | YES | Exit code + message |
| gate_dry_run_executes_nothing_and_reports_the_full_table | PARTIAL | Table shape + DRY RUN marker asserted; file-existence assert weak (F-003), ui-SKIP branch dead (F-004) |
| gate_dry_run_json_prints_the_envelope | YES | Envelope fields asserted |
| rebrand_skeleton::* (8, incl. 3 ignored) | YES (5 active) | Skeleton assertions + error paths; 3 boot-tier ignores carry documented reasons and pointer to the real proof |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cheap invocations only ... NEVER execute the real gate table" | cli.rs:3-4 | REAL (gate table) / OVERSTATED (cheapness) | codegen test compiles catalog-gen cold: 12.18s my run (F-005) |
| "The stub and dry-run behaviors exercised here are the frozen interface other cards rely on" | cli.rs:5-6 | UNDERSTATED COVERAGE | 11 of 17 subcommands have no binary-surface test (F-001) |
| "The JSON log is always written" | cli.rs:155 | WEAKLY PROVEN | is_file() on a shared target path (F-003) |

## 7. Dead / duplicated / unreachable code
The `if !repo_root().join("ui/package.json").is_file()` branch (L148-154) is dead in this tree (ui/package.json exists — verified). No other dead code.

## 8. Dependencies & flags
None beyond std (`Command`, `PathBuf`); uses cargo-provided `CARGO_BIN_EXE_xtask` and `CARGO_MANIFEST_DIR` env at compile time. The codegen test requires `cargo` on PATH at RUN time — an undocumented environment dependency (my bare-PATH run failed exactly there).

## 9. Open questions for the next agent
- The codegen bridge design (xtask spawning `cargo run -p commands --bin catalog-gen`) means every `cargo xtask codegen` invocation can trigger a compile — is that acceptable inside full_gate timing, or should the bridge resolve the prebuilt binary?
- 3 ignored rebrand boot-tier tests claim a nightly tier exists — does any CI lane actually run the nightly tier (rg .github for it), or are these permanently ignored?
