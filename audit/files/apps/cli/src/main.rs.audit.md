# AUDIT: apps/cli/src/main.rs
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-21 |
| Lines / functions | 23 / 1 |
| File verdict | REAL (with a proven S1 panic path) |
| Reachable from shipped entry point? | This IS the shipped entry point — `lumen`. Executed directly: baseline smoke (INDEX.md §MP4 SMOKE, `CMD: timeout -k 5 60 …/lumen decode-bench … EXIT=0`; lumen-engine-info.txt) and this session (`lumen --version` → `lumen 0.1.3`, EXIT=0). |
| Findings | S0: 0 S1: 1 S2: 0 S3: 0 S4: 1 |
| Commands run | Read main.rs; Grep `env::args\(\)` across apps/ (4 sites: cli main.rs:14, control-server main.rs:265, desktop-tauri app.rs:276, examples/emit_stream.rs:62); Grep `args_os` across apps/ + crates/ (ZERO hits — no mitigation anywhere); EXECUTED the pre-built baseline binary: `/home/z/my-project/audit-workspace/target/release/lumen open $'\xff\xfe.mp4'` → panic, EXIT=101 (evidence: audit/runs/W13-A04-argv-panic.txt); `lumen --version` → `lumen 0.1.3` EXIT=0 (binary identity) |

## 1. One-paragraph verdict
The thin-main architecture is real and every baseline smoke run went through this door — but the door itself has a proven panic: `std::env::args()` (main.rs:14) unwraps argv as UTF-8 and the shipped binary ABORTS with exit 101 on any non-UTF-8 argument — executed this session on a media-shaped filename (`\xFF\xFE.mp4`), a perfectly legal Unix path. That crashes `lumen open`/`decode-bench`/`info` on files whose names carry odd bytes, directly contradicting the module's own "Std-only, no panics: every failure is a typed `CommandError`" contract, and no binary in the repo uses the non-panicking `args_os` twin. The rest of the file (bench wall origin, terminal detection, single delegation, ExitCode mapping, flush) is correct.

## 2. What actually works (verified)
- Entry point executes: baseline runs (INDEX.md §MP4 SMOKE: `lumen decode-bench … EXIT=0`; `lumen engine-info` output recorded in lumen-engine-info.txt) and this session: `lumen --version` → `lumen 0.1.3`, EXIT=0.
- `mark_process_start()` is main's first statement, exactly as bench.rs:83 documents (verified side-by-side) — `decode-bench`'s `wall_s` origin is the true process start.
- `interactive = stdin().is_terminal()` — the in-memory test path passes the flag explicitly (bench tests call `run(…, false)`), so the library stays testable as the module doc claims.
- `ExitCode::from(code)` with `code: u8` from `run` — sysexits-class codes (0/65/69/86…, pinned by rc-smoke and bench tests) pass through unmodified.

