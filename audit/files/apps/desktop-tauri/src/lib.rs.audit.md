# AUDIT: apps/desktop-tauri/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W14-A10 |
| Date | 2026-11-24 |
| Lines / functions | 8 / 0 (2 `pub mod` declarations) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | Structurally YES: `lumen-tauri` bin (Cargo.toml `[[bin]]`) → main.rs:50 `lumen_desktop_tauri::app::run()` → this file's `pub mod app` → app.rs:288 `run()`. Practically NO: the crate cannot compile on ANY host (E0425 at app.rs:301), and the release lane builds only `-p lumen-cli -p desktop` (this crate is workspace-excluded with its own Cargo.lock) |
| Findings | S0: 1 (inherited) S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `sed -n` on all quoted ranges (exit 0); `rg -n 'spawn_event_pump\|spawn_session_pump'` repo-wide (exit 0, 5 hits); `rg` diagnostics/shell symbols in apps/desktop/src (exit 0); `rg -n 'frame_channel' apps/desktop-tauri/src/app.rs` (exit 0, use sites 195/215/224); no cargo (blocked: gdk-sys env limit + E0425 makes compile moot) |

## 1. One-paragraph verdict
A nine-line module root exposing exactly two modules, both genuinely consumed (`app` by the bin via main.rs:50; `frame_channel` by app.rs:195/215/224) — and that is all the file itself does. The problem is what it fronts: the crate it roots does not compile (app.rs:301 calls `spawn_session_pump`, which is defined nowhere — the ticker is `spawn_event_pump` at app.rs:190), so every symbol this file makes public is dead in every real build, and the §7.4 "anti-bypass audit" its doc advertises scans only app.rs, not the crate — it cannot catch a smuggled second command in the very module (`frame_channel`) this file exports.

## 2. What actually works (verified)
— `app::run()` is a real function (app.rs:288) and is the bin's entry (main.rs:50, sed-verified); `frame_channel` has three real call sites in app.rs (rg: lines 195, 215, 224). Both `pub mod` targets exist and are non-empty. Nothing executed: compilation of this crate is impossible here (gdk-sys, audit/runs/baseline/cargo-build-desktop-tauri.txt:310) and impossible anywhere (E0425, below).

