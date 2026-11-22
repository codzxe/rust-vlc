# AUDIT: apps/cli/src/main.rs

| Field | Value |
|---|---|
| Auditor | W13-A03 |
| Date | 2026-11-22 |
| Lines / functions | 23 / 1 (`main`) |
| File verdict | REAL-UNPROVEN (thin, correct; its observable behavior captured in W00 baselines) |
| Reachable from shipped entry point? | It IS the shipped entry point (`[[bin]] name = "lumen"`, apps/cli/Cargo.toml). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `sed -n` quote verification; W00 baseline outputs (lumen-help.txt, mp4-smoke*.txt, *-smoke-bench.txt) prove the binary ran. NO cargo (adaptation). |

## 1. One-paragraph verdict

A correct thin `main`: seeds the bench wall origin first (so `decode-bench`'s `wall_s` is the true process wall), collects argv, detects terminal stdin, delegates everything to `lumen_cli::run::run`, flushes, maps the exit code. No logic, no panics of its own. The W00 baselines prove this binary executed end-to-end (help text, three smoke scripts, four bench runs). The single caveat is behavioral, not structural: the observed `lumen open <file>` one-shot smoke timed out (`exit=124`, audit/runs/baseline/mp4-smoke.txt SMOKE 1) because argv-mode `open` enters the interactive REPL loop and blocks on stdin that the harness never closed — a run.rs UX decision this file faithfully forwards (`interactive = stdin().is_terminal()`), and the concrete shape of the owner's "an MP4 video cannot be opened properly" complaint when driven non-interactively.

## 2. What actually works (verified)

- W00 baseline executions of this exact binary: `lumen --help` output captured (lumen-help.txt), scripted session runs (mp4-smoke2-stderr.txt, mjpeg-smoke-bench.txt), `decode-bench` runs with BENCH lines (mp4-smoke-bench.txt, exit=0).
- `mark_process_start()` before argv handling — the bench wall origin is honest in the binary path (bench.rs:78-92 consumes it).
- `ExitCode::from(code)` — the exit-code table (exit.rs) reaches the OS verbatim (W00 recorded 0/64/65/66/124 across its runs).

## 3. Findings

### F-001 [S4] — The entry forwards an interaction model that hangs non-interactive callers

Where: apps/cli/src/main.rs:15-19

Code (main.rs:14-19, verbatim):
```rust
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let interactive = std::io::stdin().is_terminal();
    let mut input = std::io::stdin().lock();
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    let code = lumen_cli::run::run(&argv, &mut input, &mut out, &mut err, interactive);
```

What is wrong: nothing in THIS file — the hazard lives in run.rs's mode selection (a media-bearing argv enters `repl_mode`, which reads stdin lines forever; with a never-closing stdin the process never exits). The W00 one-shot smoke hit exactly that: `=== SMOKE 1: one-shot open === exit=124` (audit/runs/baseline/mp4-smoke.txt) — the harness's timeout kill. The file's only contribution is that `interactive` is computed but the blocking behavior does not depend on it (a piped-but-open stdin also blocks, just without the prompt). Recorded here because this is the shipped surface where the hang is observed; the fix (EOF/`--wait`/timeout policy) belongs to run.rs (another agent's file).
Confidence: HIGH — Verification: read + W00 smoke output.
Severity: S4 in this file (faithful forwarder); the run.rs-side defect is for that file's auditor to grade.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `main` | 9-23 | REAL-UNPROVEN | S4 (F-001) | Thin, ordered correctly (bench origin first); flush errors deliberately ignored on exit (`let _ =`) — acceptable at process end. |

## 5. Test honesty

No tests in this file; the binary is exercised by tests/container_conformance.rs and tests/bench_decode.rs via `CARGO_BIN_EXE_lumen` (both audited: real-binary harnesses, unexecuted in the baseline).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "thin main — the whole player is the library, so every mode is testable in-memory" | :1-5 | REAL | run.rs drives every mode through the library; the in-memory tests (bench.rs tests, native_seek, pipeline_accounting) exist. |
| "the wall-clock origin for decode-bench's wall_s (true process wall)" | :10-12 | REAL | bench.rs:84-92 consumes it; W00 BENCH lines carry wall_s. |

## 7. Dead / duplicated / unreachable code

- None.

## 8. Dependencies & flags

- `[[bin]] name = "lumen"` (apps/cli/Cargo.toml); std-only; no feature gates.

## 9. Open questions for the next agent

- run.rs's auditor should grade the REPL-blocks-on-open-stdin behavior against the owner's "cannot open an MP4" complaint — with the W00 evidence (exit=124) this is the leading user-facing reproducer, ahead of any decode question (see my bench.rs audit: the "0 frames decoded" story was the bench allowlist, not the player).
