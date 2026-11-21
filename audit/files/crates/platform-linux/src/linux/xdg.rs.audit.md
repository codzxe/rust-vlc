# AUDIT: crates/platform-linux/src/linux/xdg.rs (whole file, 351 lines)

| Field | Value |
|---|---|
| Auditor | W03-A09 |
| Date | 2026-11-21 |
| Lines / functions | 351 / 14 production units (+6 integration tests in tests/linux_integrations.rs) |
| File verdict | UNREACHABLE (real, tested code — no shipped entry point reaches it: `platform-linux` has zero consumers) |
| Reachable from shipped entry point? | NO. `platform-linux` is a workspace member (`crates/*` glob) but **nothing depends on it**: `rg platform.linux` over every Cargo.toml in the repo matches only its own manifest; apps/cli's deps have no platform crates; apps/desktop (the composition root) depends on lumen-cli/media-types/renderer-wgpu/video-engine/commands/testkit/media-clock only; desktop-tauri depends on desktop/commands/tauri/serde. The crate is a library awaiting the L5 composition the lib.rs docs defer to the Lead ("unify into a shared L5 crate later — [PROPOSAL]"). Its own integration suite IS executed: 13/13 linux_integrations tests pass in `orchrun-test-workspace-nofailfast.log.txt` (the platform_linux lib target at 3638, the suite incl. all 6 xdg tests). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read tool over the whole file + lib.rs (232) + tests/linux_integrations.rs 205-400; `rg -l "platform_linux" --glob '!crates/platform-linux/**' .` (consumers: none in code — only a docs gate log and the baseline test log); `rg -n "platform" apps/desktop-tauri/Cargo.toml apps/desktop/Cargo.toml` (dependency lists — no platform-linux); `sed -n '40,45p' Cargo.toml` (members glob); `rg -n "zenity\|xdg\|kdialog\|id = \"platform" docs/feature-matrix.toml` (no dialog matrix row — no inflation claim to check); `grep -A18 "Running tests/linux_integrations" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (13/13 ok) |

## 1. One-paragraph verdict
This file is the honest-stub pattern done right: the portal `FileChooser` D-Bus client is declared unimplemented by a `const fn -> false` that the capability report and the refusal message both quote (no "works on my machine" anywhere), the session-bus probe is a real `UnixStream::connect` with typed refusal of abstract sockets, the zenity/kdialog fallbacks spawn explicit argv with no shell, cancel is a first-class outcome, and the injectable seam has six integration tests driving real fake binaries (exec bits, ETXTBSY race retried — a CI failure this test file's own history documents). It is also, today, dead weight in the shipped sense: nothing in the repo calls `LinuxPlatform::discover()`, so no user can open a dialog through any shipped binary. The one real defect is in the exit-code interpretation: a tool that exits 1 **with stderr** — zenity's actual behavior for environment errors like a missing display, since zenity uses exit 1 for both cancel and error — is classified as `Cancelled` because the code consults stderr only for codes other than 1; a headless host's broken dialog environment is indistinguishable from a user clicking cancel, and no test covers that cell of the matrix. Two hygiene items: the D-Bus address parser takes the first `path=` field and would build a garbage `PathBuf` from a `;`-separated multi-address list, and the "fallback chain" only falls back at discovery time — a zenity that spawns but fails never defers to kdialog.

## 2. What actually works (verified)
— Honest stub declaration — verified: `portal_filechooser_implemented() -> false` (52-54) is quoted by `no_backend_error` (167-171) and lib.rs's `dialog_seam` (214-218); the test asserts it stays false (linux_integrations 380-382). No code path claims the portal works.
— Session-bus probe — verified + EXECUTED (engine-suite level): unset var, non-unix prefix, live socket, dead path, and abstract refusal all traced and test-pinned (linux_integrations 326-383; 13/13 passing in the no-fail-fast baseline).
— Command discovery — verified: `$PATH` split, empty entries filtered, `is_file()` + any-x-bit check (`mode & 0o111`), metadata follows symlinks (PATH-normal); `xdg_discovery_uses_the_real_environment` runs it against the real host.
— zenity invocation — verified: `--file-selection`, `--save`, `--title=`, `--multiple` + a real newline `--separator` (Rust `"\n"` is 0x0A — correct, since zenity's default multiple separator is `|`), `--filename=` with the trailing-`/` directory fix, `--file-filter=label|patterns`; explicit argv, no shell. The two-path fake-zenity test pins the parsing end-to-end.
— kdialog invocation — verified: `--getopenfilename`/`--getsavefilename` with a `.` default, `--multiple --separate-output` (newline-separated output — matches `interpret`'s line parsing), `--title` as a separate arg. Fake-kdialog test pins it.
— Typed refusals — verified: `no_backend_error` names every tried thing (bus state, zenity, kdialog, the portal gap) — test asserts all three substrings; spawn failure and non-zero exits map to typed `SpawnFailed`/`CallFailed` with stderr text (test: exit 2 + stderr → CallFailed code 2).
— Cancel-first-class — verified: exit-1-empty-stdout → `Cancelled` for both tools (tests), success-no-output → `Cancelled` (documented GUI-kill rationale).

## 3. Findings

### F-001 [S3]
Where: crates/platform-linux/src/linux/xdg.rs:250-264
Code:
```rust
    match output.status.code() {
        Some(1) if output.stdout.is_empty() => Ok(FilePick::Cancelled),
        code => {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
```
What is wrong: Exit code 1 is treated as "user cancelled" whenever stdout is empty — but stderr is never consulted on that arm. Both zenity and kdialog exit 1 for *errors* as well as cancel (zenity's documented exit status for errors is 1; "cannot open display" on a headless session prints to stderr and exits 1 with empty stdout). The code even half-knows this — the comment at 244-245 admits "a GUI kill can land here too" — yet the discriminating signal (non-empty stderr) is read one line too late, only for codes ≠ 1. A dialog tool that failed because the environment is broken is reported as `Ok(FilePick::Cancelled)`: a swallowed environment error the host cannot distinguish from a user's deliberate cancel.
Why it matters: The exact environment this audit runs in (headless sandbox, no display) is the trigger: any host that eventually wires this seam will report "user cancelled" on every headless machine instead of surfacing the display error. The tests cover exit1-no-stderr (cancel) and exit2-with-stderr (error) but never exit1-with-stderr — the one cell reality produces on broken hosts.
What correct looks like: `Some(1) if output.stdout.is_empty() && output.stderr.is_empty() => Ok(FilePick::Cancelled)` — and route non-empty-stderr exit-1 into the `CallFailed` arm (zenity's stderr line is the detail). Add the fake-zenity `echo 'cannot open display' >&2; exit 1` test asserting `CallFailed`.
Confidence: HIGH — Verification: read (control flow traced; the zenity exit-1-for-errors behavior is its documented convention; the untested matrix cell identified against the two existing tests).

### F-002 [S4]
Where: crates/platform-linux/src/linux/xdg.rs:138-152
Code:
```rust
        for field in body.split(',') {
            if let Some(path) = field.strip_prefix("path=") {
                let socket = PathBuf::from(path);
                return match std::os::unix::net::UnixStream::connect(&socket) {
```
What is wrong: The parser takes the first `path=` field of a comma-split body. D-Bus address *lists* (`DBUS_SESSION_BUS_ADDRESS` may hold `unix:path=/a;unix:path=/b` — the spec's `;`-separated candidate form) are not handled: the `;` and everything after it becomes part of the `PathBuf` (`/a;unix:path=/b`), producing a guaranteed-failing connect with a nonsense path in the error detail, instead of trying the second candidate or naming the list form as unsupported.
Why it matters: Robustness of a probe whose whole job is honest reporting; a host with a fallback address list gets a misleading "connect to /a;unix:path=/b failed" instead of either success via candidate 2 or a typed "multi-address lists unsupported". Rare in practice (single-address is the norm).
What correct looks like: Split on `;` first; if more than one address, either iterate candidates or return the typed "address lists unsupported" refusal mirroring the abstract-socket precedent at 146-150.
Confidence: MEDIUM — Verification: read (parsing traced; the multi-address env form is D-Bus's documented `;` syntax — no in-tree fixture exercises it, UNVERIFIED against a real bus setup).

### F-003 [S4]
Where: crates/platform-linux/src/linux/xdg.rs:187-199 (the chain), 299-302 (the propagation)
Code:
```rust
        if let Some(zenity) = self.zenity.as_ref() {
            return run_zenity(zenity, save, request);
        }
        if let Some(kdialog) = self.kdialog.as_ref() {
            return run_kdialog(kdialog, save, request);
        }
```
What is wrong: The "fallback chain" resolves once at discovery and commits: if zenity is found but fails at run time (`SpawnFailed` — e.g., a broken interpreter or missing shared lib — or a `CallFailed` display error), the error propagates and kdialog is never tried, even though it was discovered. The chain falls back only on absence, never on failure. Cosmetic sibling asymmetry: zenity filters carry `label|patterns` while kdialog filters drop the labels (328-337) — kdialog shows the raw pattern as the entry text.
Why it matters: The chain's advertised shape ("zenity/kdialog command fallback chain", lib.rs:40) suggests resilience it does not have; on a GNOME-less host with a broken zenity and a working kdialog, dialogs fail. No user today (the crate is unreachable), so S4.
What correct looks like: On `SpawnFailed` (and arguably exit-1-with-stderr, F-001), try the next discovered command before erroring; pass kdialog `pattern|label` filter groups.
Confidence: HIGH — Verification: read (control flow; no test drives zenity-present-but-broken with kdialog also present).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `PortalStatus` (enum) | 31-44 | REAL | — | Two honest outcomes with detail payloads. |
| `portal_filechooser_implemented` | 52-54 | REAL (honest false) | — | The single truth quoted by report + refusal; test-asserted false. |
| `XdgDialogs` (struct) | 58-63 | REAL | — | Resolved once at construction. |
| `XdgDialogs::discover` | 69-85 | REAL | — | `$PATH` + `$DBUS_SESSION_BUS_ADDRESS`; test runs it against the real host. |
| `XdgDialogs::with_config` | 90-99 | REAL | — | The test seam; used by all 6 tests. |
| `XdgDialogs::resolve_commands` | 102-105 | REAL | — | Idempotent delegation. |
| `XdgDialogs::zenity` / `kdialog` | 109-117 | REAL | — | Pure accessors; consumed by lib.rs's seam choice + tests. |
| `XdgDialogs::portal_status` | 122-127 | REAL | — | Probe wrapper. |
| `XdgDialogs::bus_socket` | 131-153 | REAL | S4 (F-002) | Real connect; abstract refusal; multi-address gap. |
| `XdgDialogs::no_backend_error` | 157-184 | REAL | — | Names everything tried; the honest refusal — test-pinned substrings. |
| `XdgDialogs::run_command` | 187-199 | REAL | S4 (F-003) | Discovery-time-only fallback. |
| `impl FileDialogs` (open_file/save_file) | 202-210 | REAL | — | Thin delegation to the chain. |
| `find_executable` | 213-228 | REAL | — | x-bits + is_file; symlink-following is PATH-correct. |
| `interpret` | 231-265 | REAL (defective) | S3 (F-001) | Exit-1 stderr-blind cancel classification. |
| `run_zenity` | 268-304 | REAL | — | Correct flag construction incl. the newline separator and trailing-`/`; explicit argv; typed SpawnFailed. |
| `run_kdialog` | 307-350 | REAL | — | Correct mode flags; `.` default; label-less filters (S4 note in F-003). |

## 5. Test honesty (tests/linux_integrations.rs:205-383, all passing in baseline)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `xdg_dialogs_run_the_zenity_fallback` (209) | YES | Real fake binary with exec bits; asserts discovery + two-path parse. |
| `xdg_dialogs_cancel_is_not_an_error` (243) | YES | Both tools' exit-1-empty → Cancelled. |
| `xdg_dialogs_kdialog_fallback_parses_output` (266) | YES | kdialog path end-to-end. |
| `xdg_dialogs_without_any_backend_refuse_with_the_typed_error` (286) | YES | Refusal substrings + the exit-2-stderr → CallFailed cell. |
| `xdg_portal_probe_reports_bus_truth` (326) | YES | All five probe outcomes incl. a real bound socket. |
| `xdg_discovery_uses_the_real_environment` (370) | WEAK | Calls discover and discards both results (`let _`) — only proves no panic; the honest-no-guess claim is unasserted. |
| `open_file_retrying_etxtbsy` (47) | — | Harness, not a test — but its history comment (CI pids 43400611226/43415927075) documents a real overlayfs exec race it works around: evidence the suite runs against real process spawning. |
| Coverage hole | — | No exit-1-with-stderr case (F-001); no multi-address env var (F-002); no zenity-broken+kdialog-present case (F-003). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The portal `FileChooser` client itself is not implemented in v0 … [`PortalStatus`] says exactly this" | xdg.rs:9-14 | REAL | const false + quoted in both consumers; test-pinned |
| "zenity / kdialog (real, testable) … spawned with explicit argv (no shell). The seam is injectable (`with_config`) so tests drive fake binaries" | xdg.rs:15-17 | REAL | 268-350 use `Command::new` + args only; all 6 tests use with_config |
| "Cancel is a first-class outcome, not an error" | xdg.rs:19-20 | SHALLOW | True for the cancel cell; the error cell (exit 1 + stderr) is misclassified as cancel (F-001) |
| "This answers 'could a portal client talk to the bus at all'" | xdg.rs:6-8 | REAL | Real UnixStream::connect; live-socket test |
| "Dialogs: xdg-desktop-portal session-bus probe (std) with a zenity/kdialog command fallback chain" | lib.rs:39-42 | SHALLOW | The chain falls back on absence only (F-003) — and no shipped host reaches it at all (§Reachability) |
| (No matrix row claims dialogs — checked `rg "zenity\|xdg\|kdialog\|id = \"platform" docs/feature-matrix.toml`) | — | N/A | No inflation to flag; the platform rows are audio/video/hwdecode |

## 7. Dead / duplicated / unreachable code
- The FILE is unreachable-from-shipped (zero consumers of platform-linux — see header). Within the crate everything is wired: lib.rs's `dialog_seam` constructs `XdgDialogs::discover()` and its capability feeds `PlatformReport`.
- No duplication in-file. The three `platform-*` crates intentionally mirror each other's seam vocabulary (lib.rs:44-47 — a documented Lead deferral, not accident).

## 8. Dependencies & flags
Std-only (unix `PermissionsExt`, `UnixStream`, `Command`) + crate-internal seams. One cfg gate upstream (`target_os = "linux"` on `pub mod linux`, lib.rs:58). No unsafe, no feature flags, no panics (all fallibility typed; `unwrap`/`expect` absent).

## 9. Open questions for the next agent
1. Who owns the L5 composition: three platform crates with full seam/report machinery and zero consumers is a deliberate staging decision — the matrix/parity auditor should confirm no parity row counts `platform-linux` code as shipped capability (my matrix grep found no dialog row; the audio platform rows cite `platform-aout-*` — check their evidence strings name crates that ARE in a shipped graph).
2. events.rs/capability.rs (same crate, later in my list): keep the zero-consumer fact in view — every verdict there is UNREACHABLE from shipped entry points by the same proof as this file.
3. F-001's fix should land before any host wires the seam — headless hosts are the common CI case, and "user cancelled" is the failure mode hardest to debug from a bug report.
