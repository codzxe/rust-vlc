# AUDIT: apps/desktop-tauri/tests/single_command.rs
| Field | Value |
|---|---|
| Auditor | W14-A05 |
| Date | 2026-11-23 |
| Lines / functions | 78 / 3 (1 helper + 2 `#[test]`) |
| File verdict | SHALLOW (sound in-file scan logic, verified by replication — but the test target cannot compile or run anywhere, and its header claims about host coverage and enforcement scope are false) |
| Reachable from shipped entry point? | N/A (test code, not shipped). CI-reachable: YES — ci.yml:731 runs `cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets` in the unconditional `tauri-check` job (on: push/pull_request to main, no `if:`, no `continue-on-error`) — but that step cannot pass on the current tree (see F-001) |
| Findings | S0: 1 S1: 0 S2: 2 S3: 0 S4: 2 |
| Commands run | Read + rg (cited inline); sed -n + python3 byte-dumps with `[`→`<LB>` marker (this sandbox's Bash output pipeline swallows `[`+letter pairs — the artifact ci.yml:21-30 documents; the marker dump is the byte truth); `python3 audit/runs/W14-A05-replicate.py` -> both tests' logic PASS against real src/app.rs + 5 mutation probes; `python3 audit/runs/W14-A05-paren-probe.py` -> paren-form probe. No cargo (crate unbuildable here: gdk-sys, baseline cargo-build-desktop-tauri.txt:310; and E0425 everywhere — cargo-build-budget) |

## 1. One-paragraph verdict

This test does NOT run the tauri app, a tauri context, a command, or any library-level seam — it is a pure source-text scan: `include_str!("../src/app.rs")`, strip `//` lines, string-hunt `generate_handler!`, extract the `[`…`]` body, assert it equals `["lumen_dispatch"]`, plus a same-file occurrence count of 1. That logic is sound for mutations at the one scanned site (I replicated it byte-faithfully in python: adding a second command to the array fails the assert; paren-form or macro-less handler registration fails loudly via `unreachable!`). But the test is dead code masquerading as a living gate: its own crate's lib target is compile-broken (src/app.rs:301 calls `spawn_session_pump`, defined nowhere — the fn is `spawn_event_pump`, app.rs:190; Wave 13 F-001), and cargo integration tests link that lib, so this test cannot compile on ANY host, GTK-equipped or not; in this sandbox it dies even earlier at gdk-sys (no GTK). Its header claims — "honest on every host — including hosts without webkit" and "`cargo test` inside it still runs the full battery of the shell library it links" — are both false, and INFRA-BLOCKED.md:33's "CI compiles it for real" is false against the current tree: the ci.yml:731 step must fail at compile before a single assertion executes. No recorded green run of this test exists anywhere in the repo's evidence trail (worklog:1631 admits only a "Local simulation PASS").

## 2. What actually works (verified)

- Verified by `python3 audit/runs/W14-A05-replicate.py` (faithful port of `registration_source` + both tests against the real src/app.rs): test 1 `registered: ['lumen_dispatch'] -> PASS`; test 2 `sites: 1 -> PASS`. The comment-strip + bracket-extraction logic is correct for the current source, and its index arithmetic (relative slice finds) is correct.
- Mutation probes (same script): adding a second command at app.rs:293 is CAUGHT (`registered=['lumen_dispatch', 'sneaky_cmd']` → assert fails); replacing the macro with a hand-rolled closure or removing it is CAUGHT loudly (`unreachable!` panic). Within the scanned file the gate is tight.
- `rg "generate_handler" apps/desktop-tauri/` -> only src/app.rs:18 (doc comment, stripped) and src/app.rs:293 (the registration): the single-site premise holds in the crate today.

## 3. Findings

F-001 [S0]
Where: apps/desktop-tauri/tests/single_command.rs:38-78 (the whole test target); root cause src/app.rs:301
Code (src/app.rs:301, byte-verified via marker dump):
```
            spawn_session_pump(app.handle().clone());
```
What is wrong: `spawn_session_pump` has no definition in the repository (rg over the crate: the defined ticker is `fn spawn_event_pump(app: AppHandle)` at src/app.rs:190) — E0425. `src/lib.rs:7` `pub mod app;` puts app.rs in the lib target; cargo builds the lib before any integration test, so this test target cannot compile on any host. INFRA-BLOCKED.md:31-33 sells it as live enforcement —
```
- **The §7.4 anti-bypass audit** (`tests/single_command.rs`): reads the
  registration site, fails on any second command; plus a
  unique-site check. (Local simulation PASS; CI compiles it for real.)
```
— and src/app.rs:25-27 says the single-command rule is "enforced by `tests/single_command.rs`". Nothing enforces anything: the ci.yml:731 step `cargo test --manifest-path apps/desktop-tauri/Cargo.toml --locked --all-targets` (unconditional `tauri-check` job, push/PR to main, no continue-on-error) must fail at compile on the current tree. Why it matters: a test proving nothing is used as the anti-bypass evidence for the entire ADR-004 "one command" posture (doc 05 §5.1/§7.4 item 3). What correct looks like: fix the E0425 (rename call to `spawn_event_pump`), re-run the tauri-check lane, and record a real green run. Confidence: HIGH — Verification: read + rg + cross-ref W13-A10 F-001 (audit/files/apps/desktop/src/shell.rs.audit.md) and wave-13/SUMMARY.md:40.

F-002 [S2]
Where: apps/desktop-tauri/tests/single_command.rs:7-10
Code:
```
//! Source-scanning (not compiling) the wrapper keeps the test honest on
//! every host — including hosts without webkit (this crate is
//! workspace-excluded; `cargo test` inside it still runs the full
//! battery of the shell library it links). The single-command rule is a
//! REGISTRATION rule, so the test reads the registration site in
```
What is wrong: two false claims. (a) "not compiling the wrapper … every host — including hosts without webkit": source-scanning avoids RUNNING webview, but the integration test target still COMPILES the wrapper lib and its whole tauri closure — on a host without GTK/webkit dev packages the test cannot build at all; orchestrator baseline proves it: `error: failed to run custom build command for `gdk-sys v0.18.2`` (audit/runs/baseline/cargo-build-desktop-tauri.txt:310). (b) "runs the full battery of the shell library it links": rg shows zero `#[test]`/`#[cfg(test)]` in apps/desktop-tauri/src/ — the crate's entire battery IS these two tests; apps/desktop's test battery runs in the workspace lane (this crate is excluded, root Cargo.toml:51 `exclude = ["apps/desktop-tauri"]`), never via this manifest. Why it matters: the header is the repo's own justification for why the gate is trustworthy cross-host; both halves are false. What correct looks like: state the real contract — "the scan avoids launching the app; the test still requires the tauri toolchain to build; it covers only src/app.rs". Confidence: HIGH — Verification: read + rg + baseline citation.

F-003 [S2]
Where: apps/desktop-tauri/tests/single_command.rs:67-77
Code:
```
/// The wrapper's registration is the ONLY `generate_handler` site in the
/// crate's code — a second site (smuggling its own handler) would
/// bypass the single-command audit above.
#[test]
fn the_registration_site_is_unique_in_the_crate() {
    let source = registration_source();
    let sites = source.matches("generate_handler!").count();
    assert_eq!(
        sites, 1,
        "exactly one `generate_handler!` call site may exist in src/app.rs code"
    );
}
```
What is wrong: the name (`..._unique_in_the_crate`) and doc promise crate-wide uniqueness — the exact smuggling threat the author names ("a second site … would bypass") — but `registration_source()` reads only `include_str!("../src/app.rs")` (line 23); the crate has three other source files (src/main.rs, src/lib.rs, src/frame_channel.rs — verified by ls) that are never scanned. A smuggled `tauri::Builder` with its own `generate_handler![a, b]` in any of them keeps both tests green while the shipped `lumen-tauri` binary registers extra commands. No second site exists today (rg: only app.rs:18/293), so this is a latent hole, not a live bypass — but for an anti-bypass gate the hole is the failure mode it exists to catch, and `main.rs` (the bin, the actual shipped entry) is precisely the most natural smuggling spot. Why it matters: the gate enforces a spelling in one file, not the crate's registration surface (doc 05 §5.1: "The desktop shell registers a single Tauri command" — crate-scoped). What correct looks like: scan every file in src/ (walk the directory or include_str! each), or assert on the compiled registration. Confidence: HIGH — Verification: read + structural proof + `python3 audit/runs/W14-A05-paren-probe.py` file listing; no runtime demo possible (test unbuildable, F-001).

F-004 [S4]
Where: apps/desktop-tauri/tests/single_command.rs:24-31
Code:
```
fn registration_source() -> String {
    let source = include_str!("../src/app.rs");
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//")
        })
```
What is wrong: only `//`-line comments are stripped; `/* ... */` block comments and string literals are scanned as code. Verified with a replication probe: inserting `/* generate_handler![decoy] */` before `pub fn run()` makes test 1 parse the decoy (`registered=['decoy']` → assert fails) and test 2 count 2 sites — a spurious RED from commented-out text (fail-noisy, not a bypass). Similarly a `generate_handler!` inside a string literal would be counted. No current trigger (app.rs has exactly one code occurrence). Confidence: HIGH — Verification: ran (python replication `W14-A05-replicate.py`, "block-comment decoy" probe).

F-005 [S4]
Where: apps/desktop-tauri/tests/single_command.rs:47
Code:
```
        .unwrap_or_else(|| unreachable!("generate_handler![…] array form (doc 05 §5.1 verbatim)"));
```
What is wrong: the message cites "doc 05 §5.1 verbatim" for the `generate_handler![…]` array form, but docs/05-command-layer.md §5.1 (lines 457-467) contains no `generate_handler` at all — it shows only the `#[tauri::command] async fn lumen_dispatch(...)` signature. The array-form promise actually lives in apps/desktop/src/dispatch.rs's module docs (quoted at src/app.rs:18). A diagnostic that cites a wrong source sends a fixer to the wrong doc. Confidence: HIGH — Verification: rg `generate_handler` docs/05-command-layer.md -> 0 hits.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `registration_source` | 22-32 | REAL logic / UNRUNNABLE target | S4 | strips only `//` lines; `/* */` and string literals survive (F-004); reads ONE file of four in src/ (F-003) |
| `the_shell_registers_exactly_one_command_and_it_is_lumen_dispatch` | 38-65 | REAL logic / UNRUNNABLE target | — | in-file mutations all caught (replication: array-add → assert; paren-form/macro-removal → `unreachable!` panic); never executes the command or app (source-scan only) |
| `the_registration_site_is_unique_in_the_crate` | 70-78 | REAL logic / UNRUNNABLE target / over-promising name | S2 | name+doc claim crate-wide; body enforces app.rs-only (F-003) |

## 5. Test honesty

| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `the_shell_registers_exactly_one_command_and_it_is_lumen_dispatch` | Only for registration-TEXT mutations at src/app.rs (verified by replication: second command in array → assert fails; paren-form / hand-rolled closure / removal → `unreachable!` panic). NO for every behavioral break — if `lumen_dispatch` returned garbage, `run()` panicked at startup, or the event pump deadlocked, the scan stays green; nothing of the app is ever executed. And on the current tree it can neither pass nor fail: the target cannot compile (F-001). |
| `the_registration_site_is_unique_in_the_crate` | Only for a second `generate_handler!` inside src/app.rs. NO for second sites in src/main.rs, src/lib.rs, src/frame_channel.rs (never scanned). Block-comment text can trigger false failures (F-004). |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "reads the registration site, fails on any second command; plus a unique-site check. (Local simulation PASS; CI compiles it for real.)" | INFRA-BLOCKED.md:31-33 | HALF-REAL / "CI compiles it for real" FALSE on current tree | logic verified by replication; CI step ci.yml:731 cannot pass — E0425 (src/app.rs:301), cross-ref W13-A10 F-001 |
| "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" | INFRA-BLOCKED.md:3-5 | FALSE against current tree | same E0425; established by W13 (audit/files/apps/desktop/src/shell.rs.audit.md:24-27); this audit adds: it also kills the test lane, not just the build |
| "Source-scanning (not compiling) the wrapper keeps the test honest on every host — including hosts without webkit" | tests/single_command.rs:7-8 | FALSE | integration test compiles the lib + tauri closure; gdk-sys build failure (baseline cargo-build-desktop-tauri.txt:310) |
| "`cargo test` inside it still runs the full battery of the shell library it links" | tests/single_command.rs:8-10 | FALSE | 0 `#[test]` in crate src/ (rg); battery = these 2 tests; apps/desktop's tests run only in the workspace lane, never via this crate's manifest |
| "an architecture test reads the desktop shell's invoke_handler registration and fails if any command other than `lumen_dispatch` is registered" | docs/05-command-layer.md:616 (§7.4 item 3) | REAL-UNPROVEN | logic implemented + replication passes; the test target itself has no recorded execution anywhere (worklog:1631 "Local simulation PASS" only) |
| "enforced by `tests/single_command.rs`" (single-command rule) | src/app.rs:24-27 | PAPER-ONLY on current tree | the enforcement instrument cannot build (F-001) |
| "generate_handler![…] array form (doc 05 §5.1 verbatim)" | tests/single_command.rs:47 | FALSE citation | rg: 0 `generate_handler` in docs/05-command-layer.md; the verbatim form is quoted from dispatch.rs docs at src/app.rs:18 |

## 7. Dead / duplicated / unreachable code

- The entire test target is currently UNREACHABLE at runtime: dead in this sandbox (gdk-sys, environment limit) and dead on every host (E0425, repo defect — W13 F-001). The 4744-passed workspace baseline does NOT cover it: `rg 'single_command|desktop-tauri' audit/runs/baseline/cargo-test-workspace.txt` -> 0 hits (crate is workspace-excluded, root Cargo.toml:51).
- The test's own doc header (lines 12-14) references `check_architecture` (xtask) and "the ESLint/boundary lanes" for the other §7.4 halves — out of this file's scope, listed for the next agent.

## 8. Dependencies & flags

- Test target links the crate lib `lumen-desktop-tauri` (implicit; integration-test semantics) → inherits deps `tauri 2` (locked 2.12.5, Cargo.lock:3688-3690), `desktop` (path), `commands` (path), serde, tauri-build. No features involved; crate is workspace-EXCLUDED (root Cargo.toml:51) so no workspace lane can ever run it — only ci.yml `tauri-check`, tauri-launch.yml, release.yml reference the manifest.
- Locked tauri 2.12.5 API fact (from the local registry cache, extracted source): `Builder::invoke_handler` accepts any `F: Fn(Invoke<R>) -> bool + Send + Sync + 'static` (tauri-2.12.5/src/app.rs:1723-1729) — `generate_handler!` is sugar; `generate_handler` is a delimiter-agnostic function-like proc macro (tauri-macros-2.7.4/src/lib.rs:177-179, `Handler::parse` = `parse_terminated(CommandDef::parse, Token![,])`). Consequence: the true registration surface is wider than any `generate_handler!` text scan; only the in-file, bracket-form, macro route is policed (and today not even that can run).

## 9. Open questions for the next agent

- Did the last green `tauri-check` run predate the `spawn_event_pump` rename (i.e., is E0425 an untested-since rename, or has the lane been red)? Needs CI run history — UNVERIFIED here (no git, no network).
- Does any UI/ESLint lane actually implement §7.4 items 1/2/4 (the header's parenthetical)? UNVERIFIED — outside this file.
- `ui/src/client/window-controls.ts:12` claims "single_command.rs audit stays untouched" for Chrome commands — if window-control commands ride `core:window:*` IPC (not `lumen_dispatch`), is the "single command" rule already semantically diluted? Worth a UI-side check.
