# AUDIT: apps/cli/src/exit.rs
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2026-11-21 |
| Lines / functions | 176 / 8 (5 production fns, 3 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES for `exit_code`/`exit_code_of`/`print_error_to` (call chain: `lumen` main.rs:23 → `run::run` → run.rs:169 `exit::exit_code(error.code)` and 12+ more call sites; print_error_to at run.rs:496, :526 and bench.rs:234). `print_error` and `report`: NO caller anywhere in apps/cli (dead pub API) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `rg -n "exit::\|exit_code\|print_error\|report\(" apps/cli/src/main.rs apps/cli/src/run.rs apps/cli/src/dispatch.rs` (all call sites); `rg -rn "print_error\b\|exit::report" apps/cli/ --type rust` (zero callers for print_error/report, only a doc mention at run.rs:65); `cat apps/cli/src/main.rs` (main routes through run::run, never the printer); `sed -n '480,530p' apps/cli/src/run.rs` (print_error_to is the real printer); `rg -n "exit::tests" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (all 3 tests ok :2860-2862); Read/sed on every quoted range (verbatim verified) |

## 1. One-paragraph verdict
The exit-code table is real, total, and pinned: 18 `ErrorCode` variants → 8 sysexits values, the module-doc table matches the match arms exactly, `exit_code` is called from a dozen real run.rs sites, and all three tests passed in the orchestrator run (:2860-2862). Two functions are dead with false doc claims: `print_error` claims "This is the ONLY error printer in the app — `main` routes every failure through it" while main.rs:14-23 routes everything through `run::run` → `print_error_to` (the writer twin) and nothing calls `print_error`; `report` claims to be "the one-call form `main` uses for startup failures" — main has no such call site. And the one test that names the printed form never invokes any printer — it hand-builds the expected string with its own `format!` and asserts the commands crate's Display, a tautology for this module.

## 2. What actually works (verified)
- `exit::tests::{every_error_code_maps_to_an_exit_code, printed_form_is_stage_tagged_with_details, representative_rows_are_pinned}` → all `ok`, orchrun-test-workspace-nofailfast.log.txt:2860-2862.
- Exit codes reach the process: main.rs:23 `std::process::ExitCode::from(code)` where code comes from run.rs's `exit::exit_code(...)` sites (e.g. run.rs:169 one-shot dispatch failure, :77/:93/:114 arg failures).
- `print_error_to` is the production printer: run.rs:496 (teardown_step), run.rs:526 (print_response error arm), bench.rs:234.
- Table/doc consistency: all 8 rows of the module-doc table (:15-25) match the match arms (:47-68) — verified row by row; `representative_rows_are_pinned` pins all 8 distinct exit values.

## 3. Findings
### F-001 [S3]
Where: apps/cli/src/exit.rs:78-88 and :99-105
Code:
```
/// This is the ONLY error printer in the app — `main` routes every
/// failure through it so the output shape cannot drift per mode.
pub fn print_error(error: &CommandError) {
    eprintln!("error: {error}");
    for (key, value) in &error.details {
        eprintln!("  {key}: {value}");
    }
}
```
and
```
/// Print one error and answer its exit code (the one-call form `main`
/// uses for startup failures).
#[must_use]
pub fn report(error: &CommandError) -> std::process::ExitCode {
```
What is wrong: Both doc claims are false and both functions are dead. main.rs (the entire 24-line binary) never imports or calls either: it delegates to `lumen_cli::run::run(&argv, …, &mut err)` and converts the returned code. rg over the whole app finds zero callers for `print_error` and `report` (the only mention is run.rs:65's doc link). The app's real printer is the writer-taking twin `print_error_to` (run.rs:496/:526, bench.rs:234).
Why it matters: The docs assert a wiring that does not exist — a reader auditing output-shape drift would look here, and a future contributor "fixing" the printer by editing `print_error` would change nothing. Dead pub API with authoritative-sounding claims is exactly the fakeness pattern this audit hunts, at small scale.
What correct looks like: Delete `print_error` and `report` (or route them through `print_error_to` to stderr and fix the docs to name run.rs's actual paths).
Confidence: HIGH — Verification: read + rg (zero callers; main.rs read in full).

### F-002 [S3]
Where: apps/cli/src/exit.rs:163-175
Code:
```
    #[test]
    fn printed_form_is_stage_tagged_with_details() {
        let error = CommandError::new(Stage::Probe, ErrorCode::Unsupported, "no signature matched")
            .with_detail("format", "unknown");
        let mut seen = Vec::new();
        // Capture stderr is not possible std-only; assert the shape of
        // the pieces the printer uses instead (Display + summary are
        // the contract under the println!s).
        seen.push(format!("error: {error}"));
```
What is wrong: The test never calls `print_error`, `print_error_to`, or `report`. It constructs the expected line itself with its own `format!("error: {error}")` and asserts the commands crate's `Display` output plus a details-key listing. The "Capture stderr is not possible std-only" excuse is false in spirit: `print_error_to` (92-97) exists precisely as the parameterized sink — `let mut buf = Vec::new(); print_error_to(&error, &mut buf)` is std-only and captures everything.
Why it matters: A printer regression (wrong prefix, dropped details, wrong indentation, details printed to stdout) passes this test untouched. The test name promises "printed form" coverage; the body tests a different crate's Display — a tautology for this module (checklist: "tests asserting nothing / names promising more than the body").
What correct looks like: `let mut sink = Vec::new(); print_error_to(&error, &mut sink); assert_eq!(String::from_utf8(sink).unwrap(), "error: [probe.unsupported]: no signature matched\n  format: unknown\n");`
Confidence: HIGH — Verification: read (no printer call in the body; print_error_to signature verified).

### F-003 [S4]
Where: apps/cli/src/exit.rs:112-136
Code:
```
    /// Every catalog `ErrorCode` variant must classify — a new code
    /// added to the catalog without an exit mapping fails here.
    #[test]
    fn every_error_code_maps_to_an_exit_code() {
        let all = [
```
and at :136
```
        assert_eq!(all.len(), 18, "the catalog gained a code — classify it");
```
What is wrong: The totality is enforced by the compiler (the `match` at :45-69 is exhaustive over the enum — a new variant breaks compilation of `exit_code`), not by this test: the hand-written `all` array only ever contains the 18 variants the author typed. A new variant added to `ErrorCode` plus its match arm compiles and passes this test with the new code uncovered; the len==18 pin does not detect an addition (the array does not grow by itself).
Why it matters: The comment credits the test with a guarantee the match provides; the honest statement is "the match is exhaustive by construction; this test pins the current count and the sysexits codomain."
What correct looks like: Reword the comment, or derive the variant list (`strum::EnumIter`-style) if the commands crate ever grows codes.
Confidence: HIGH — Verification: read (match exhaustiveness is Rust semantics).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `exit_code` | 44-70 | REAL | — | Total 18-arm match → 8 sysexits values; doc table matches arms exactly (verified row-by-row); called from 12+ run.rs sites. |
| `exit_code_of` | 73-76 | REAL | — | Thin ExitCode wrapper; used by dead `report` only — no production caller found beyond it; harmless twin of the u8 form actually used. |
| `print_error` | 83-88 | UNREACHABLE | S3 | Zero callers; false "ONLY error printer / main routes through it" doc (F-001). |
| `print_error_to` | 92-97 | REAL | — | The real printer (run.rs:496/:526, bench.rs:234); `let _ =` writeln discards write errors — acceptable for stderr/Vec sinks, standard idiom. |
| `report` | 101-105 | UNREACHABLE | S3 | Zero callers; false "the one-call form main uses" doc (F-001). |
| `tests::every_error_code_maps_to_an_exit_code` | 114-145 | REAL | S4 | Codomain pinned (64/65/66/69/70/71/74/77); totality is compiler-enforced, comment overclaims (F-003). |
| `tests::representative_rows_are_pinned` | 149-159 | REAL | — | All 8 distinct exit values pinned — catches a remapped class. |
| `tests::printed_form_is_stage_tagged_with_details` | 163-175 | SHALLOW | S3 | Never invokes a printer; tautology for this module (F-002). |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| every_error_code_maps_to_an_exit_code | YES (codomain) / NO (new variant) | A mapped value outside the sysexits set fails; a NEW ErrorCode variant is caught by compilation of exit_code, not this test (F-003). |
| representative_rows_are_pinned | YES | Exact 8-value pinning — any reclassification fails. |
| printed_form_is_stage_tagged_with_details | NO (for this module) | The printer is never called; it tests CommandError's Display (commands crate's contract), not exit.rs's printing (F-002). |
All executed: no-failfast :2860-2862.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Exit codes map EngineError stages deterministically [PROPOSAL]" (doc 01 §2.8) | module docs :4-5 | REAL | exit_code total match; run.rs sites; main.rs:23 conversion. |
| sysexits table (8 rows, 18 codes) | module docs :15-25 | REAL | Arms :47-68 match the table exactly; representative test pins all 8 values. |
| "The totality of the table is pinned by test (every ErrorCode variant maps — a new catalog code fails the test until classified)" | :27-28 | SHALLOW | Match exhaustiveness is the real guard (F-003). |
| "This is the ONLY error printer in the app — main routes every failure through it" | :81-82 | FALSE | Zero callers; run.rs uses print_error_to (F-001). |
| "the one-call form `main` uses for startup failures" (report) | :99-100 | FALSE | main.rs has no report call (F-001). |
| "no path can leak userinfo or full user paths here" (redactor claim) | :32-37 | REAL-UNPROVEN | Depends on CommandError's constructor routing through the V8 redactor (commands crate, other wave); not contradicted here; redaction itself UNVERIFIED in this audit. |
| Baseline: "The refused paths exit 0 (High 4:4:4 MP4, HEVC MP4)" | baseline INDEX CORRECTION v2 | REAL (not this file's defect) | decode.unsupported arrives as an async event in the interactive loop; the session ends normally → run returns 0; exit.rs only maps CommandErrors — the exit-0-on-failed-open decision lives in run.rs (other agent's chunk). |

## 7. Dead / duplicated / unreachable code
`print_error` and `report` — zero callers in the app (F-001). `exit_code_of` has no production caller either (only the dead `report` uses it) — the u8 `exit_code` form is what run.rs consumes. All three are `pub` in a lib crate, so they compile into the shipped artifact as unused surface.

## 8. Dependencies & flags
Single import: `commands::{CommandError, ErrorCode}`. std-only, no feature gates, compiled unconditionally into `lumen`. No unsafe, no panics (writeln results explicitly discarded in the writer twin — deliberate).

## 9. Open questions for the next agent
- run.rs owner (parts 1-2): confirm whether the interactive loop's exit-0-after-error-event behavior (baseline CORRECTION v2's standing defect "a failed open is reported as success by exit code") should map `PlaybackState::Error` at session end to a nonzero exit — exit.rs already offers the vocabulary (`ErrorCode::Decoder` → 65).
- commands crate owner: verify the V8-redactor claim (:35-37) — cited as the reason print paths are secret-safe.