## 3. Findings
F-001 [S0 — inherited blocker; defect lives in app.rs, first filed by W13-A10]
Where: apps/desktop-tauri/src/app.rs:301 (def: app.rs:190); poisons this file's entire public face
Code:
```
            spawn_session_pump(app.handle().clone());
```
vs the only definition (app.rs:190):
```
fn spawn_event_pump(app: AppHandle) {
```
What is wrong: `spawn_session_pump` has no definition anywhere in the repository (repo-wide rg: the only `fn spawn_*` hits are `spawn_event_pump` at app.rs:190; app.rs's `use` list, lines 84-94, has no glob or alias import that could supply it). This is E0425: the crate this file roots does not compile on any host, GTK or not. Every item `pub mod app` / `pub mod frame_channel` expose — `lumen_dispatch`, `ShellState`, `ShellError`, `EVENT_CHANNEL`, `run()`, the frame channel — is unreachable in any shipped build. Why it matters: lib.rs is the crate root; the shipped entry point `lumen-tauri` (a shipped entry point per the audit spec) resolves through it; the whole ADR-004 GUI story dies at this line. Cross-refs: audit/files/apps/desktop/src/shell.rs.audit.md F-001 (W13-A10), audit/verify/fabrications.md:123, audit/waves/wave-13/SUMMARY.md:40-41. What correct looks like: rename the call site to `spawn_event_pump` and re-run the tauri-check lane. Confidence: HIGH — Verification: read + rg (static proof; cargo build impossible: gdk-sys env limit + cargo-build-budget).

F-002 [S3]
Where: apps/desktop-tauri/tests/single_command.rs:22-23 + 70-78; claim sold by this file's doc (lib.rs:5)
Code (single_command.rs:20-23, sed-verified):
```
/// The registration source with comment lines stripped (`//`, `//!`,
/// `///`): the code the compiler sees.
fn registration_source() -> String {
    let source = include_str!("../src/app.rs");
```
Code (single_command.rs:70-71):
```
#[test]
fn the_registration_site_is_unique_in_the_crate() {
```
What is wrong: the test's NAME asserts uniqueness "in the crate", but `registration_source()` reads ONLY `../src/app.rs`. A second `generate_handler!` in `src/frame_channel.rs` — a module this very lib.rs exports — would not be seen by either test. Mutation reasoning (no edit performed): add a `generate_handler![smuggled]` registration to frame_channel.rs → both tests still pass → the §7.4 rule "fails if any command other than `lumen_dispatch` is registered" is enforced for one file, not the crate. lib.rs:5 advertises this as "the §7.4 anti-bypass audit", inheriting the overstatement. Why it matters: an anti-bypass test exists precisely to catch adversarial drift; its scope is narrower than its name. What correct looks like: scan every `src/**/*.rs` of the crate (or at minimum lib.rs's two module files). Confidence: HIGH — Verification: read + mutation reasoning (UNVERIFIED by execution: the test binary cannot even compile until F-001 is fixed). Blast radius limited: a smuggler must still wire a Builder, and the only one lives in app.rs.

F-003 [S4]
Where: apps/desktop-tauri/src/lib.rs:8
Code:
```
pub mod frame_channel;
```
What is wrong: `frame_channel` has no consumer outside the crate (repo-wide rg: use sites are app.rs:195/215/224 via `crate::frame_channel`, which resolves identically for a private module). The `pub` over-exposes the module to any future external consumer of the lib face, widening the surface the F-002 test does not cover. What correct looks like: `mod frame_channel;` (or `pub(crate)`). Confidence: HIGH — Verification: read + rg. Severity floor: hygiene.

## 4. Per-function table (file < 100 LOC — module inventory)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `pub mod app` | 7 | REAL-UNPROVEN | S0 (inherited) | Contents real (run/lumen_dispatch/pump) but crate broken by E0425 (F-001) |
| `pub mod frame_channel` | 8 | REAL-UNPROVEN | S4 | Consumed only inside the crate; over-exposed (F-003) |
| crate doc | 1-5 | Accurate except §7.4 scope | S3 | "app::run() is the whole entry" — TRUE (main.rs:50); anti-bypass audit scope overstated (F-002) |

## 5. Test honesty (tests/single_command.rs, referenced by this file's doc)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `the_shell_registers_exactly_one_command_and_it_is_lumen_dispatch` (single_command.rs:39) | YES, for app.rs | Registers 2 commands in app.rs's `generate_handler![…]` → `assert_eq!(registered, vec!["lumen_dispatch"])` fails. Honest for the one file it scans. |
| `the_registration_site_is_unique_in_the_crate` (single_command.rs:71) | NO, for the crate | Scans only app.rs; a site in frame_channel.rs/main.rs-adjacent modules evades it (F-002). Name promises crate-wide. |
| Both | CANNOT RUN | `cargo test --manifest-path apps/desktop-tauri/Cargo.toml` (ci.yml:731) must compile the lib first — E0425 (F-001) makes the lane red against the current tree. UNVERIFIED by execution. |

## 6. Doc claims touching this file — the master prompt §8.14 features
| Claim (§8.14 features) | Where does it exist? | Verdict | Proof (path:line, all sed-verified) |
|---|---|---|---|
| `lumen_dispatch` | HERE — apps/desktop-tauri/src/app.rs:144-154 (`#[tauri::command] async fn lumen_dispatch(...)`: locks, `runtime.dispatch(&request)`, `forward_pending`) | REAL-UNPROVEN | Real body, but crate cannot compile (F-001, app.rs:301); never executed in sandbox or CI against current tree |
| Shell runtime | apps/desktop — `pub struct ShellRuntime` at apps/desktop/src/shell.rs:101; constructed in this crate at app.rs:290-292 (`ShellRuntime::new(DEFAULT_EVENT_CAPACITY)`) | REAL-UNPROVEN for the GUI path | desktop crate compiles in the release lane, but its only GUI composition is this broken crate; per W13-A10 (shell.rs.audit.md) no shipped binary besides the broken wrapper touches it |
| Pacer (GUI ticker) | HERE — `spawn_event_pump` app.rs:190-232, cadence `SESSION_TICK = Duration::from_millis(10)` (app.rs:109) | REAL-UNPROVEN | Ticker is real code (sleep → lock → `pump_session` → `forward_pending` → frame ship) but dead: never compiled (F-001). NOTE: the GUI cadence is 10 ms, NOT the apps/desktop `PACER_TICK` (30.625 ms, shell.rs:80-81) that docs sell as the GUI cadence (W13-A10 finding) |
| Crash handler | apps/desktop — `write_crash_log` diagnostics.rs:95, `install_panic_hook` diagnostics.rs:166; wired in THIS crate's bin (main.rs:49 `desktop::diagnostics::install_panic_hook();`, main.rs:53 `write_crash_log("startup", &error)`) | REAL-UNPROVEN for the tauri app; code ships compiled via the `desktop` crate (release lane builds it) | Not in lib.rs at all; its only invocation site in this crate sits in the compile-broken bin |
| "`app::run()` is the whole entry" (lib.rs:3) | TRUE | REAL | main.rs:50 `lumen_desktop_tauri::app::run()` |
| "tests/single_command.rs for the §7.4 anti-bypass audit" (lib.rs:4-5) | Overstated | SHALLOW | Test scans only app.rs (F-002); and cannot run until F-001 is fixed |

§8.14 bottom line: all four features have genuine code, split across two crates (lumen_dispatch + GUI 10 ms pacer in apps/desktop-tauri; ShellRuntime + PACER_TICK + crash handler in apps/desktop), but the GUI half — the only composition that ties them into a shipped `lumen-tauri` binary — is compile-broken (E0425), so lumen_dispatch and the GUI pacer have never run in any build; the crash handler and ShellRuntime compile in the release lane via `desktop` but are reached by no shipped binary other than the broken wrapper.

## 7. Dead / duplicated / unreachable code
Everything this file exposes is transitively dead in shipped builds: (a) E0425 breaks compilation (F-001); (b) the release lane never builds this crate (workspace-excluded, own Cargo.lock — Cargo.toml:6-16 rationale). `pub mod frame_channel` is over-exposed (F-003). No duplication.

## 8. Dependencies & flags
lib.rs declares no imports. The crate's deps (Cargo.toml): `desktop` (path), `commands` (path), `tauri = "2"`, `serde`; build-dep `tauri-build = "2"`. Inline lints mirror the workspace posture (`unsafe_code = "deny"`, `unwrap_used`/`expect_used`/`panic`/`todo`/`unimplemented` denied, Cargo.toml:35-48) — note `#[allow(unsafe_code)]` in main.rs:118 can override the Cargo-level deny (only `forbid` is allow-proof); that is main.rs's audit territory.

## 9. Open questions for the next agent
- W14 (main.rs / app.rs / frame_channel.rs auditor): confirm `frame_channel.rs` contains no `generate_handler!` today (F-002 is a scope hole, not a present violation).
- CI auditor: when did tauri-check (ci.yml:680) last go green? If after the `spawn_event_pump` rename, INFRA-BLOCKED.md's "the crate compiles in its dedicated CI lane" is FALSE against current HEAD (W13-A10 already flagged this).
- Did any doc claim the GUI ticker runs at PACER_TICK (30.625 ms)? apps/desktop/src/shell.rs:16 does — the live GUI value is SESSION_TICK 10 ms (app.rs:109).
