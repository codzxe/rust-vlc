# AUDIT: apps/desktop/src/diagnostics.rs
| Field | Value |
|---|---|
| Auditor | W13-A04 |
| Date | 2026-11-24 |
| Lines / functions | 458 (incl. ~193 test lines) / 12 non-test fns + 3 fixture types + 6 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES via lumen-tauri: apps/desktop-tauri/src/main.rs:49 `desktop::diagnostics::install_panic_hook();` and :53 `let log = desktop::diagnostics::write_crash_log("startup", &error);` (bin `lumen-tauri`, Cargo.toml:31). NOT wired in the `-p desktop` lane's own binary: apps/desktop/src/main.rs (lumen-desktop skeleton) never calls it — the hook/log exist only in the tauri shell. Runtime UNVERIFIED (desktop-tauri fails at gdk-sys in this sandbox — baseline environment limit); unit level VERIFIED (tests run). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | ./target/debug/deps/desktop-c2864a891fe6f82b diagnostics → 6 passed / 0 failed, 0.01s. |

## 1. One-paragraph verdict
This is the rare file in the tree that does exactly what its docs claim, with tests that would fail if it broke: a std-only crash-log sink with a swallowed-IO best-effort contract, a wrapping panic hook, and a dependency-free UTC calendar (Hinnant's civil-from-days) pinned against the epoch, an end-of-day, a leap day and a 2023 instant — all 6 tests pass, including a REAL `panic!` driven through `catch_unwind` proving the hook writes the sink AND still runs the wrapped hook, and a sabotaged sink (a regular file squatting on the parent dir) proving the never-panic/return-intended-path contract. The only debt is cosmetic (empty swallowed-error blocks, a test that writes the developer's real temp sink, and a hook-restore ordering hazard inside one test). The doc's cross-claims check out: the Windows MessageBox half exists (desktop-tauri/src/main.rs:127 `MessageBoxW`, `fatal_dialog` :110).

## 2. What actually works (verified)
— `./target/debug/deps/desktop-c2864a891fe6f82b diagnostics` → 6 passed: deterministic sink path, stamped rotation, ISO pins (`1970-01-01T00:00:00Z`, `2000-02-29T00:00:00Z`, `2023-11-14T22:13:20Z`), full `source()` chain in the log (`caused by:` ×2), version/platform/stage lines, round-trip time line, real-panic hook test, sabotaged-sink contract.

## 3. Findings
### F-001 [S4] Swallowed-error bodies are empty `if` blocks
Where: apps/desktop/src/diagnostics.rs:109-115
Code (sed-verified):
```rust
        if std::fs::create_dir_all(dir).is_err() {
            // fall through
        }
    }
    if std::fs::write(path, body).is_err() {
        // swallowed by the same contract — see the module docs
    }
```
What is wrong: style only — the contract (never panic, never mask) is documented (:28-35) and pinned by `a_failing_sink_returns_the_intended_path_and_never_panics` (ran, passed). `let _ = …` or a named helper would read better. Confidence: HIGH — Verification: read + ran.

