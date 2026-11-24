# CLAIM AUDIT: apps/desktop-tauri/INFRA-BLOCKED.md
| Field | Value |
|---|---|
| Auditor | W14-A04 |
| Date | 2026-11-23 (session) |
| Doc lines / claims extracted | 107 / 34 |
| REAL | 21 |
| REAL-UNPROVEN | 4 |
| SHALLOW | 0 |
| STUB | 0 |
| PAPER-ONLY | 0 |
| FALSE | 8 |
| UNREACHABLE | 0 |
| UNVERIFIED | 1 |
| Internal contradictions found | 9 (3 cross-doc, 6 doc-vs-tree) |

## 1. Blunt verdict
This doc's headline claim — "the crate compiles in its dedicated CI lane" — is FALSE
against the tree it ships with: `src/app.rs:301` calls `spawn_session_pump`, a function
that does not exist anywhere in the crate (the real name is `spawn_event_pump`,
defined at app.rs:190 and NEVER called — so the crate fails E0425 *and* would trip
dead-code under `clippy -D warnings`). Wave 13 proved this (W13-A10, orchestrator
rg-verified); I re-proved it on this tree. The doc is a wave-30 snapshot (status
2026-11-13, w30 progress 2026-11-15) that was never re-verified after the w31.1
frame-channel edits broke the call site; every one of its 8 FALSE claims is the same
failure mode — confident specifics (468 packages, 40 ms pump, 0-or-86 smoke,
`bundle.icon: []`, `core:default` only) rotted against code that moved on. The
structural claims (exclude, edges, CSP, CI lane steps, packaging scripts, crash
face) are real and verify cleanly. The doc is FALSE as a present-tense status
statement; it is REAL as a history of 2026-11-13/15.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "the crate compiles in its dedicated CI lane (`tauri-check`, Linux + webkit2gtk)" | INFRA-BLOCKED.md:4-5 | **FALSE** | app.rs:301 `spawn_session_pump(app.handle().clone());` — rg repo-wide: only def is `fn spawn_event_pump` app.rs:190 (never called). E0425 static-proof. audit/waves/wave-13/SUMMARY.md:39-43 (W13-A10). S0 |
| 2 | "The wrapper exists" | :3-4 | REAL | app.rs:288-322 `pub fn run()` read in full |
| 3 | "the anti-bypass audit exists" | :4 | REAL | tests/single_command.rs:39-78, two tests read |
| 4 | "This file supersedes the 4.H1 INFRA-BLOCKED handoff ... the 4.H1 text lives in git history" | :6-8 | UNVERIFIED | git history inaccessible (agents never run git); no 4.H1 copy in tree |
| 5 | "§B.1 is DONE (the real app ships in every payload; the stub is gone)" | :10-11 | REAL-UNPROVEN | scripts real (rows 24-25) but no payload artifact in tree; CI execution unverifiable; NOTE: this tree cannot build lumen-tauri at all (row 1), so the claim is unreproducible from HEAD |
| 6 | "the crash face + the playback pacer landed" | :13-14 | REAL | main.rs:45-67 (crash face); app.rs:171-232 (pacer) |
| 7 | "the WebView2 posture has a visible-dialog/crash-log half" | :13-14 | REAL | main.rs:49,53,64,82-88; diagnostics.rs:52,57,95,166 |
| 8 | "workspace-EXCLUDED (root Cargo.toml `exclude`)" | :18-19 | REAL | Cargo.toml:51 `exclude = ["apps/desktop-tauri"]` |
| 9 | "Own lockfile (`Cargo.lock`, 468 packages incl. the tauri closure)" | :21-22 | **FALSE** | `rg -c '^\[\[package\]\]' apps/desktop-tauri/Cargo.lock` -> **493** (not 468) |
| 10 | "Edges: `desktop` (path...), `commands` (path...), `tauri` (registry, E1 exception)" | :22-23 | REAL | apps/desktop-tauri/Cargo.toml:53-60 (`desktop = { path = "../desktop" }`, `commands = { path = ... }`, `tauri = "2"`) |
| 11 | "the verbatim `#[tauri::command] lumen_dispatch` ... `State<ShellState>` + `WireRequest` in, `Result<WireResponse, ShellError>` out" | :24-26 | REAL | app.rs:144-154 matches the described shape; CAVEAT: adds `app: AppHandle` param (:146) — "verbatim" is loose vs dispatch.rs:20-28's 2-param promise, deviation documented at app.rs:294-299 |
| 12 | "the 40 ms event pump" | :28 | **FALSE** | app.rs:109 `const SESSION_TICK: Duration = Duration::from_millis(10);` — 10 ms, not 40; own comment :102-105 says "33 ms audio quantum ... 30 fps". Stale number also in tauri-launch.yml:19 |
| 13 | "`apply_window_spec` from `product.toml [window]`" | :28-29 | REAL | app.rs:237-247 `parse_window_table(include_str!("../../../product.toml"))` |
| 14 | "drag-drop + argv file-open through the production `PathDragDrop` handler" | :29-30 | REAL | app.rs:92 (`use desktop::files::PathDragDrop`), :270-285 (argv), :305-320 (drop) |
| 15 | "ordering discipline under one `Mutex<ShellRuntime>`" | :30-31 | REAL | app.rs:113 (`pub struct ShellState(pub Mutex<ShellRuntime>)`), :150-152, :198-213 |
| 16 | "reads the registration site, fails on any second command; plus a unique-site check" | :31-33 | REAL | single_command.rs:39-65 (one-command assert), :70-78 (unique-site assert) |
| 17 | "(Local simulation PASS; CI compiles it for real.)" | :33 | **FALSE** (2nd half) | 1st half: my scan agrees — rg `generate_handler` app.rs -> 1 code site (:293, entry `lumen_dispatch`; :18 is a comment the test strips) -> assertions hold; cargo NOT run (gdk-sys env limit). 2nd half FALSE: the test target cannot compile on this tree (row 1, E0425) |
| 18 | "Real CSP ... `default-src 'self'` ... `connect-src ipc: http://ipc.localhost` ... `csp: null` is gone" | :34-38 | REAL | tauri.conf.json:23 — every element present (style-src 'unsafe-inline', media/blob, form-action/object-src/frame-src 'none'); `csp` is a string |
| 19 | "Capabilities: unchanged `core:default` only (no plugins — the 4.H1 posture)" | :39-40 | **FALSE** | capabilities/default.json:6-12: core:default + FOUR window grants (allow-minimize/toggle-maximize/close/start-dragging, w28). "No plugins" half TRUE |
| 20 | "CI lane (`ci.yml` → `tauri-check`): webkit2gtk dev packages, the ui/dist build, `cargo test --locked --all-targets`, clippy `-D warnings`, cargo-deny ... repo-root deny.toml" | :41-45 | REAL | ci.yml:680 (job), :703-711 (webkit2gtk apt), :719-724 (ui/dist), :731 (test), :733 (clippy), :738-743 (deny over subcrate manifest) |
| 21 | "the scoped `libloading` wrapper ... allowlisted only through `libappindicator-sys`, never blanket" | :45-47 | REAL | deny.toml:189 `{ name = "libloading", wrappers = ["libappindicator-sys"] }`; context :177-188 |
| 22 | "a link/exit smoke (0 or 86 acceptable)" | :47-48 | **FALSE** | ci.yml:744-775: smoke is `ls` + `ldd` link-resolution ONLY, never execs; :758-763 "there is no typed 86 'webview-pending' state in THIS binary; that exit code belongs to the pre-tauri skeleton" |
| 23 | "verdict flipped to 'imported (this crate's lockfile only)'; the workspace graph stays tauri-free" | :49-51 | REAL | docs/dependencies/records/tauri-wry-2.x.md:13-17; `rg '^name = "tauri|"wry|"tao"' Cargo.lock` -> 0 matches. ("R1–R9 refreshes there" not itemized — record declares fields R1–R10 at :3) |
| 24 | "the exit-86 `lumen-desktop` stub LEFT every user payload (zip/tar.gz/AppImage/deb/rpm/dmg + the MSI...)" | :60-62 | REAL-UNPROVEN | Script level real: verify-linux-artifacts.sh:161-162,180-181,210-211 (stub present => die), build-targz.sh:69, build-zip.sh:52, gen-wxs.sh:209/233/256; execution unverifiable (CI inaccessible) |
| 25 | "the AppImage `AppRun` and the macOS `CFBundleExecutable` launch `lumen-tauri`, not the stub" | :62-65 | REAL | Shipped lane: scripts/packaging/build-appimage.sh:93-107 (AppRun `exec "$HERE/$appdir_bin/${BIN_TAURI}"`), make-app.sh:77,106-107 (`CFBundleExecutable` = `${BIN_TAURI}`). CONTRADICTED by stale twin scripts/release/make-appimage.sh:63,72 — still execs `${BIN_DESKTOP}` (lumen-desktop, the stub) — and RELEASE.md:112 + release-checklist.md:104 still cite THAT script as the AppImage builder |
| 26 | "the `[VERIFY ...]` marker in `ui/src/client/tauri-transport.ts` ... clears ONLY when the launch lane proves an event frame round-trip" | :69-72 | REAL | tauri-transport.ts:93 marker still present (uncleared — consistent); tauri-launch.yml:39-50 records the clearing procedure |
| 27 | "the release lane now carries the Windows launch smokes — the zip lane launches the extracted `lumen-tauri.exe` (10 s survival + failure receipts) and the MSI upgrade test launches the installed exe" | :72-76 | REAL-UNPROVEN | release.yml:292-359 (zip smoke, 20x500ms survival + stderr receipts), :395 (`test-msi-upgrade.ps1`) — steps real, never observed; on THIS tree the lane dies earlier at `cargo build` (row 1) |
| 28 | "`tauri-launch.yml` deep verdicts ... remain manual" | :76-77 | REAL | tauri-launch.yml:6 "workflow_dispatch ONLY: manual by design" |
| 29 | "Icons (`bundle.icon: []`) + per-OS bundle metadata — a packaging follow-up" | :78-79 | **FALSE** | tauri.conf.json:29-32 lists `icons/icon.png` + `icons/icon.ico`; `file` -> "PNG image data, 512 x 512" / "MS Windows icon resource - 1 icon, 256x256". Icons exist; item is stale |
| 30 | "The dialog plugin (`pick_and_open`): ... (capabilities stay `core:default`)" | :80-82 | **FALSE** | Unwired half REAL (app.rs:78-82 "pick_and_open ... is NOT wired here"); capabilities restatement FALSE (row 19) |
| 31 | "ADR-004 per-OS verdicts + the security review (TB3 ...) — execute against the real launched app, then flip the ADR rows" | :83-85 | REAL-UNPROVEN | Open item by design; docs/adr/ADR-004-os-webview.md exists (not audited here — pointer for next agent) |
| 32 | "the failure is visible now (w30.6: panic hook + crash log under `<tmp>/lumen/` + a Windows no-console native MessageBox carrying the WebView2 actionable hint)" | :86-89 | REAL | main.rs:49 (`install_panic_hook`), :53 (`write_crash_log`), :36-40+:64 (gated MessageBox), :82-88 (WebView2 hint + bootstrapper URL); diagnostics.rs:52/57 (dir `lumen`, file `lumen-tauri-crash.log`) |
| 33 | "the no-bootstrapper assumption is still a pinned comment in the twins" | :91-92 | REAL | gen-wxs.sh:203-208 and build-msi.ps1:180-184 — identical comment "bootstrapper/detection row stays a recorded option for a future packaging lane" |
| 34 | Recipe (§C): build/test/clippy via `--manifest-path ... --locked --all-targets`; "tauri-cli is NOT used" | :96-107 | REAL | Mirrors ci.yml:731-733; `rg 'tauri-cli|cargo tauri' .github/workflows` -> 0 (only xtask gate.rs:21 row 9 references `cargo tauri build` — a dead gate row, Wave-1 S2, not a release-lane use) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "the crate compiles in its dedicated CI lane" (:4-5) | Tree: app.rs:301 calls undefined `spawn_session_pump` (E0425; def is `spawn_event_pump` :190, never called) | **Tree wins** (static proof, rg; W13-A10 concur). The crate cannot compile anywhere until fixed; "compiles" was true of some pre-w31.1 tree at best |
| "Own lockfile ... 468 packages" (:21-22) | Cargo.lock has 493 `[[package]]` sections | Cargo.lock wins (`rg -c` -> 493) |
| "the 40 ms event pump" (:28) | app.rs:109 SESSION_TICK = 10 ms | Code wins (sed-verified); tauri-launch.yml:19 repeats the stale 40 ms |
| "Capabilities: unchanged `core:default` only" (:39-40, restated :81) | capabilities/default.json:6-12 adds 4 core:window grants | File wins (sed-verified) |
| "a link/exit smoke (0 or 86 acceptable)" (:47-48) | ci.yml:758-763: binary is never exec'd; "there is no typed 86 ... in THIS binary" | ci.yml wins (the lane evolved after live rounds 43453921560/43455796103; doc is stale) |
| "Icons (`bundle.icon: []`)" remains (:78-79) | tauri.conf.json:29-32 lists 2 icons; both files exist (`file` verified) | Config wins; doc item is done, not open |
| INFRA-BLOCKED/ci.yml/main.rs cite real CI runs (tauri-check run 43384519598; 43453921560; 43455796103; 43400610890) — CI executes | docs/PARITY-REPORT.md:122-125: "CI is billing-blocked ... every GitHub Actions run dies at 0 steps" | **UNRESOLVABLE from sandbox** (no CI access). PARITY-REPORT is computed 2026-11-10 (its :7), i.e. BEFORE RELEASE.md:145-148 "Billing was unblocked 2026-11-12 ... executed (v0.1.0–v0.1.2)" — PARITY-REPORT is stale but still titled "finalized", and its citation of "docs/RELEASE.md known-limit #2" now points at a flipped (struck-through) limit. Register for contradiction-register owner |
| (implied) release payloads contain lumen-tauri built by CI | Current tree cannot build lumen-tauri (E0425) | Both cannot hold at HEAD: either payloads predate w31.1 or the executed-run story is wrong. Tree state is proven; CI history is not |
| "the AppImage AppRun ... launch lumen-tauri" (:63-65) | scripts/release/make-appimage.sh:63,72 still installs+execs `${BIN_DESKTOP}` (the exit-86 stub); RELEASE.md:112 and release-checklist.md:104 cite make-appimage.sh as the AppImage builder | The SHIPPED lane (scripts/packaging/build-appimage.sh, called via verify-linux-artifacts.sh per release.yml:831) execs BIN_TAURI — doc claim true for the shipped lane; the legacy twin + its doc citations are the defect (dead stub-launcher still referenced) |

