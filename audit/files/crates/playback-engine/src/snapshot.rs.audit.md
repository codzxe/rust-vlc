# AUDIT: crates/playback-engine/src/snapshot.rs

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 253 / 8 functions + 7 tests |
| File verdict | UNREACHABLE (dead planning module; the wire command it plans for is refused by the only shipped dispatch) |
| Reachable from shipped entry point? | NO — `rg SnapshotConfig|SnapshotPlan|SnapshotFormat` over the whole repo: matches only inside snapshot.rs itself (definitions + tests). The `video.take-screenshot` command it plans for is catalog-only: crates/commands/src/catalog/command.rs:243-245 defines it, and the CLI dispatch answers it through the catch-all refusal apps/cli/src/dispatch.rs:154 `other => Err(unserved(other))` (the `[command-layer.unsupported-for-host]` typed refusal). |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:1 |
| Commands run | `sed -n` + `od -c` verbatim verifications (snapshot.rs 31-38; host.rs 425-434; dispatch.rs 153-155) — exit 0; `rg` caller searches over repo. No cargo (adaptation). Note: the Bash output pipeline mangles `#[must_use]` into `#ust_use]` (od -c proves the file contains `#[must_use]`); quotes below corrected against the byte dump. |

## 1. One-paragraph verdict

This module is a well-tested, honest **file-name planner** for a screenshot feature that does not exist in the shipped product. It computes `<dir>/vlcsnap-000001.png`-style output paths, refuses unknown formats and empty paths typed, and its seven tests are exact. But no code anywhere calls it — not the CLI, not the desktop, not the web host, not the commands layer. The wire command it plans for (`video.take-screenshot`, doc 05 §2.7) is defined in the catalog and even bound to Shift+S in keymaps/default.toml:161, yet the only shipped command dispatch routes it to the typed `unsupported-for-host` refusal. Worse, the module's own "structural gap" doc claims no pixel-readback seam exists, while apps/cli/src/host.rs:430-434 exposes a working `screenshot_png()` (the 2.D1 byte-stable encoder, test-exercised in apps/cli/tests/pipeline_accounting.rs:59 and tests/integration/first-vertical) — also with no production caller. The feature is a catalog row plus two disconnected halves: a planner nobody asks and an encoder nobody commands.

## 2. What actually works (verified)

- Path planning is correct and exact: derived names `{prefix}{counter:06}.{ext}` (153-158), explicit path wins with extension-selected format (138-145), unknown extension refused typed not fallen back (139 `SnapshotFormat::parse(ext)?`), empty explicit path / empty prefix refused (133-134, 147-149). All pinned by the in-file tests with exact `PathBuf` equality.
- `SnapshotFormat::parse` is case-sensitive per the inventory enum, refuses "jpeg"/"PNG" — pinned (176-185).
- `is_supported_v1` honestly reports PNG-only so a host can refuse jpg/tiff typed (79-81) — no host asks.

## 3. Findings