### F-002 [S4] `write_crash_log_records_version_stage_and_full_chain` writes the developer's REAL default sink
Where: apps/desktop/src/diagnostics.rs:373-376
Code (sed-verified):
```rust
        let path = write_crash_log("startup", &failure);
        assert_eq!(path, crash_log_path());
        let log = fs::read_to_string(&path).unwrap_or_default();
```
What is wrong: the test clobbers `<tmp>/lumen/lumen-tauri-crash.log` on every run (serialized by `DEFAULT_SINK`, :278, so no race — but the machine's real diagnostic artifact is destroyed by a test run; a user-support scenario running tests after a real crash erases the evidence the module exists to preserve). The stamped variant exists for exactly this. Confidence: HIGH — Verification: read + ran (the run wrote the sink).

### F-003 [S4] Hook-restore ordering hazard in the panic-hook test
Where: apps/desktop/src/diagnostics.rs:446-447
Code (sed-verified):
```rust
        assert!(caught.is_err(), "the probe must actually panic");
        std::panic::set_hook(previous);
```
What is wrong: if the `assert!` (or any earlier assert in this test) fires, `set_hook(previous)` never runs and the process-wide hook stays the wrapped probe for the remaining 59 tests in the binary. Restore should precede asserts. Cosmetic blast radius (test-binary only). Confidence: HIGH — Verification: read (mutation reasoning; not executed — no failure to trigger).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crash_log_path | 66-70 | REAL | — | deterministic sink; test ran |
| crash_log_path_with_stamp | 76-83 | REAL | — | rotation shape pinned (`lumen-tauri-crash-w30.log`); test ran |
| write_crash_log | 95-97 | REAL | — | composition of path + chain; test ran |
| write_crash_log_at | 103-117 | REAL | S4 | best-effort contract honored; empty-if style (F-001) |
| crash_log_body | 121-139 | REAL | — | version/platform/stage/time/error/report; every line asserted by test |
| error_chain | 144-153 | REAL | — | hand-walked `source()` chain; both hops asserted |
| install_panic_hook | 166-179 | REAL | — | wraps, never replaces; proven by the real-panic test |
| PanicFailure (Display/Error) | 183-195 | REAL | — | reuses the chain writer |
| payload_text | 201-209 | REAL | — | &str/String/other arms; String arm exercised by test |
| platform_label | 212-222 | REAL | — | compile-time labels |
| unix_now | 227-232 | REAL | — | pre-epoch degrades to (0,0), never fails |
| iso_utc_from_unix_secs | 239-260 | REAL | — | Hinnant algorithm; 4 pinned instants + round-trip, all passed |
| tests (6) | 327-457 | REAL | — | all ran, all passed; F-002/F-003 hygiene notes |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| crash_log_path_is_the_deterministic_temp_sink | YES | asserts dir + exact file name |
| with_stamp_rotates_alongside_without_clobbering_the_default | YES | parent equality, distinctness, exact stamped name |
| iso_utc_pins_known_instants | YES | 4 exact strings incl. leap day |
| write_crash_log_records_version_stage_and_full_chain | YES | reads the written file back; version/stage/chain hops/report line + round-trip time |
| a_failing_sink_returns_the_intended_path_and_never_panics | YES | deterministic sabotage (file squatting on dir path) |
| panic_hook_writes_the_log_and_still_runs_the_previous_hook | YES | real `panic!` via catch_unwind; asserts wrapped-hook flag + sink contents (`stage: panic`, payload, `diagnostics.rs` location) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "write_crash_log NEVER panics and NEVER masks … the INTENDED path is returned regardless" | :30-35 | REAL | sabotaged-sink test ran, passed |
| "The Windows-native half (the MessageBox …) lives in the excluded crate's main.rs" | :12-14 | REAL | desktop-tauri/src/main.rs:127 MessageBoxW, :110 fatal_dialog |
| "the UTC timestamp is unix seconds → ISO 8601 by the civil-from-days integer algorithm … unit-tested against known instants including a leap day" | :39-41 | REAL | 4 pinned instants, test ran |
| "calling it twice chains hooks — the shell's main calls it exactly once" | :155-157 | REAL-UNPROVEN | lumen-tauri main.rs:49 single call read; tauri runtime not executable here (gdk-sys) |
| "on shared Unix hosts the fixed path can collide across users — accepted" | :23-26 | REAL (honest limitation) | read; `temp_dir()` + fixed name |

## 7. Dead / duplicated / unreachable code
None. Nuance: within the `-p desktop` release-lane binary (lumen-desktop skeleton) this module is library-only (main.rs never calls it); its shipped consumer is lumen-tauri (desktop-tauri crate).

## 8. Dependencies & flags
std-only (no chrono/time/unsafe) — matches the C5 boundary claim. `testkit::{Must, assert_contains}` test-only. No feature gates.

## 9. Open questions for the next agent
- desktop-tauri owner: is the crash-log receipt line ("lumen-tauri: crash log: …") covered by the release smokes that grep "admission shell failed" (main.rs:55-58 comment)? Unverifiable here (build fails at gdk-sys).
