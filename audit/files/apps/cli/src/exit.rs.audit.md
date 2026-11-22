AUDIT: apps/cli/src/exit.rs
===========================

| Field | Value |
|---|---|
| Auditor | W13-A01 |
| Date | 2026-11-17 |
| Lines / functions | 176 / 9 (4 production fns + 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `exit_code` called from run.rs (10 call sites: 77, 93, 103, 114, 123, 157, 169, 211, 346, 364) and bench.rs:235; `print_error_to` from run.rs:496/526; main.rs:22 consumes the returned code. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |

Commands run (static; no cargo per adaptations):
- Read file complete (single pass)
- `rg -n "exit::report|exit::print_error\b|exit_code_of|exit::exit_code" apps/cli` → caller map above (quote verification of reachability)
- W00 baseline cross-ref: `mp4-smoke2-stdout.txt` shows the usage-exit shape (64) this table produces; `unsupported_path.rs` tests pin 64/65/66/69 through the real binary.

## 1. One-paragraph verdict

A 70-line total exit-code table over the catalog's `ErrorCode` vocabulary, a single honest printer, and a genuinely good totality tripwire: `exit_code`'s match is exhaustive without a wildcard arm (a new catalog code fails the BUILD), and the test additionally pins the array length at 18 so a silently-reused mapping fails the test. The sysexits mapping is sensible and matches every exit code the W00 baseline actually observed (64 usage, 65 data, 66 noinput, 69 unavailable). Two hygiene items only: `report` and `print_error` have no callers in the crate (dead public API kept "for main" that main does not use), and the printer-shape unit test admits it cannot capture stderr and instead asserts the `CommandError` Display it is given — a near-tautology saved only by the real binary tests elsewhere.

## 2. What actually works (verified)

— verified by caller map (rg above): `exit_code` (10 run.rs + 1 bench.rs call sites) and `print_error_to` (run.rs teardown/one-shot paths) are live in the shipped binary.
— verified by reading tests: `every_error_code_maps_to_an_exit_code` (114-145) enumerates all 18 codes, asserts the count, and asserts each maps into the allowed sysexits set — a new code without classification fails compile (no wildcard) or this test (len drift). `representative_rows_are_pinned` (149-159) pins all 8 numeric rows.
— verified by cross-file evidence: unsupported_path.rs (this batch) pins 64/65/66/69 with exact stderr shapes through the real binary — the printer's actual output format is under test there.

## 3. Findings

### F-001 [S4] — `report` and `print_error` are dead public API
Where: apps/cli/src/exit.rs:83-88, 99-105
Code:
```rust
pub fn print_error(error: &CommandError) {
    eprintln!("error: {error}");
```
```rust
/// Print one error and answer its exit code (the one-call form `main`
/// uses for startup failures).
#[must_use]
pub fn report(error: &CommandError) -> std::process::ExitCode {
```
What is wrong: rg over apps/cli shows zero callers of `print_error` or `report` (the runner uses the writer-taking `print_error_to`; main.rs uses `run`). The doc comment on `report` claims main uses it — main.rs:19-22 does not.
Why it matters: dead public API whose doc comment states a false usage; drift risk.
Confidence: HIGH — Verification: read + rg caller map.

### F-002 [S4] — the printer-shape unit test tests the inputs, not the printer
Where: apps/cli/src/exit.rs:163-175
Code:
```rust
        // Capture stderr is not possible std-only; assert the shape of
        // the pieces the printer uses instead (Display + summary are
        // the contract under the println!s).
        seen.push(format!("error: {error}"));
```
What is wrong: the test never calls `print_error`/`print_error_to`; it formats the error itself and asserts `CommandError`'s Display and details keys — the `  {key}: {value}` indentation logic (85-87, 94-96) executes in no test in this file. (It IS covered end-to-end by unsupported_path.rs through the real binary, so the risk is contained.)
Why it matters: locally, a regression in the printer's line shapes would pass this file's tests.
What correct looks like: route `print_error_to` into a `Vec<u8>` sink (it already takes `W: Write` — trivially testable; the "std-only cannot capture stderr" excuse applies to `eprintln!` only, not to the twin it tests).
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `exit_code` | 44-70 | REAL | — | exhaustive match, no wildcard; 18 codes → 8 sysexits values; table documented 15-25. |
| `exit_code_of` | 74-76 | REAL | S4 | only caller is dead `report` (F-001). |
| `print_error` | 83-88 | REAL (dead) | S4 | F-001; eprintln-based. |
| `print_error_to` | 92-97 | REAL | — | the live printer (run.rs); `let _ =` on writeln — best-effort stderr, acceptable. |
| `report` | 102-105 | REAL (dead) | S4 | F-001; doc comment claims main uses it — false. |
| `every_error_code_maps_to_an_exit_code` | 114-145 | REAL | — | count tripwire + value-set assert; would fail on catalog growth. |
| `representative_rows_are_pinned` | 149-159 | REAL | — | all 8 rows. |
| `printed_form_is_stage_tagged_with_details` | 163-175 | SHALLOW | S4 | F-002; pins Display + detail keys, not the printer. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| every_error_code_maps_to_an_exit_code | YES | enumeration + count + value-set. |
| representative_rows_are_pinned | YES | exact numbers. |
| printed_form_is_stage_tagged_with_details | NO (for the printer) | never invokes the printer (F-002); would catch a Display regression in the commands crate only. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The totality of the table is pinned by test (every `ErrorCode` variant maps — a new catalog code fails the test until classified)" | exit.rs:27-28 | REAL | test 114-145 + compile-exhaustive match. |
| "`report` … the one-call form `main` uses for startup failures" | exit.rs:99-100 | FALSE | rg: no caller; main.rs uses `run` (main.rs:19). |
| "no path can leak userinfo or full user paths here" | exit.rs:35-37 | REAL-UNPROVEN | redaction is the commands crate's V8 redactor (another wave's file); the observed behavior (`…/missing.avi`, unsupported_path.rs:306) corroborates for the missing-file path. |

## 7. Dead / duplicated / unreachable code

- `print_error`, `report`, and (transitively) `exit_code_of`: dead in apps/cli (F-001). Possibly consumed by other crates? rg was scoped to apps/cli — the functions are `pub` in a lib crate (`lumen_cli`), so external consumers could exist; no such consumer found in apps/desktop or apps/desktop-tauri (quick rg of their Cargo.tomls shows no lumen-cli dependency — UNVERIFIED beyond that).

## 8. Dependencies & flags

- Depends only on `commands` types. No features, no panics, no I/O beyond stderr best-effort.

## 9. Open questions for the next agent

1. commands crate (Wave 2): confirm `CommandError`'s Display actually is `[stage.code]: message` (pinned here at 173) and that the V8 redactor covers `details` values, not just the message (the printer prints values verbatim at 86/95).