## 4. Fabrication / inflation indicators
- **Present-tense compile-health claims never re-verified after w31.1 edits**: the doc
  says "compiles in CI" while the tree it lives in fails E0425 at app.rs:301 —
  the exact class of claim AGENT-SPEC §1.3 bans trusting.
- **Confident precise numbers all stale**: 468 packages (actual 493), 40 ms pump
  (actual 10 ms), "0 or 86 acceptable" (lane execs nothing). Pattern: specifics
  written at admission time, never re-derived.
- **Dangling citation chain**: PARITY-REPORT (2026-11-10, "finalized") cites
  RELEASE.md known-limit #2 as "workflow has never executed"; that limit is now
  struck through and says the opposite (unblocked 2026-11-12, v0.1.0–v0.1.2 ran).
  One of the two docs is wrong about CI; neither was reconciled.
- **Run IDs as evidence** (43384519598 etc. in ci.yml/main.rs comments) are
  unfalsifiable from the sandbox — recorded as UNVERIFIED, not accepted.
- **Stale twins shipped as truth**: scripts/release/make-appimage.sh still launches
  the exit-86 stub and is still cited by RELEASE.md/release-checklist.md, while
  INFRA-BLOCKED §B.1 asserts the stub "LEFT every user payload".

## 5. Matrix rows
Not a matrix chunk — N/A.

## 6. Notes for the next agent
- Fix priority (repo, not audit): app.rs:301 `spawn_session_pump` -> `spawn_event_pump`
  restores compilability (one-token fix, S0).
- audit/files/apps/desktop-tauri/src/frame_channel.rs.audit.md (prior agent) already
  rg'd `spawn_session_pump` repo-wide — consistent with this ledger.
- ADR-004 (docs/adr/ADR-004-os-webview.md) not audited here; its verdict rows are
  the flip-target this doc describes.
- PARITY-REPORT-vs-RELEASE billing contradiction needs an owner with CI access.