## 3. Findings
### F-001 [S1]
Where: apps/cli/src/main.rs:14 (with module-doc contract at :3-5)
Code:
```rust
    let argv: Vec<String> = std::env::args().skip(1).collect();
```
and the contract it violates (:3-5):
```rust
//! the whole player is the library, so every mode is testable in-memory
//! (see [`lumen_cli::run`]). Std-only, no panics: every failure is a
//! typed `CommandError` printed by the one honest printer and mapped
```
What is wrong: `std::env::args()` panics on any argument that is not valid UTF-8 (std's documented behavior — the non-panicking twin is `args_os`). EXECUTED PROOF (this session, pre-built baseline binary):
```
$ /home/z/my-project/audit-workspace/target/release/lumen open $'\xff\xfe.mp4'
EXIT=101
thread 'main' (25744) panicked at /rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/library/std/src/env.rs:878:51:
called `Result::unwrap()` on an `Err` value: "\xFF\xFE.mp4"
```
(evidence: audit/runs/W13-A04-argv-panic.txt). The panic fires during `.collect()` at main.rs:14 — before `run`, before any typed error handling, before the "one honest printer".
Why it matters: On the platforms this binary ships for (linux/macos per the parity report), filenames are byte strings and non-UTF-8 names are valid, common media-library realities (old encodings, partial downloads, emoji mangling). `lumen open <such-a-file>` crashes with an uncontrolled panic (exit 101 — not even a sysexits code), contradicting the module's core "no panics" contract. The same pattern ships in three more binaries: apps/control-server/src/main.rs:265, apps/desktop-tauri/src/app.rs:276, apps/desktop/examples/emit_stream.rs:62; `args_os` appears nowhere in the repo (grep: 0 hits).
What correct looks like: `std::env::args_os()` collected to `Vec<OsString>`, with the CLI's parser accepting `OsString` paths (the file lanes ultimately build URIs — percent-encoding can carry arbitrary bytes) and refusing non-UTF-8 FLAGS with a typed usage error (exit 64), never a panic.
Confidence: HIGH — Verification: EXECUTED (pasted output above; binary self-identifies `lumen 0.1.3`, matching baseline).

### F-002 [S4]
Where: apps/cli/src/main.rs:20-21
Code:
```rust
    let _ = out.flush();
    let _ = err.flush();
```
What is wrong: Both flush results are discarded. With a closed stdout pipe (e.g. `lumen --help | head -1`; Rust ignores SIGPIPE, so writes surface as EPIPE `Err`s), any still-buffered output is lost and the process still exits with `run`'s code — a truncated/failed output reports success.
Why it matters: Exit code is the machine contract (the sysexits-class discipline the module doc advertises); output-delivery failure is silently equated with success for piped consumers.
What correct looks like: On flush error, override the exit code with the appropriate sysexits value (e.g. `EX_IOERR`/74 — or at minimum a nonzero), or document the best-effort contract at the flush site.
Confidence: HIGH — Verification: read (the swallow is unconditional; whether `run` itself surfaces earlier write errors is run.rs's lane — UNVERIFIED here).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `main` | 9-23 | REAL | S1 (F-001), S4 (F-002) | Order is correct (bench origin first); stdin/stdout/stderr locked once; single delegation to `lumen_cli::run::run`; ExitCode passthrough. The argv collection is the panic site. |

## 5. Test honesty
No tests in this file (23 lines, thin main by design). The file's behavior is covered indirectly: library-path tests drive `run` in-memory (bench tests, baseline green) and rc-smoke exercised the real binary 6/6 (baseline #20). The F-001 panic path is tested NOWHERE — no test in the workspace passes a non-UTF-8 argument (cargo test argv is always UTF-8); it was found only by executing the binary. A regression test would need `Command::new(binary).arg(OsString-from-bytes)` — the same shape pipeline.rs:317-322 already uses for the stub-binary check.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "thin `main` — the whole player is the library, so every mode is testable in-memory" | module docs :1-3 | REAL | bench tests call `crate::run::run` in-memory; baseline green. |
| "Std-only, no panics: every failure is a typed `CommandError`" | module docs :3-5 | FALSE | F-001: executed panic, exit 101, on a valid Unix filename argument. |
| "mapped to a sysexits-class code" | module docs :4-5 | FALSE for the panic path | Panic exits 101 (Rust default), outside the sysexits discipline. |
| "The wall-clock origin for `decode-bench`'s `wall_s` … main's first statement" | comment :10-12 + bench.rs:83 | REAL | Verified: first statement of main; bench fallback documented. |

## 7. Dead / duplicated / unreachable code
None in this file. The duplicated `env::args()` anti-pattern lives in three sibling binaries (F-001 lists them) — a workspace-wide fix belongs in one card.

## 8. Dependencies & flags
`lumen_cli` (the workspace lib — everything), std only. No feature gates; this exact file compiles into the shipped `lumen` (baseline #1 clean; binary version 0.1.3 self-identifies).

## 9. Open questions for the next agent
- run.rs / parse.rs owners (A01/W13 siblings): does the parser's `SourceSpec::File` path ever see `OsString` (it takes `String` today)? F-001's fix needs the path lane to accept OS strings end-to-end (`file_uri(path)` canonicalization included).
- control-server main.rs owner (W13 sibling): same panic at main.rs:265 — `lumen-control-server --config <non-utf8-path>` aborts identically; grade it there with this evidence as precedent.
- Whoever owns scripts/rc-smoke.sh: adding one non-UTF-8 argv check (expect typed exit 64, not 101) would pin F-001's fix permanently.