### F-001 [S2] Zero callers: the module is dead code, and the command it plans for is refused by every shipped host
Where: crates/playback-engine/src/snapshot.rs:131 (`pub fn plan` — the module's only entry point); dispatch gap apps/cli/src/dispatch.rs:153-155
Code (dispatch.rs:153-155, verbatim):
```rust
        )),
        // ---- everything else: the honest typed refusal ----
        other => Err(unserved(other)),
```
What is wrong: `plan()` has no caller in the repository (rg: `SnapshotConfig|SnapshotPlan|SnapshotFormat` match only snapshot.rs). The wire command `video.take-screenshot` is catalog data (commands/src/catalog/command.rs:243-245) and a keymap binding (keymaps/default.toml:161 `snapshot = { key = "Shift+S", ... command = "video.take-screenshot" ... }`), but `lumen_dispatch`'s match arm for it is the catch-all refusal — a Shift+S keystroke through any shipped host yields `[command-layer.unsupported-for-host]`, and the completion event `video.screenshot-saved` (event.rs:190) can never fire.
Why it matters: docs/feature-matrix.toml:1602 uses this file as its `evidence` string for the screenshot row ("crates/playback-engine/src/snapshot.rs (SnapshotFormat/SnapshotConfig::plan + tests …)"). That is the path-existence-only evidence pattern the audit was chartered to expose: the row's proof is a module that no shipped entry point can reach, planning a capture that no command executes. The matrix row is at best SHALLOW-planning-only, at worst FALSE depending on its status wording (row adjudication belongs to waves 18/19).
What correct looks like: dispatch `VideoTakeScreenshot { path }` → `host.screenshot_png()` (which already exists) + `plan()` for the output path + `video.screenshot-saved` emission — three small wires, none of which exist.
Confidence: HIGH — Verification: rg caller enumeration + read of dispatch.rs serve list.

### F-002 [S3] The module's "structural gap" doc is false: a working pixel-readback seam exists in the CLI host
Where: crates/playback-engine/src/snapshot.rs:31-38 vs apps/cli/src/host.rs:427-434
Code (snapshot.rs:31-38, verbatim):
```rust
//! # The structural gap (honest)
//!
//! A real capture needs pixel readback from the **presenting renderer**
//! — the video-engine sink hands frames to the platform surface and
//! keeps no copy. No frame-grab seam exists there today; wiring one is
//! video-engine/renderer-owned work. This module pins everything the
//! command layer *can* decide ahead of that seam, so the readback lands
//! onto a planned, tested path.
```
Code (host.rs:427-434, verbatim, `#[must_use]` confirmed by `od -c`):
```rust
    /// The current surface as a PNG screenshot (2.D1's byte-stable
    /// encoder) — `None` before a video limb exists.
    #[must_use]
    pub fn screenshot_png(&self) -> Option<Result<Vec<u8>, media_types::EngineError>> {
        let consumer = self.consumer.as_ref()?;
        let guard = consumer.lock();
        let video = guard.video()?;
        Some(video.screenshot_png())
    }
```
What is wrong: the snapshot.rs doc asserts no frame-grab seam exists. The CLI host's headless video consumer has one — `screenshot_png()` produces encoded PNG bytes today, and tests exercise it (apps/cli/tests/pipeline_accounting.rs:59; tests/integration/first-vertical/src/harness.rs:302,323 and scenarios.rs:672). The doc's framing ("readback lands onto a planned, tested path" once video-engine wires a seam) misstates where the gap is: the capture works in the headless host; what is missing is (a) any dispatch arm for the command and (b) any caller of `plan()`.
Why it matters: an engineer reading snapshot.rs would conclude the blocker is renderer-owned pixel readback; the actual blocker is three lines of dispatch wiring. Docs that mislocate the work are how this project accumulated unplanned paper.
Confidence: HIGH — Verification: read + rg (screenshot_png call sites: host.rs:434 self, two test files).
Note: for the *desktop* presenting renderer (real wgpu surface) the readback claim may still be true — that path is another agent's files; the false statement is the unqualified "no frame-grab seam exists there today".

### F-003 [S4] `counter` is host-incremented by convention; no host increments it
Where: crates/playback-engine/src/snapshot.rs:107-109 — "The session's monotonic snapshot counter (drives the derived filename; the host increments it per capture)". No host holds a `SnapshotConfig` at all (F-001), so the monotonic-naming contract (`counter_advances_names_monotonically` test) is upheld by nobody. Folded into F-001's deadness; recorded because the test implies a live mechanism.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SnapshotFormat::parse` | 57-65 | REAL (dead) | — | case-sensitive, typed None refusal |
| `SnapshotFormat::as_str` | 68-75 | REAL (dead) | — | |
| `SnapshotFormat::is_supported_v1` | 78-81 | REAL (dead) | — | honest PNG-only; no caller asks |
| `Default for SnapshotConfig` | 112-121 | REAL (dead) | — | VLC defaults |
| `SnapshotConfig::plan` | 131-167 | REAL (dead) | S2 | correct planning; zero callers (F-001) |
| tests (7) | 175-252 | REAL | — | see §5 |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| format_parses_the_inventory_enum_and_refuses_the_rest | YES (reasoned) | exact enum + jpeg/PNG refusals + v1 support flags |
| derived_path_uses_vlc_prefix_counter_and_extension | YES (reasoned) | exact PathBuf `/tmp/snaps/vlcsnap-000001.png` |
| explicit_path_wins_and_its_extension_selects_the_format | YES (reasoned) | three-way extension behavior |
| unknown_extension_is_refused_typed_not_fallen_back | YES (reasoned) | `.webp` → None |
| empty_explicit_path_or_prefix_is_refused | YES (reasoned) | both refusal families |
| no_directory_keeps_the_plan_relative_for_the_host | YES (reasoned) | relative path shape |
| counter_advances_names_monotonically | YES (reasoned) | 000000 → 000001 |

All seven are honest and exact — they test a module nothing uses. Mutation run not possible (no cargo): reasoned, UNVERIFIED at runtime.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| doc 05 §2.7 `video.take-screenshot` (class A, completes on `video.screenshot-saved`) | docs/05-command-layer.md:274 | FALSE for shipped build | dispatch.rs:154 refuses it; event cannot fire (F-001) |
| "No frame-grab seam exists there today" | snapshot.rs:34-36 | FALSE (for the CLI host) | host.rs:430-434 working `screenshot_png`, test-exercised (F-002) |
| feature-matrix evidence: snapshot.rs proves the screenshot row | docs/feature-matrix.toml:1602 | SHALLOW/UNREACHABLE | module exists+tested but has zero callers; the command is refused (F-001) |
| wire v1 encodes PNG only; jpg/tiff parse but refuse | snapshot.rs:26-29 | REAL | is_supported_v1 honest — though moot while unreachable |

## 7. Dead / duplicated / unreachable code

The entire module, relative to any shipped entry point. `DEFAULT_SNAPSHOT_PREFIX`, `SnapshotPlan`, `SnapshotConfig` — all uncalled outside the file's tests.

## 8. Dependencies & flags

- std paths only; no deps, no feature gates. Compiles into playback-engine unconditionally (W00: release PASS) — dead weight shipped in every binary.

## 9. Open questions for the next agent

- Does the desktop-tauri/web host serve `video.take-screenshot` through any path other than lumen_dispatch's catch-all? (apps/desktop-tauri/src/lumen_dispatch.rs — if the desktop ships its own dispatch, re-check there.)
- Is `video.screenshot-saved` referenced by the UI as a live event (a dead listener would compound F-001)? Files: ui/src/**, apps/web/src/protocol.rs.
- feature-matrix row at docs/feature-matrix.toml:1602 — waves 18/19 must adjudicate status vs this UNREACHABLE verdict.
