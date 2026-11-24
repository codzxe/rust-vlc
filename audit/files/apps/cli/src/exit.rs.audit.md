# AUDIT: apps/cli/src/exit.rs
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-23 |
| Lines / functions | 176 / 5 prod fns (+3 tests) |
| File verdict | REAL (live half) + dead trio |
| Reachable from shipped entry point? | YES for `exit_code` + `print_error_to`: main.rs:19 → run::run → run.rs/bench.rs call both (every error path of every mode). NO for `print_error`/`exit_code_of`/`report`: zero callers in the tree (only report→print_error/exit_code_of internally; nothing calls report) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `./target/debug/deps/lumen_cli-9887c2f6df9cd333 exit::` → 3 passed / 0 failed (exit 0); rg (exhaustive, apps/ + Cargo.toml reverse deps) for callers; cross-read crates/commands/src/error.rs (redaction, BTreeMap). |

## 1. One-paragraph verdict
The exit-code table is real, exhaustive by construction (no wildcard arm — a new `ErrorCode` variant breaks the build), pinned by an honest totality test (18 variants, len-pinned), and its rows are confirmed by executions from my other files: 64 usage, 65 unsupported (garbage-file bench test, HEVC open per baseline), 69 not-ready (play-without-session test), 70 internal. Determinism of detail lines is guaranteed upstream (`details: BTreeMap`, commands/src/error.rs:158) and the redaction claim checks out (`CommandError::new`/`with_detail` route through `redact`, error.rs:163-175). The rot: `print_error` (the global-stderr printer), `exit_code_of`, and `report` form a dead call chain — nothing in the tree calls `report` — while the doc on `print_error` claims "This is the ONLY error printer in the app — `main` routes every failure through it", which is false on both halves (main calls only `run::run`; the used printer is the writer twin `print_error_to`).

## 2. What actually works (verified)
— `./target/debug/deps/lumen_cli-9887c2f6df9cd333 exit::` → 3/3 pass (totality + spot pins + form).
— Executed cross-evidence this session: exit 64 (run.rs usage tests), 65 (bench garbage test; baseline HEVC smoke), 69 (play-without-session), 0/86 (other lanes) — all match the table.

## 3. Findings
F-001 [S3]
Where: apps/cli/src/exit.rs:78-105
Code:
```
/// This is the ONLY error printer in the app — `main` routes every
/// failure through it so the output shape cannot drift per mode.
pub fn print_error(error: &CommandError) {
```
What is wrong: `report` (99-105) has zero callers in the tree (rg over apps/, crates/, tests/ — the only hits for `print_error(`/`exit_code_of(` are inside exit.rs itself, lines 103-104); `print_error` and `exit_code_of` are reachable only through the dead `report`. The doc claim is false: `main` (apps/cli/src/main.rs:19-22) calls `run::run` and flushes — it routes nothing through this printer; the actual single printer in use is `print_error_to` (run.rs:526, bench.rs:234). Why it matters: dead public API + a doc that misdescribes the app's error-printing architecture. What correct looks like: delete the trio (or make main use `report` for real startup failures). Confidence: HIGH — Verification: read (exhaustive rg) + ran (tests).
F-002 [S4]
Where: apps/cli/src/exit.rs:163-175
Code:
```
    fn printed_form_is_stage_tagged_with_details() {
        let error = CommandError::new(Stage::Probe, ErrorCode::Unsupported, "no signature matched")
            .with_detail("format", "unknown");
        let mut seen = Vec::new();
```
What is wrong: the test never calls either printer — it formats the strings itself (`seen.push(format!("error: {error}"))`) and asserts its own construction; it verifies `CommandError`'s Display + details keys, not `print_error`/`print_error_to`. The name promises the printed form. (The printer itself does have real coverage via run.rs's run_lines assertions on `[session.not-ready]` etc.) The in-code comment half-admits this ("Capture stderr is not possible std-only"). Confidence: HIGH — Verification: read + ran.

## 4. Per-function table (mandatory, >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| exit_code | 44-70 | REAL | — | exhaustive match, no wildcard; table rows execution-confirmed (64/65/69/70) |
| exit_code_of | 73-76 | UNREACHABLE | — | only caller is dead `report` (F-001) |
| print_error | 83-88 | UNREACHABLE | S3 | F-001; shape identical to the live twin |
| print_error_to | 92-97 | REAL | — | THE printer: run.rs + bench.rs every error path |
| report | 101-105 | UNREACHABLE | S3 | zero callers; doc claims main uses it (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| every_error_code_maps_to_an_exit_code | YES (twice) | exhaustive match = compile error on a new variant; `all.len()==18` pin fails on add/remove; sysexits-set assertion catches bogus mappings |
| representative_rows_are_pinned | YES | 8 exact pins |
| printed_form_is_stage_tagged_with_details | NO (for the printers) | near-tautology: asserts strings it built itself; tests CommandError Display, not print* (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The totality of the table is pinned by test (every ErrorCode variant maps)" | exit.rs:27-28 | REAL | test at 114-145, executed 3/3 |
| "[CommandError]'s constructor already routes message and details through … V8 redactor" | exit.rs:35-37 | REAL | commands/src/error.rs:163-175 `redact(&message)`, `redact(key)/redact(&value)` |
| "This is the ONLY error printer in the app — main routes every failure through it" | exit.rs:81-82 | FALSE | main.rs:19 calls run::run only; print_error has zero live callers; print_error_to is the used one |
| sysexits table rows (64/65/66/69/70/71/74/77) | exit.rs:15-25 | REAL | exit.rs:45-69 match + representative test + executed exits this session |

## 7. Dead / duplicated / unreachable code
- Dead chain: `report` → `print_error` + `exit_code_of` (F-001).

## 8. Dependencies & flags
- commands crate only (CommandError, ErrorCode). No feature gates. Deterministic detail order via upstream BTreeMap (verified).

## 9. Open questions for the next agent
- tests/compat and tests/integration/first-vertical depend on lumen-cli — if either wants a process-level exit-code harness, `report` is the natural seam; otherwise it should go.
