# AUDIT: tools/xtask/src/main.rs

| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-16 |
| Lines / functions | 193 / 8 (4 free fns + 4 impl methods; 3 unit tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (`cargo xtask` alias in `.cargo/config.toml` (`xtask = "run -p xtask --locked --"`) → `xtask::main` → `dispatch` → `cmd::lookup` → `Subcommand::run`; binary-surface integration test `tools/xtask/tests/cli.rs` executes the compiled binary directly) |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | Read full file; `grep -rn NotYetImplemented src/` (matches only at main.rs:42, :79, :167 — test-only construction); `sed -n '16,23p;40,47p;119,131p'` (quote verification); `ls tools/xtask/tests/` (cli.rs present). All read-only; no cargo build/test run. |

## 1. One-paragraph verdict (blunt: what it claims vs what it does)

main.rs claims to be the frozen dispatch surface of a 17-subcommand task runner with a stable 0/1/2 exit-code contract, no panics/unwraps, and typed errors. It does exactly that: argv parsing is real, `cmd::lookup` dispatches to real registered functions (verified: all 17 registered subcommands resolve to `pub fn run` in their modules), unknown commands exit 2 with the known-name list, and the error type carries context into stderr. Nothing here fabricates output — this file produces no analysis at all, only routing. The two defects are housekeeping: a dead `NotYetImplemented` error variant that nothing constructs outside its own unit test, and an exit-code doc that undersells who uses exit 1.

## 2. What actually works (verified) — with the command you ran and its result

- Dispatch is genuine: `main` → `dispatch` → `cmd::lookup(name)` → `(sub.run)(&args[1..])` (lines 119-131). Verified by reading; the `cargo xtask` alias exists in `.cargo/config.toml` (read), and `tools/xtask/tests/cli.rs::run_xtask` spawns `CARGO_BIN_EXE_xtask` with `help`, asserting exit 0 and the frozen names — the compiled binary is exercised by tests (I did not run cargo test; orchestrator owns builds).
- Exit codes: no-subcommand → usage error on stderr + exit 2 (lines 113-118, 104-107); unknown subcommand → exit 2 (lines 126-130); `help` → exit 0 (lines 120-123). All match the module contract at lines 16-23.
- No `unwrap`/`expect`/`panic!`/`todo!` outside `#[cfg(test)]` (grep over the file: only the test module's `unreachable!` guards are `#[track_caller]` test helpers; the production path is panic-free). Verified by reading the whole file.
- `XtaskError` is a real typed error: `Display` (75-89) and `std::error::Error::source` (91-98) wire the `Io` variant's underlying `std::io::Error` through.

## 3. Findings

### F-M01 [S3] — Dead `NotYetImplemented` variant; the "unimplemented stub" exit-2 source no longer exists
Where: tools/xtask/src/main.rs:40-47, 79-83, 161-176
Code (40-47):
```rust
    /// Pre-registered subcommand whose owning card has not landed yet.
    /// Exit code 2.
    NotYetImplemented {
        /// Subcommand name as typed by the user.
        subcommand: &'static str,
        /// Owning card in `docs/13-agent-wave-program.md`.
        card: &'static str,
    },
```
What is wrong: since every registered subcommand flipped `implemented: true` (cmd/mod.rs:238-262), no production code path constructs this variant. `grep -rn NotYetImplemented tools/xtask/src/` returns only the enum definition (main.rs:42), its `Display` arm (main.rs:79), and its unit test (main.rs:167). It is dead production code kept alive by its own test.
Why it matters: dead error arms with dedicated docs invite the belief that stub behavior is still a live mechanism; auditors and CI authors reading "exit 2 = unimplemented stub" will look for stubs that cannot exist. `print_usage`'s "not yet implemented" label branch (lines 142-146) is likewise unreachable.
What correct looks like: either delete the variant and the label branch, or keep one genuine stub registered with `implemented: false` so the mechanism has a real user.
Confidence: HIGH
Verification: read (grep across src/ proving zero non-test constructors).

### F-M02 [S4] — Exit-code contract doc names only `gate` as the exit-1 producer
Where: tools/xtask/src/main.rs:16-23
Code (18-20):
```rust
//! * `1` — `gate` ran to a verdict and at least one gate command failed
//!   (doc 10 §2.4 `result: FAILURE`).
```
What is wrong: exit 1 is also the FAILURE/drift code of `check-architecture` (check_architecture.rs:196-200), `sbom --check` drift (sbom.rs:194, 212), and the other violation-reporting subcommands. The "stable contract CI and scripts depend on" under-enumerates the producers of exit 1.
Why it matters: a script author distinguishing "tree broken" from "tool broken" via this doc alone will mis-prioritize a check-architecture FAILURE as a tool error is impossible — but the doc still misleads about which commands can yield 1 at all.
What correct looks like: "1 — a ran-to-verdict FAILURE from any verdict-reporting subcommand (gate, check-architecture, feature-audit, sbom --check, ...)".
Confidence: HIGH
Verification: read (cross-referenced exit sites in sbom.rs/check_architecture.rs).

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `XtaskError::internal` | 60-64 | REAL | — | Constructor, fine. |
| `XtaskError::io` | 66-73 | REAL | — | Attaches context; used by all subcommands. |
| `Display::fmt` (XtaskError) | 75-89 | REAL | — | Correct per-variant rendering. |
| `Error::source` | 91-98 | REAL | — | Wires `Io` source only; correct. |
| `main` | 100-109 | REAL | — | argv→dispatch, exit 2 on error; matches contract. |
| `dispatch` | 112-132 | REAL | — | help/-h, lookup, unknown→exit-2 with name list. |
| `print_usage` | 135-154 | REAL | S4 | Unreachable "not yet implemented" label branch (F-M01). |
| `eprint_usage` | 157-159 | REAL | — | Fine. |
| tests (3) | 161-193 | HONEST | — | Error-rendering assertions; would fail if Display regressed. |

## 5. Test honesty

The three unit tests assert exact `XtaskError` `Display` strings — if the error rendering were broken, they fail. They test only the error type, not dispatch; dispatch is covered by `tools/xtask/tests/cli.rs` against the compiled binary (exit codes 0/2 asserted there). The unit tests would NOT catch a broken registry (that is mod.rs's tests' job) — acceptable layering.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "FROZEN INTERFACE — read before editing … frozen after card 1.A3" (lines 3-14) | module header | MISLEADING (see mod.rs audit F-C01) | mod.rs itself accreted 9 post-freeze registrations citing granted precedent (mod.rs:20, 23-31). |
| Exit codes: 0/1/2 with "1 — gate … FAILURE" | lines 16-23 | PARTLY TRUE | Exit 1 producers include check-architecture/sbom drift (F-M02). |
| "deny unwrap/expect/panic!/todo! … this crate propagates typed errors" | lines 26-28 | TRUE (this file) | Full-file read: no such macros outside `#[cfg(test)]`. |

## 7. Dead / duplicated / unreachable code

- `XtaskError::NotYetImplemented` (F-M01): dead in production; only its own test constructs it.
- `print_usage`'s `implemented`-false label branch: unreachable while all registry entries are `implemented: true`.

## 8. Dependencies & flags

- std only (`fmt`, `process::ExitCode`). No args parsing beyond `first` + pass-through; per-subcommand flags are owned by each module.
- `mod cmd` dependency is the only crate-internal edge; no filesystem or process work here.

## 9. Open questions for the next agent

- Is any CI script actually branching on the 0/1/2 triad? `scripts/full_gate.sh` uses `if cargo xtask <cmd>` shell logic — exit-1 semantics matter there; confirm no consumer conflates 1 and 2.
- If the registry ever re-adds a stub, the dead `NotYetImplemented` path resurrects — decide delete-vs-keep now, before it rots further.
