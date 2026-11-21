# AUDIT: apps/desktop-tauri/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W14-A05 |
| Date | 2026-11-21 |
| Lines / functions | 8 / 0 functions (1 crate doc comment, 2 `pub mod` declarations) |
| File verdict | REAL-UNPROVEN (the two declarations are correct and resolve, and the entry chain is fully traced — but the crate they root cannot compile: app.rs:301 references a function that does not exist, so nothing here has ever been, or can currently be, built or executed; standing UNVERIFIED-BUILD, baseline #2) |
| Reachable from shipped entry point? | YES by design, NO in practice. Chain: shipped bin `lumen-tauri` (Cargo.toml:30-32) → main.rs:50 `lumen_desktop_tauri::app::run()` → **this file** line 7 `pub mod app;` → app.rs:288 `run()`. `frame_channel` (line 8): app.rs:195/215/224 via `crate::frame_channel::*`. The chain is severed at compile time by F-001 (E0425) — no binary can be produced from this tree. |
| Findings | S0: 1 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg 'spawn_session_pump\|spawn_event_pump'` repo-wide → def app.rs:190, call app.rs:301, doc app.rs:42, NOTHING else — **no definition of the called name exists** (E0425 proof); `rg 'lumen_desktop_tauri'` repo-wide → main.rs:50 only (sole external consumer of this lib); `rg 'frame_channel::\|frame_channel(\|send_record\|FrameChannel'` excluding apps/desktop-tauri → 0 hits (no external consumer of the `frame_channel` module); `sed -n '1,8p'` lib.rs, `'188,192p;294,304p'` app.rs, `'50p'` main.rs (quote verify); Grep root Cargo.toml → `exclude = ["apps/desktop-tauri"]` at :51 (why the clean workspace build never sees this crate); `sed -n '680,745p'` .github/workflows/ci.yml → tauri-check lane, `cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets` at :730-731 (the lane that WOULD catch E0425); verified all sibling imports resolve (shell.rs:57/:101, files.rs:100, window_config.rs:191, diagnostics.rs:95/:166); baseline #2 (build-desktop-tauri.log.txt, exit 101) |

## 1. One-paragraph verdict
This file is 8 lines of pure module wiring and every declaration in it is individually
correct: `app` and `frame_channel` both exist as sibling files, `pub mod app;` is
required public (the `lumen-tauri` bin consumes `lumen_desktop_tauri::app::run()` at
main.rs:50), and the doc comment's structural claims all check out. But the module it
declares at line 7 cannot compile: app.rs:301 calls `spawn_session_pump`, a function
with no definition anywhere in the repository (the only ticker definition is
`spawn_event_pump`, app.rs:190). That single mismatch (E0425) makes this library
unbuildable, the shipped GUI bin unlinkable, the documented "event pump" session
pacer dead code, and the §7.4 anti-bypass test (referenced by this file's own docs)
un-runnable — while INFRA-BLOCKED.md:5 claims "the crate compiles in its dedicated CI
lane". The break is invisible to every green gate: the crate is workspace-excluded
(root Cargo.toml:51) and the sandbox build died at gdk-3.0 pkg-config before
type-checking (baseline #2). As a file, REAL; as shipped code, none of it exists.

## 2. What actually works (verified)
— Both declared modules resolve to real sibling files: `src/app.rs` (322 lines) and
`src/frame_channel.rs` (57 lines); `src/` holds no orphan module files (lib.rs,
main.rs, app.rs, frame_channel.rs — all wired). Verified by LS + read.
— The crate doc's claim "`app::run()` is the whole entry" is accurate: main.rs:50
`if let Err(error) = lumen_desktop_tauri::app::run() {` is the bin's only use of the
lib (rg repo-wide → 1 hit), and `app::run` exists at app.rs:288.
— `pub mod app;` visibility is required, not overexposed: the bin links the lib as an
external crate, so `app` must be `pub` for main.rs:50 to compile. (Contrast F-002 for
`frame_channel`.)
— The `desktop`/`commands` symbols app.rs/main.rs import through this lib all resolve
(ShellRuntime shell.rs:101, DEFAULT_EVENT_CAPACITY shell.rs:57, PathDragDrop
files.rs:100, parse_window_table window_config.rs:191, diagnostics.rs:95/:166) — the
crate's compile failure is NOT a missing dependency symbol.

## 3. Findings

### F-001 [S0] — cross-file compile blocker owned by this file's module tree (primary finding recorded by W14-A01 on app.rs; repeated here because it determines this file's reachability and verdict)
Where: apps/desktop-tauri/src/app.rs:301 (the call this file's `pub mod app;` at line 7 roots)
Code:
```
            apply_window_spec(app.handle())?;
            spawn_session_pump(app.handle().clone());
            open_argv_files(app.handle());
```
vs the only definition in the entire repository (app.rs:190):
```
fn spawn_event_pump(app: AppHandle) {
```
What is wrong: `spawn_session_pump` has no definition anywhere. `rg 'spawn_session_pump'`
over the whole repo returns exactly one hit — the call site app.rs:301. app.rs has no
glob `use` (its imports are enumerated: tauri::{…}, commands::{…}, desktop::{…}), no
macro in scope can generate it, and the std prelude has no such name → hard compile
error E0425 (`cannot find function spawn_session_pump in this scope`). The crate —
and therefore this file's entire library face — cannot compile.
Why it matters: (1) **No shipped GUI can exist from this tree**: the bin `lumen-tauri`
(Cargo.toml:30-32) calls `lumen_desktop_tauri::app::run()` (main.rs:50) through this
lib; if the lib does not build, no `lumen-tauri` binary builds, and every release
payload's GUI launcher (INFRA-BLOCKED.md §B.1: AppImage `AppRun` and macOS
`CFBundleExecutable` exec `lumen-tauri`) has nothing to exec. (2) **The documented
session pacer is dead code**: app.rs:38-49's own module doc states the engine is
synchronous single-writer and "between two invokes NOTHING pumps" — `spawn_event_pump`
IS the session thread; it is never called (the call site names a different function),
so even a "fix" that deleted the call instead of renaming it would produce a GUI that
opens a file and then never advances playback — precisely the Wave-14 hostile question
(a GUI that hangs on open). (3) **A shipped-facing claim is FALSE**: INFRA-BLOCKED.md:5
asserts "the crate compiles in its dedicated CI lane (`tauri-check` …)" — the lane's
`cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets`
(ci.yml:730-731) type-checks this lib and would fail with E0425 on the current tree;
either the claim predates a rename (the fn is documented as `spawn_event_pump` at
app.rs:42 too) or the lane has not run since. (4) **The §7.4 anti-bypass audit this
file's doc advertises cannot run**: integration tests build the lib they test;
`tests/single_command.rs` is unreachable behind the failed build. (5) **Every green
gate is blind to it**: the crate is excluded from the workspace (root Cargo.toml:51
`exclude = ["apps/desktop-tauri"]`) so baseline #1 (workspace build, exit 0) and #5
(clippy, clean) never touch it, and baseline #2 (this crate's build) died at
gdk-3.0 pkg-config (build-desktop-tauri.log.txt, exit 101) before compiling a single
Rust line — the standing UNVERIFIED-BUILD masked a statically provable certainty.
What correct looks like: rename the call site to `spawn_event_pump` (one line,
app.rs:301), then re-run the tauri-check lane; and add a compile-canary to the repo's
own gates (e.g., a `cargo check` step on a webkit-equipped runner — which ci.yml
already is — listed as blocking in release lanes) so a rename can never again strand
the crate's only entry.
Confidence: HIGH — Verification: read (rg proof of no definition; no glob imports;
sed-verified both quotes); the compile itself is UNVERIFIED in this sandbox (webkit
dev libs absent, baseline #2) — REQUEST-ORCH-RUN below would turn the static proof
into a compiler receipt.

### F-002 [S4]
Where: apps/desktop-tauri/src/lib.rs:8
Code:
```
pub mod frame_channel;
```
What is wrong: `frame_channel` is exported from the library's public API but has zero
external consumers: `rg 'frame_channel::|frame_channel\(|send_record|FrameChannel'`
outside `apps/desktop-tauri/**` returns 0 hits; the only uses are in-crate
(app.rs:195 `crate::frame_channel::FrameChannel`, :215
`crate::frame_channel::frame_channel(&app, id)`, :224
`crate::frame_channel::send_record(channel, record)`). A crate-private
`mod frame_channel;` (with `pub(crate)` items in frame_channel.rs) is the minimal
correct surface. The crate is `publish = false` with exactly one bin consumer
(main.rs) that never names `frame_channel`.
Why it matters: public API is contract; anything exported must be maintained and can
be depended on. Here the export additionally forces the awkward
`crate::frame_channel::frame_channel` path (module and function share a name — legal,
but the export makes it part of a promised surface instead of an internal quirk).
What correct looks like: `mod frame_channel;` in lib.rs; the items inside stay
`pub(crate)` (that change belongs to frame_channel.rs — W14-A03's file; this line is
where the overexposure is granted).
Confidence: HIGH — Verification: read + rg (0 external consumers).

## 4. Per-function table
This file contains no functions/methods/impls/macros/tests/config keys — only module
declarations. Every item in the file gets its row:

| Item | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| crate doc comment | 1-5 | REAL (all 4 structural claims verified — see §6) | — | Names the crate, the entry, the sibling contract and the test file; all exist. Silent on the one thing that matters: nothing here compiles today. |
| `pub mod app;` | 7 | REAL-UNPROVEN | S0 (via F-001) | Roots app.rs, which fails E0425 at :301. `pub` is REQUIRED (bin consumes `lumen_desktop_tauri::app::run`, main.rs:50 — sole external use). Reachable by design; unbuildable in fact. |
| `pub mod frame_channel;` | 8 | REAL-UNPROVEN | S4 (F-002) | Roots frame_channel.rs; consumed only in-crate (app.rs:195/215/224). `pub` unnecessary — overexposure. |

## 5. Test honesty (tests in/for this file)
No tests in lib.rs. The crate's only test file, `tests/single_command.rs`, is the
§7.4 anti-bypass audit this file's doc comment points at:

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `the_shell_registers_exactly_one_command_and_it_is_lumen_dispatch` (single_command.rs:39-65) | YES, for its narrow rule | Mutation check (reasoned, not run): add a second command to `generate_handler![…]` in app.rs → the parsed `registered` vec becomes `["lumen_dispatch", "foo"]` → `assert_eq!` fails. Remove `generate_handler!` entirely → `unreachable!` fires. Honest for the REGISTRATION rule it claims. |
| `the_registration_site_is_unique_in_the_crate` (single_command.rs:71-78) | YES, for its narrow rule | A second `generate_handler!` site in app.rs code → count 2 → `assert_eq!` fails. A string literal containing the token would false-positive (over-strict, not under-strict) — acceptable. |
| BOTH, today | MOOT — cannot run | Integration tests build the lib they link; with F-001's E0425 the test target never builds. The §7.4 guarantee is currently enforced by TEXT-scanning a tree that cannot compile — the audit passes on paper while the audited crate is unshippable. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`app::run()` is the whole entry" | lib.rs:3 | REAL | main.rs:50 calls `lumen_desktop_tauri::app::run()`; rg → sole external consumer; app.rs:288 defines it. |
| app.rs holds the wrapper's contract: "single command, event pump, window spec, file-open seams" | lib.rs:3-4 | REAL as locations / DEAD as behavior for "event pump" | single command: app.rs:293 `generate_handler![lumen_dispatch]` (verified by tests/single_command.rs scan logic); window spec: app.rs:237 `apply_window_spec`; file-open seams: app.rs:271 `open_argv_files` + :305 drag-drop handler; event pump: app.rs:190 `spawn_event_pump` exists but is NEVER CALLED (F-001) — the documented pump is dead code. |
| "`tests/single_command.rs` for the §7.4 anti-bypass audit" | lib.rs:5 | REAL-UNPROVEN | File exists (79 lines, 2 tests); currently un-runnable behind E0425 (§5). |
| "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" | INFRA-BLOCKED.md:5 | FALSE | Static proof of E0425 (no `spawn_session_pump` definition repo-wide); ci.yml:730-731 `cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets` would fail on this tree. Claim predates or ignores the break. |
| (context) "its dedicated CI lanes compile it on webview-equipped hosts" | root Cargo.toml:47-48 | UNVERIFIED | No network access to CI run history; the lane exists (ci.yml:680+), its current status unknown; the current source cannot pass it. |

## 7. Dead / duplicated / unreachable code
- `spawn_event_pump` (app.rs:190-232) — dead code in the current tree: defined,
  documented (app.rs:42), never called (call site :301 names a non-existent
  `spawn_session_pump`). It contains the entire frame-shipping logic (w31.1) and the
  only thing that drives playback between invokes.
- lib.rs itself: no dead items; no orphan source files in `src/` (main.rs is the
  declared bin, app.rs + frame_channel.rs are the declared modules).
- No duplication: the bin does NOT re-declare `mod app;` (main.rs links the lib
  instead — verified: main.rs has no `mod` items), so the classic Tauri
  double-compilation/static-duplication trap is absent.

## 8. Dependencies & flags
- Lib target is implicit: no `[lib]` section in Cargo.toml → lib name
  `lumen_desktop_tauri` from package `lumen-desktop-tauri` — matches main.rs:50's path.
  Verified consistent.
- Crate-level lint posture (Cargo.toml:34-47 `[lints.rust]/[lints.clippy]` deny set)
  applies to this file with no attributes needed; lib.rs carries no `#![allow]`s.
- Dependencies the rooted modules need all resolve (§2); the E0425 is not a
  dependency-resolution failure.
- Workspace exclusion (root Cargo.toml:51) verified — this crate is invisible to
  `--workspace` build/test/clippy (baseline #1/#3/#5), which is exactly why F-001
  survives a clean workspace gate.

## 9. Open questions for the next agent
- W14-A01 (app.rs): owns the F-001 fix locus — rename call site (:301) to
  `spawn_event_pump` vs renaming the fn; and WHEN the tree broke (doc app.rs:42
  already says `spawn_event_pump`, so the call site is the stale side; dating needs
  git history — blocked by SESSION-2 no-git; orchestrator may know).
- W14-A02 (main.rs): the three-face failure story depends on `app::run()` returning
  `Err` — with the crate unbuildable, none of faces 1-3 has ever executed; note the
  cross-reference.
- W14-A03 (frame_channel.rs): items should drop to `pub(crate)` if F-002 is accepted;
  module/function same-name (`frame_channel::frame_channel`) is an S4 there.
- Wave prompt's "GUI mounts no <canvas> / no video" Wave-0 evidence: that evidence
  describes a GUI that RAN — impossible from this tree (F-001). Either the evidence
  predates the break or the binary tested was built from an older revision;
  orchestrator should date the divergence (REQUEST-ORCH-RUN below is the compile
  receipt; a CI history check is the dating half).
- REQUEST-ORCH-RUN: `cargo check --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets` (on a webkit2gtk-equipped host, i.e. what ci.yml:689-701 installs) — expected output: `error[E0425]: cannot find function spawn_session_pump in this file` at apps/desktop-tauri/src/app.rs:301, converting F-001's static proof into a compiler receipt.
