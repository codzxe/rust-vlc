# AUDIT: apps/desktop/src/diagnostics.rs
| Field | Value |
|---|---|
| Auditor | W13-A09 |
| Date | 2026-11-21 |
| Lines / functions | 458 / 21 items (11 production + 6 tests + 3 fixture types + mutex) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (by construction): `lumen-tauri` main (apps/desktop-tauri/src/main.rs:49 `desktop::diagnostics::install_panic_hook();`, :53 `write_crash_log("startup", &error)`). Caveat: the tauri shell itself did not build in the sandbox (baseline #2, missing gdk/gtk dev libs — sandbox limitation, INDEX "Tooling gaps"), so the END-TO-END lane is UNVERIFIED-BUILD; the desktop lib (this file) compiled and its tests ran green (test-workspace.log.txt:2302-2307 and nofailfast:2261-2266). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | Read diagnostics.rs whole; rg `install_panic_hook\|write_crash_log\|crash_log_path` over repo (sole production caller apps/desktop-tauri/src/main.rs:49,53; no other callers); rg `crash_log_path_with_stamp` outside the file (zero callers — test-only); sed desktop-tauri/src/main.rs:40-70 (consumption verified); rg version in apps/desktop-tauri/Cargo.toml (0.1.3 = desktop's CARGO_PKG_VERSION — lockstep currently holds); baseline test logs rg `crash_log\|panic_hook\|iso_utc\|failing_sink\|with_stamp` (all 6 tests ok in BOTH runs). |

## 1. One-paragraph verdict
A small, disciplined, genuinely tested diagnostics module: a fixed latest-wins crash sink under `<tmp>/lumen/lumen-tauri-crash.log`, a wrapping panic hook, and a dependency-free ISO-8601 converter pinned against the epoch, a leap day, and a 2023 instant. The best-effort swallow contract ("never a second failure") is not an excuse — it is documented, and the sabotaged-sink test proves it (a regular file squatting on the parent dir: the writer returns the intended path, no panic, no log). All six tests ran green in both baseline workspace runs. The consumers are real (the tauri shell's main installs the hook and writes the startup log before its stderr receipt and dialog). Defects are minor: the crash handler's `source()` walk has no hop cap (a cyclic error chain — a bug elsewhere — would hang the last line of defense instead of reporting it), the stamped-rotation variant is a public API with zero production callers, and the log's `version:` line is this crate's version, correct only while `desktop` and `lumen-desktop-tauri` versions move in lockstep (both 0.1.3 today, nothing enforces it).

## 2. What actually works (verified)
- All 6 tests green in baseline: `diagnostics::tests::{crash_log_path_is_the_deterministic_temp_sink, iso_utc_pins_known_instants, panic_hook_writes_the_log_and_still_runs_the_previous_hook, with_stamp_rotates_alongside_without_clobbering_the_default, write_crash_log_records_version_stage_and_full_chain, a_failing_sink_returns_the_intended_path_and_never_panics}` — test-workspace.log.txt:2302-2307 AND orchrun-test-workspace-nofailfast.log.txt:2261-2266 (the first run aborted later, at the desktop `zero_alloc_transport` failure, so these are not fail-fast ghosts).
- The sabotage test is a real negative test: a FILE at the parent-dir slot makes both `create_dir_all` and `write` fail; the function returns the intended path, no panic (code path traced: 105-116 swallows, returns `path.to_path_buf()`).
- The panic-hook test uses a REAL `panic!` (String payload arm), verifies the wrapped previous hook still ran (`WRAPPED_HOOK_RAN` AtomicBool), restores the hook, and asserts the log carries `stage: panic`, the payload, and the location file — mutation of the hook (e.g., dropping the `previous(info)` call) fails it.
- `iso_utc_from_unix_secs` is the canonical Hinnant `civil_from_days`; I re-derived the four pinned instants independently (0 → 1970-01-01T00:00:00Z, 86399 → 23:59:59, 951782400 → 2000-02-29, 1700000000 → 2023-11-14T22:13:20Z) — all correct.
- Consumption: apps/desktop-tauri/src/main.rs:49/53 (install hook → write startup log → stderr receipt naming the log → dialog → exit 1) — the three-face failure design the module doc claims.
- `unix_now` degrades pre-epoch clocks to (0,0) rather than erroring — matches its doc.

## 3. Findings

### F-001 [S3] — `error_chain` walks `source()` with no hop cap: a cyclic chain hangs the crash handler
Where: apps/desktop/src/diagnostics.rs:144-153
Code:
```
fn error_chain(error: &dyn Error) -> String {
    let mut chain = format!("error: {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        chain.push_str("\ncaused by: ");
        chain.push_str(&cause.to_string());
        source = cause.source();
    }
```
What is wrong: unbounded `while let` over `source()`. The std `Error` contract says chains are acyclic, but nothing enforces it — a bug in any error type (a `source()` returning an ancestor, or two errors pointing at each other) turns this into an infinite loop building an unbounded String.
Why it matters: this code IS the last line of defense — its whole contract (module doc lines 28-35) is "NEVER panics and NEVER masks the failure being reported". A hang here converts a diagnosable startup failure into a frozen process with no log and no dialog — strictly worse than the bug being reported. The panic-hook path is safe (`PanicFailure` has no `source()`), but `write_crash_log("startup", &error)` walks whatever `lumen_desktop_tauri::app::run()` returns, across the whole tauri stack.
What correct looks like: cap the walk (e.g., 64 hops — deeper chains are corruption, not diagnostics) and append `… (chain truncated)`.
Confidence: HIGH that no guard exists (read); MEDIUM that a cycle can ever occur in practice (would require an upstream bug).

### F-002 [S4] — `crash_log_path_with_stamp`: public "rotation" API with zero production callers
Where: apps/desktop/src/diagnostics.rs:76-83
Code:
```
pub fn crash_log_path_with_stamp(stamp: &str) -> PathBuf {
    let stem = CRASH_LOG_FILE
        .strip_suffix(".log")
        .unwrap_or(CRASH_LOG_FILE);
```
What is wrong: rg over the whole repo: the only callers are this file's own tests (`with_stamp_rotates_alongside_without_clobbering_the_default`, line 341/346). The doc motivates it with "a caller wanting to keep N failures can stamp" — no such caller exists; rotation is a feature nobody uses.
Why it matters: dead public surface on a shipped lib; the doc sells a capability (rotation) the product does not have.
What correct looks like: keep it test-private, or delete until a rotation consumer lands.
Confidence: HIGH — Verification: read + rg (zero external call sites).

### F-003 [S4] — The log's `version:` line is the `desktop` crate's version, correct only while two crates move in lockstep
Where: apps/desktop/src/diagnostics.rs:126-127
Code:
```
    body.push_str("version: ");
    body.push_str(env!("CARGO_PKG_VERSION"));
```
What is wrong: the artifact is named "lumen-tauri crash log" (line 125) but the version recorded is `apps/desktop`'s package version, compiled into the lib. The binary the user ran is `lumen-desktop-tauri` (apps/desktop-tauri, its own `version = "0.1.3"`). Today both are 0.1.3 (rg Cargo.toml verified), so the line is truthful; nothing enforces the lockstep, and a version bump of the shell without the lib writes a wrong version into the one artifact support triage reads first.
Why it matters: a support-facing artifact that can silently misreport the product version.
What correct looks like: the shell passes its version in (it already passes `stage`), or the tauri crate re-exports the body writer with its own `env!("CARGO_PKG_VERSION")`.
Confidence: HIGH — Verification: read + rg both Cargo.tomls (currently equal — no live defect, latent only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `const CRASH_LOG_DIR` / `CRASH_LOG_FILE` / `STAGE_PANIC` | 52-61 | REAL | — | literals carry `lumen-allow-literal` markers with rationale |
| `crash_log_path` | 66-70 | REAL | — | temp_dir/join — deterministic sink; consumed by main.rs:53 via write_crash_log + tests |
| `crash_log_path_with_stamp` | 76-83 | REAL | S4 | F-002: test-only |
| `write_crash_log` | 95-97 | REAL | — | delegates; contract documented |
| `write_crash_log_at` | 103-117 | REAL | — | the swallow is THE contract, pinned by the sabotage test — not a silent-error defect |
| `crash_log_body` | 121-139 | REAL | S4 | version line (F-003); body shape pinned by test |
| `error_chain` | 144-153 | REAL | S3 | F-001: no hop cap |
| `install_panic_hook` | 166-179 | REAL | — | wraps (take+set), restores nothing (process-global by design), `let _sink` path discard is fine (the shell prints the path on the startup path; a panic has stderr face) |
| `struct PanicFailure` + impls | 183-195 | REAL | — | no source — safe leaf |
| `payload_text` | 201-209 | REAL | — | &str/String/else — matches std payload shapes; never panics |
| `platform_label` | 212-222 | REAL | — | compile-time cfg! |
| `unix_now` | 227-232 | REAL | — | pre-epoch → (0,0) honest degradation |
| `iso_utc_from_unix_secs` | 239-260 | REAL | — | Hinnant algorithm, independently re-verified; u64 domain keeps days ≥ 0 so div_euclid/rem_euclid agree with `/` |
| tests: `DEFAULT_SINK` mutex | 278 | REAL | — | serializes real-sink tests; poison-tolerant locks at 367-369/430-432 |
| fixture types RootCause/MiddleCause/StartupFailure | 282-325 | REAL | — | three-level source() chain, real impls |
| test `crash_log_path_is_the_deterministic_temp_sink` | 327-336 | REAL | — | would fail on path-shape change |
| test `with_stamp_rotates_alongside_without_clobbering_the_default` | 338-352 | REAL | — | pins exact stamped name |
| test `iso_utc_pins_known_instants` | 354-363 | REAL | — | 4 instants incl. leap day — mutation breaks |
| test `write_crash_log_records_version_stage_and_full_chain` | 365-400 | REAL | — | writes REAL sink (serialized); round-trips the time line; asserts all 3 chain hops; `!log.is_empty()` catches a failed write (read_to_string unwrap_or_default) |
| test `a_failing_sink_returns_the_intended_path_and_never_panics` | 402-421 | REAL | — | genuine sabotage (file-as-dir); cleanup on success path only (junk in temp on mid-test panic — trivial) |
| test `panic_hook_writes_the_log_and_still_runs_the_previous_hook` | 423-457 | REAL | — | real panic + catch_unwind + wrapped-hook proof; swaps the process-global hook mid-run (benign while no parallel test panics); restores after |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| crash_log_path_is_the_deterministic_temp_sink | YES | asserts dir + file name against the constants |
| with_stamp_rotates… | YES | exact-name pin incl. two-stamp non-collision |
| iso_utc_pins_known_instants | YES | four hard instants; any arithmetic mutation breaks at least one |
| write_crash_log_records_version_stage_and_full_chain | YES | version/stage/platform/3-hop chain/how-to-report all asserted; empty-log guard catches write failure |
| a_failing_sink_returns_the_intended_path_and_never_panics | YES | real IO failure induced, not a mock: assert on returned path + non-existence |
| panic_hook_writes_the_log_and_still_runs_the_previous_hook | YES | dropping `previous(info)` fails WRAPPED_HOOK_RAN; dropping the write fails the log asserts |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "write_crash_log NEVER panics and NEVER masks … pinned by test" | module doc 28-35 | REAL | sabotage test (402-421), baseline green both runs |
| "One fixed, findable file — <OS temp dir>/lumen/lumen-tauri-crash.log — latest failure wins" | module doc 19-21 | REAL | fs::write overwrite semantics; path test |
| "panic hook WRAPS, never replaces" | doc 159-165 | REAL | take_hook/set_hook + WRAPPED_HOOK_RAN test |
| "the stamped variant exists for rotation" | doc 72-75 | SHALLOW | rotation has no consumer (F-002) — the variant exists for tests only |
| "version … the library half of the shipping pair" | doc 90-91 | REAL (latent drift) | F-003: both 0.1.3 today |
| "No chrono/time: … unit-tested against known instants including a leap day" | doc 39-41 | REAL | iso_utc_pins_known_instants (2000-02-29), green in baseline |
| "std-only, no unsafe" (crate rule) | doc 14-15 | REAL | no unsafe in file (rg); crate audit (apps/desktop/Cargo.toml.audit.md) concurs |

## 7. Dead / duplicated / unreachable code
- `crash_log_path_with_stamp` — test-only public API (F-002).
- Nothing else: every other item has the tauri-main consumer or is test infrastructure.

## 8. Dependencies & flags
Std-only (std::any/error/fmt/path/time) + `testkit` dev-dep — matches the crate's "std-only, no unsafe" rule. No feature gates. `#[allow(clippy::panic)]` at 428 is the single annotated exception, justified in-line (a hook test needs a real panic) — checked, not hiding anything.

## 9. Open questions for the next agent
- desktop-tauri wave: the three-face failure design (log → stderr receipt → dialog) is only as reachable as the tauri build — which the sandbox could not produce (baseline #2). If any wave gets a GUI build, run the failure path once (e.g., a bad config) and paste the crash-log receipt.
- A08/A10 (cli): my host.rs#part1 audit found the shipped CLI help text (help.rs:57) still describing seek as "reopen" — same vintage of stale-shell-text rot as F-002/F-003 here; whoever owns help.rs should sweep it.
