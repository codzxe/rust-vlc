# AUDIT: crates/platform-linux/src/linux/xdg.rs
| Field | Value |
|---|---|
| Auditor | W08-A06 |
| Date | 2026-11-23 |
| Lines / functions | 350 / 16 |
| File verdict | REAL (code) / UNREACHABLE (from shipped entry point) |
| Reachable from shipped entry point? | NO — platform-linux is an orphan crate: `rg platform-linux` over every Cargo.toml in the workspace matches only its own; apps/cli and apps/desktop dependency lists do not include it (verified). Wired only into `LinuxPlatform::dialog_seam` (src/lib.rs:203-231), itself called only by tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "XdgDialogs" -g '!audit/**'` (callers); `rg platform-linux crates/*/Cargo.toml apps/*/Cargo.toml` (no dependents); prebuilt integration binary run (xdg tests pass, see §2) |

## 1. One-paragraph verdict
This is the honest Linux dialog implementation: a real std-only D-Bus session-bus socket probe, a plainly documented typed refusal of the portal FileChooser client (`portal_filechooser_implemented() -> false` — no fake portal), and real zenity/kdialog subprocess invocation with explicit argv (no shell injection). Linux dialogs = zenity-or-kdialog command fallback only; there is NO xdg-desktop-portal client. The defects are a fallback "chain" that never falls through on failure (zenity present but failing blocks kdialog entirely), dropped filter labels in the kdialog path, and argv fidelity to the real tools that is unverifiable in this sandbox (neither zenity nor kdialog exists here — the tests only exercise fake stubs, which prove output parsing, not real-tool argv semantics). The whole crate is dead weight in the release lane: nothing depends on it.

## 2. What actually works (verified)
- Ran prebuilt `linux_integrations-ca1702194c57d626` -> `xdg_dialogs_run_the_zenity_fallback ... ok`, `xdg_portal_probe_reports_bus_truth ... ok` etc. (7 of 13 tests in that binary drive this file's code through real subprocesses and real Unix sockets).
- D-Bus probe verified live: the test binds a real `UnixListener` and `bus_socket()` connects to it (test output above).

## 3. Findings
F-001 [S3]
Where: crates/platform-linux/src/linux/xdg.rs:187-199
Code:
```rust
    fn run_command(
        &self,
        save: bool,
        request: &FileDialogRequest,
    ) -> Result<FilePick, PlatformError> {
        if let Some(zenity) = self.zenity.as_ref() {
            return run_zenity(zenity, save, request);
        }
```
What is wrong: The advertised "fallback chain" is discovery-order only. If zenity is present but its invocation fails (headless host: no DISPLAY → zenity exits non-1 with an error), the error is returned immediately; kdialog — installed and functional — is never attempted. Why it matters: on hosts with both tools installed, the dialog fails although a working backend exists; the module doc (line 1-2: "zenity/kdialog command fallback") and card 7.A2 language imply a chain. What correct looks like: on `CallFailed`/non-cancel failure from zenity, retry with kdialog before erroring. Confidence: HIGH — Verification: read + mutation reasoning (tests never install both a failing zenity and a working kdialog).
F-002 [S3]
Where: crates/platform-linux/src/linux/xdg.rs:328-338
Code:
```rust
    let filters = if request.filters.is_empty() {
        String::from("*")
    } else {
        request
            .filters
            .iter()
            .map(|f| f.patterns.join(" "))
            .collect::<Vec<_>>()
            .join("\n")
    };
```
What is wrong: `filter.label` is silently discarded in the kdialog path (patterns only); zenity's path (lines 292-298) passes `label|patterns`. A `FileFilter::new("Video files", ...)` renders its label in zenity but not in kdialog. Also the `\n`-joined multi-filter argv format for real kdialog cannot be validated in this sandbox (kdialog absent) — the fake stub accepts anything. Why it matters: silently ignored request fields; argv fidelity to real kdialog is UNVERIFIED. What correct looks like: kdialog filter syntax `*.mp4 *.mkv|Video files` per filter, joined per kdialog's documented separator. Confidence: MEDIUM — Verification: read; real-kdialog behavior UNVERIFIED (no binary on host).
F-003 [S4]
Where: crates/platform-linux/src/linux/xdg.rs:243-247
Code:
```rust
        if paths.is_empty() {
            // Success with no output: treat as cancel (both tools use
            // exit 1 for cancel, but a GUI kill can land here too).
            return Ok(FilePick::Cancelled);
        }
```
What is wrong: The comment's justification is wrong: a GUI kill produces a signal-terminated status, which `status.success()` reports as false, landing in the `CallFailed` arm below — not here. Exit-0-with-empty-stdout is treated as user cancel; defensible, but the documented rationale does not match the code path. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/platform-linux/src/linux/xdg.rs:157-161
Code:
```rust
    fn no_backend_error(&self) -> PlatformError {
        let bus_detail = match self.bus_socket() {
            Ok(path) => format!("session bus reachable at {}", path.display()),
```
What is wrong: Every no-backend refusal performs a second live `UnixStream::connect` to the session bus solely to render the error detail (the first connect happened at most in `portal_status`, never cached). Harmless (stream dropped immediately) but it is duplicate probe I/O on every refusal call. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PortalStatus (enum) | 30-44 | REAL | — | Honest two-state bus status |
| portal_filechooser_implemented | 46-54 | REAL (typed refusal) | — | `false` — no portal client; doc admits it; single source of truth quoted by report + error |
| XdgDialogs::discover | 65-85 | REAL | — | Real PATH split + DBUS_SESSION_BUS_ADDRESS read |
| XdgDialogs::with_config | 87-99 | REAL | — | Test seam, also used in production discovery? No — only tests |
| resolve_commands | 101-105 | REAL | — | Idempotent resolve |
| zenity / kdialog | 107-117 | REAL | — | Pure accessors |
| portal_status | 119-127 | REAL | — | Verified via bound-socket test |
| bus_socket | 129-153 | REAL | — | Real connect; abstract= refused with reason; first `path=` wins |
| no_backend_error | 155-184 | REAL | S4 | Names everything tried; double connect (F-004) |
| run_command | 186-199 | REAL | S3 | No failure fallback zenity→kdialog (F-001) |
| FileDialogs::open_file/save_file | 202-210 | REAL | — | Thin delegation |
| find_executable | 212-228 | REAL | — | is_file + any x-bit; follows symlinks (metadata) — correct |
| interpret | 230-265 | REAL | S4 | Exit-1+empty→Cancelled correct for both tools; F-003 comment |
| run_zenity | 267-304 | REAL-UNPROVEN | — | argv correct per zenity docs to my reading; real zenity absent on host — UNVERIFIED against binary |
| run_kdialog | 306-350 | REAL-UNPROVEN | S3 | Label dropped (F-002); real kdialog absent — UNVERIFIED |

## 5. Test honesty
Covered in the linux_integrations.rs audit (file 1/7): 7 tests drive this file with real stub subprocesses and real sockets; they pin output parsing, cancel mapping, typed errors, and all four bus states. They do NOT pin argv fidelity against real zenity/kdialog (impossible in sandbox — no such binaries).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "portal `FileChooser` client itself is not implemented in v0" | xdg.rs:9-14 | REAL | `portal_filechooser_implemented() -> false` (lines 52-54), quoted by error + report |
| "zenity / kdialog (real, testable) ... spawned with explicit argv (no shell)" | xdg.rs:15-17 | REAL | `Command::new(program)` + `.arg()` only (273-298, 312-343); no `sh -c` anywhere |
| "Cancel is a first-class outcome" | xdg.rs:19-20 | REAL | `FilePick::Cancelled` mapping (246, 251) verified by test run |

## 7. Dead / duplicated / unreachable code
- Whole crate is orphan: no workspace crate depends on platform-linux (rg over all Cargo.tomls). Code is REAL but UNREACHABLE from lumen-cli/desktop release lane. Cross-ref: docs/feature-matrix.toml keeps Linux dialog/audio rows at phase_target 7 — the staging is documented, so this is planned wiring, not a hidden lie; still dead code today.

## 8. Dependencies & flags
- std-only (PermissionsExt, UnixStream, Command). No unsafe in this file. No feature gates; `pub mod linux` itself is `#[cfg(target_os = "linux")]` (lib.rs:58-59).

## 9. Open questions for the next agent
- Does real kdialog accept `\n`-joined filter groups and `--separate-output` for multi-select? UNVERIFIED (no kdialog on host). The integration tests cannot answer this — they use compliant fakes.
