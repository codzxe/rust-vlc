# AUDIT: crates/testkit/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W16-A05 |
| Date | 2026-11-24 |
| Lines / functions | 504 / 24 public items (trait Must + 2 impls, 3 assert fns, oracle trio, 4 fixture fns, VirtualClock + StepClock, HardwareClass, EvidenceRecord trio) + 6 unit tests |
| File verdict | REAL |
| Reachable from shipped entry point? | NO from release binaries (dev-only crate; audit/runs/baseline/full_gate.txt:48 `testkit-direction... PASS — dev-only crates [testkit] appear in [dev-dependencies] tables only`; 22 crates list it in Cargo.toml). YES from the test lane — every cargo test run compiles it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | full read; rg module files + consumers; baseline citation (cargo not installed in sandbox — reason: no-cargo) |

## 1. One-paragraph verdict
A real, working test kit: lint-legal `Must` unwrap-with-context via `unreachable!`, tolerance-based A/V-sync assertion primitive, an honest oracle-presence probe with the `LUMEN_REQUIRE_ORACLE=1` strictness knob, cargo-style workspace-root fixture resolution with typed path-carrying errors, a monotonic `StepClock`, and an anti-falsification evidence writer (divergent rewrite of an existing evidence file is an error, identical rewrite a no-op). Its own 6 unit tests exercise failure paths via `catch_unwind` — they fail if the helpers regress. Two hygiene defects: a stale in-code comment claiming `tests/fixtures/` does not exist (it does), and an `is_file()`-based PATH probe that treats a non-executable `ffmpeg` file as an oracle.

## 2. What actually works (verified)
— Baseline execution: audit/runs/baseline/cargo-test-workspace.txt:6225-6230 — all six lib.rs unit tests `ok`; :6232 `test result: ok. 26 passed; 0 failed` for the testkit unittests target (26 = 6 here + 20 in alloc_count.rs/faults.rs siblings).
— Structural: `pub mod alloc_count; pub mod faults;` (lib.rs:31-32) resolve to real files (crates/testkit/src/alloc_count.rs 328 lines, faults.rs 1261 lines — verified via ls/wc).

## 3. Findings
F-001 [S4]
Where: crates/testkit/src/lib.rs:444-445
Code:
```
        // tests/fixtures/ does not exist yet (corpus lands with doc 04's
        // plan) — the error must carry the resolved path.
```
What is wrong: the comment is false — `tests/fixtures/` exists and is heavily populated (ls: CORPUS.md, expectations/, generated/, hls/, mjpeg_h3/). The test still passes (it asks for `area/nothing.bin`), but the comment asserts a repo state that ended months of corpus work ago. Why it matters: stale in-code claims mislead the next reader about fixture layout and the corpus plan's status. What correct looks like: update to "a nonexistent fixture under tests/fixtures/ must fail with the resolved path". Confidence: HIGH — Verification: read + ls tests/fixtures.

F-002 [S4]
Where: crates/testkit/src/lib.rs:170-173
Code:
```
#[cfg(not(target_family = "windows"))]
fn path_has(dir: &Path, program: &str) -> bool {
    dir.join(program).is_file()
}
```
What is wrong: presence check is `is_file()`, not executable-bit (comment honestly says "executable-looking"). A non-executable file named `ffmpeg` on PATH makes `oracle_available()` return true, and oracle-differential tests then spawn it and fail with a spawn error instead of skipping. Why it matters: the W25 CI-failure mode this helper exists to prevent (panic on missing oracle) can reappear in a different costume. Blast radius is small (dev machines with broken PATH entries). What correct looks like: `Metadata::permissions()` + mode check on unix. Confidence: HIGH — Verification: read; spawn-failure consequence UNVERIFIED (not executed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Must for Result | 53-63 | REAL | — | unreachable! carries err text; #[track_caller] |
| Must for Option | 65-75 | REAL | — | |
| assert_contains | 78-84 | REAL | — | full haystack in failure msg |
| assert_nanos_near | 88-95 | REAL | — | abs_diff tolerance; A/V-sync primitive |
| assert_err | 99-105 | REAL | — | returns E for kind assertions |
| oracle_available | 124-134 | REAL | — | LUMEN_REQUIRE_ORACLE=1 strictness; assert on absence |
| skip_no_oracle | 145-151 | REAL | — | eprintln skip notice |
| on_path | 153-158 | REAL | — | split_paths scan |
| path_has (win/unix) | 163-173 | REAL (is_file only) | S4 | F-002 |
| FixtureError + Display | 181-201 | REAL | — | path-carrying errors |
| workspace_root_from | 205-219 | REAL | — | walks ancestors for `[workspace]` manifest |
| workspace_root | 222-227 | REAL | — | current_dir anchor |
| fixture_path | 235-242 | REAL | — | existence-checked here, not 3 layers later |
| fixture_bytes | 245-248 | REAL | — | |
| VirtualClock trait | 261-268 | REAL | — | shape pinned for Phase-2 |
| StepClock + impl | 273-293 | REAL | — | saturating_add monotonic |
| HardwareClass/as_str | 304-320 | REAL | — | mock/real label per doc 10 §7.1 |
| EvidenceRecord::new/claim | 346-363 | REAL | — | builder |
| EvidenceRecord::render | 367-378 | REAL | — | stable text form |
| EvidenceRecord::write_to | 384-398 | REAL | — | anti-falsify: divergent rewrite errors |
| tests mod (6 tests) | 401-503 | REAL | — | failure paths via catch_unwind; temp dirs cleaned |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| must_unwraps_results_and_options_with_context | YES | Ok/Some values asserted AND Err path caught panicking |
| assert_helpers_fail_with_context | YES | positive + catch_unwind negative paths for all 3 helpers |
| fixture_path_resolves_and_reports_missing | YES | asserts the exact Missing variant + resolved path suffix |
| step_clock_is_monotonic | YES | exact ns arithmetic |
| evidence_record_renders_and_writes | YES | render content, idempotent rewrite ok, divergent rewrite errors |
| oracle_path_scan_finds_programs_only_where_they_exist | YES | present/absent/empty-dir matrix |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no production crate ever depends on it — [dev-dependencies] entries only" | lib.rs:3-5 | REAL | baseline full_gate.txt:48 testkit-direction PASS (dev-dep tables only) |
| "tests/fixtures/ does not exist yet" | lib.rs:444 | FALSE | tests/fixtures/ exists (ls: CORPUS.md, generated/, expectations/, hls/) — F-001 |
| "Std-only: zero dependencies of any kind" | crates/testkit/Cargo.toml:11 | REAL | manifest has no [dependencies] section (read; 26 lines) |

## 7. Dead / duplicated / unreachable code
None in this file. alloc_count/faults are audited as their own files if assigned (they are not in my list — flag to orchestrator: crates/testkit/src/alloc_count.rs and faults.rs belong to another agent's assignment or none).

## 8. Dependencies & flags
Std-only, zero external deps (Cargo.toml has no dependencies table). `lumen-allow-literal` markers annotate env-var names in code — consistent with the repo's literal-lint convention.

## 9. Open questions for the next agent
- Are the 51 `#[ignore]`d tests baseline-wide ever run in a nightly lane? (Not this file's problem — no ignored tests here.)
