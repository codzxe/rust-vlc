# AUDIT: apps/cli/src/main.rs
| Field | Value |
|---|---|
| Auditor | W13-A05 |
| Date | 2025-06-27 |
| Lines / functions | 23 / 1 (`main`) |
| File verdict | REAL — with one executed S1 contract violation |
| Reachable from shipped entry point? | IS the shipped entry point (`lumen` bin, release lane `-p lumen-cli`) |
| Findings | S0: 0 S1: 1 S2: 0 S3: 0 S4: 1 |
| Commands run | `./target/debug/lumen $'\xff\xfe'` → PANIC, exit 101 (executed); `--version` → 0; bare → 64; `--json engine-info` → 0; unknown verb → 64; bare source → 66; `decode-bench ... --bogus-flag` → 64; baseline tests cross-ref |

## 1. One-paragraph verdict
A 23-line thin shell that does what it says — mark the bench clock origin, collect argv, dispatch to the testable `lumen_cli::run::run`, flush, map the code — and every mode I executed behaves (`--version` 0, bare usage 64, `--json engine-info` 0 NDJSON, unknown verb 64, VLC-style bare source 66). The hostile-focus answer: argv parsing is delegated to `parse::parse_argv` (apps/cli/src/parse.rs:151-175), which knows EXACTLY 4 global flags — `--json`, `--control-stdio`, `--version`/`-v`, `-h`/`--help` — confirming Wave 11's count; the only other flags in the grammar are `open`'s four (`--start-time`, `--start-paused`, `--no-video`, `--no-audio`, parse.rs:241-267); global flags are position-independent (stripped anywhere in argv, so `lumen open clip.avi --json` is legal and the doc's "flags precede the verb" rule is unenforced — already filed by the parse.rs auditor); unknown `--tokens` are not flag-parsed at all, they fall to the verb slot (typed "unknown command", or the path-shaped bare-source heuristic). But the file's header contract — "Std-only, no panics: every failure is a typed `CommandError`" — is FALSE in the one line the file owns: `std::env::args()` PANICS on non-UTF8 argv. I executed it: `./lumen $'\xff\xfe'` aborts with a Rust panic (exit 101, rustc path leaked) before a single line of player code runs. On Linux, where filenames are byte strings, a legal invocation with a non-UTF8 media filename kills the binary with a panic instead of the promised typed error.

## 2. What actually works (verified)
— `./target/debug/lumen --version` → `lumen 0.1.3`, exit 0.
— `./target/debug/lumen` → usage on stderr, exit 64 (real code, not the pipeline's).
— `./target/debug/lumen --json engine-info` → NDJSON envelope, exit 0.
— `./target/debug/lumen bogus-verb` → exit 64; `./target/debug/lumen clip-nonexistent.avi` → exit 66 (bare-source heuristic → open path → input.inaccessible).
— `lumen_cli::run` (the whole player) is covered by the baseline suite (4744 passed / 2 failed, audit/runs/baseline/cargo-test-workspace.txt) and my own bench_decode/pipeline runs.

## 3. Findings
F-001 [S1]
Where: apps/cli/src/main.rs:14 (vs the contract at main.rs:3-5)
Code: (verbatim)
```
    let argv: Vec<String> = std::env::args().skip(1).collect();
```
and the contract it violates (main.rs:3-5, verbatim):
```
//! (see [`lumen_cli::run`]). Std-only, no panics: every failure is a
//! typed `CommandError` printed by the one honest printer and mapped
//! to a sysexits-class code.
```
What is wrong: `std::env::args()` panics on any argument that is not valid Unicode (documented std behavior; the iterator unwraps the OsString→String conversion). EXECUTED: `./target/debug/lumen $'\xff\xfe'` →
```
thread 'main' (24503) panicked at .../library/std/src/env.rs:878:51:
called `Result::unwrap()` on an `Err` value: "\xFF\xFE"
```
real exit code 101. A Linux media player receives byte-string filenames; `lumen open $'\xff\xfe.mp4'` is a legal invocation with a legal filename, and the binary dies with a panic (plus a rustc library-path leak) instead of the typed `CommandError`/sysexits answer the file's own header promises — the exact "crash on valid input" the S1 bar names, and no test can catch it because every in-memory test constructs `Vec<String>` argv after this line has already run. What correct looks like: `std::env::args_os()` + a typed refusal (or lossy conversion with a warning) for non-UTF8 tokens, exit 64/65. Confidence: HIGH — Verification: ran (output above).
F-002 [S4]
Where: apps/cli/src/main.rs:20-21
Code: (verbatim)
```
    let _ = out.flush();
    let _ = err.flush();
```
What is wrong: exit-time flush failures are silently discarded — consistent with run.rs's universal `let _ = writeln!` posture (a broken stdout pipe loses output and errors without a signal anywhere). Nothing actionable at process exit, but it completes a "writes can never fail" pattern that hides EPIPE. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 9-23 | REAL | S1 (F-001) | otherwise correct: bench clock origin before parse; stdin-terminal detection feeds the REPL prompt (run.rs:221); ExitCode::from(u8) |

## 5. Test honesty
No tests in this file (23 lines, nothing to unit-test in isolation — the modes are tested in-memory via `lumen_cli::run` in run.rs's suite and through the binary in bench_decode.rs). GAP: no test anywhere spawns the binary with non-UTF8 argv — structurally impossible to catch via `Vec<String>` harnesses; only a subprocess test with `OsString` args (bench_decode.rs's `Command` pattern could do it) would pin F-001.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only, no panics: every failure is a typed CommandError ... mapped to a sysexits-class code" | main.rs:3-5 | FALSE | executed non-UTF8 argv → panic, exit 101 (F-001) |
| "thin main — the whole player is the library, so every mode is testable in-memory" | main.rs:1-3 | REAL | run.rs in-memory suite + my binary runs agree |
| "The wall-clock origin for decode-bench's wall_s (true process wall)" | main.rs:10-12 | REAL | bench_decode.rs pins wall_s ≥ decode_s on the real binary; `mark_process_start` called first (main.rs:13) |

## 7. Dead / duplicated / unreachable code
None in this file. (Context note: run.rs:219 `let _ = interactive;` is a dead statement immediately before the value is used at run.rs:221 — run.rs's owner should delete it.)

## 8. Dependencies & flags
No new deps; std only (`IsTerminal`, `Write`). The complete flag surface this entry reaches: global `--json`, `--control-stdio`, `--version`/`-v`, `-h`/`--help` (parse.rs:156-159) + open's `--start-time <T>`, `--start-paused`, `--no-video`, `--no-audio` (parse.rs:241-267). Wave 11's "only 4 flags known" is CONFIRMED as the global set; the parse.rs auditor already recorded doc 05's promised-but-missing `--rate` and the unenforced flag-position rule.

## 9. Open questions for the next agent
- Does `lumen-tauri` (or any other argv-consuming entry point) share this non-UTF8 panic? Its main constructs its own argv handling — same `env::args()` pattern would replicate F-001 there.
- If F-001 is fixed with `args_os`, decide the typed-refusal code (64 invalid-argument vs 65 dataerr) and pin it with a subprocess test using `std::os::unix::process::CommandExt::arg0`-style raw bytes.
