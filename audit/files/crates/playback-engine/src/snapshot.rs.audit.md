# AUDIT: crates/playback-engine/src/snapshot.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-21 |
| Lines / functions | 254 / 12 (3 format fns, Default, plan, 7 tests) |
| File verdict | UNREACHABLE (real, tested planning policy — zero callers) |
| Reachable from shipped entry point? | NO: `SnapshotConfig::plan` / `SnapshotFormat` have zero callers outside this file's tests (rg across crates/ + apps/); the `video.take-screenshot` command is in the catalog (commands/src/catalog/command.rs:243-245) but no host dispatches it (rg `VideoTakeScreenshot` in apps/cli, apps/desktop → no executor); the engine event `video.screenshot-saved` is not wired (playback-engine/tests/events_contract.rs:1681-1689, `#[ignore]` gap test, handoff H-5) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "SnapshotConfig\|SnapshotPlan\|take.screenshot" crates/ apps/` → catalog/keymap/events-contract references only, zero plan() callers; `rg -n "VideoTakeScreenshot" apps/cli/src/ apps/desktop/src/` → none; `rg -n "screenshot_png\(\)"` → host.rs:434 (def) + one test caller (apps/cli/tests/pipeline_accounting.rs:59); matrix rows read (feature-matrix.toml:1594-1603, 3356-3366); baseline: `rg -n "snapshot::tests::"` → 7 tests ok (orchrun :3810-3816) |

## 1. One-paragraph verdict
The path-planning half of video snapshots is real, correct, and well-tested
(7 tests, all ok in the no-fail-fast baseline): VLC's `vlcsnap-` prefix,
6-digit counter, extension-selected format, typed refusals for empty
paths/prefixes/unknown extensions — I verified each behavior by reading and
each test by mutation reasoning. But it is a planning module for a capture
that cannot happen: `plan()` has zero callers, the `video.take-screenshot`
command is cataloged but dispatched by no shipped host, the
`video.screenshot-saved` event is unwired (honest `#[ignore]` gap test), and
the pixel readback that DOES exist (`Host::screenshot_png`,
apps/cli/src/host.rs:430-435, reading the compositor surface) is called only
by a test. Five separately-tested components (catalog command, keymap
binding, this planner, host readback, renderer PNG encoder) never meet: the
snapshot feature is a disassembled puzzle, and this file is the unwired
keystone the matrix cites as evidence.

## 2. What actually works (verified)
— All 7 `snapshot::tests` executed ok: baseline
`audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3810-3816`.
— Mutation checks (by reading): swapping the extension-selection match arms
fails `explicit_path_wins_and_its_extension_selects_the_format` (:204-215);
dropping the empty-prefix refusal fails `empty_explicit_path_or_prefix_is_refused`
(:224-232); changing `{:06}` to `{:5}` fails `derived_path...` (:188-201,
expects `vlcsnap-000001.png`).
— `{:06}` is a minimum width, not a truncation: counter 12_345_678 prints
all 8 digits — no wraparound hazard in `plan` (pure; the counter increment
is host-owned, documented :107-109).
— VLC fidelity: default prefix `vlcsnap-` (:85), PNG default, jpg/tiff
parse-but-refused-in-v1 via `is_supported_v1` (:79-81) — honest enum.

## 3. Findings
### F-001 [S3]
Where: crates/playback-engine/src/snapshot.rs:36-38 (with :131)
Code:
```rust
//! video-engine/renderer-owned work. This module pins everything the
//! command layer *can* decide ahead of that seam, so the readback lands
//! onto a planned, tested path.
```
(`pub fn plan(&self, explicit_path: Option<&Path>) -> Option<SnapshotPlan>`
at :131 has no caller.)
What is wrong: The module's reason to exist — "so the readback lands onto a
planned, tested path" — is unfulfilled: `plan()` is called by nobody. The
readback half exists (Host::screenshot_png, apps/cli/src/host.rs:430-435 →
pipeline_host.rs:909, renderer-wgpu's byte-stable PNG encoder), the command
exists (catalog command.rs:243-245), the hotkey exists
(keymaps/default.toml Shift+S), the event id exists — and no code path
connects any two of them. `VideoTakeScreenshot` is dispatched by neither
apps/cli nor apps/desktop (rg: zero executor matches); `screenshot_png()`
is invoked only by a test (apps/cli/tests/pipeline_accounting.rs:59);
`video.screenshot-saved` is an `#[ignore]`d gap test (events_contract.rs:1681-1689).
Why it matters: A user pressing Shift+S in any shipped surface gets nothing
(or an unimplemented-command error) while the feature matrix records the
feature "partial" with this file as evidence; five tested components ship
without a single end-to-end wire between them.
What correct looks like: One dispatch arm: TakeScreenshot → plan() →
screenshot_png() → write file → emit VideoScreenshotSaved (or the honest
not-implemented receipt until the renderer seam lands).
Confidence: HIGH — Verification: rg sweeps (commands row) + ignored-test read.

### F-002 [S4]
Where: crates/playback-engine/src/snapshot.rs:54-65
Code:
```rust
    /// Parse the config value (`png`/`jpg`/`tiff`, case-sensitive per
    /// the inventory enum). `None` is the typed refusal for anything
    /// else — never a silent fallback.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
```
What is wrong: Extension matching inherits the case-sensitivity: a user's
explicit path `Snapshot.PNG` is refused typed (`parse("PNG")` → `None`,
pinned by test :181). VLC's own snapshot-format matching is
case-insensitive, and file extensions in the wild are mixed-case. The
documented choice ("case-sensitive per the inventory enum") conflates the
CONFIG enum (where case-sensitivity is right) with the EXTENSION match
(where the OS convention is case-insensitive).
Why it matters: A realistic explicit path is refused for a cosmetic reason;
the typed refusal is honest but the behavior diverges from the VLC parity
target.
What correct looks like: Config parsing stays case-sensitive; extension
matching lowercases before parse.
Confidence: MEDIUM (VLC's exact snapshot-format case behavior not verified
here — its config system is case-insensitive for option values generally) —
Verification: read + test :181.
Severity S4 because the blast radius is one refused capture with a typed
error, not silent corruption.

### F-003 [S4]
Where: crates/playback-engine/src/snapshot.rs:130-146
Code:
```rust
    #[must_use]
    pub fn plan(&self, explicit_path: Option<&Path>) -> Option<SnapshotPlan> {
        if let Some(path) = explicit_path {
            if path.as_os_str().is_empty() {
                return None;
            }
```
What is wrong: An explicit path that is only whitespace (" ") or only a
dot-ext (".png" — a hidden file with no stem, extension "png") is accepted;
an explicit path that IS an existing directory is accepted verbatim. The
planner validates emptiness only. None of these corrupt anything (the host
write would fail), but the "never a guessed file" discipline (:129) stops at
the string level.
Why it matters: The typed-refusal story covers the empty case only; the
near-empty cases defer failures to the filesystem layer with worse errors.
What correct looks like: Refuse a path whose file_name is empty or None
(trailing slash / dot paths); optionally refuse directories at write time.
Confidence: HIGH — Verification: read; Rust `Path::new(".png").extension()`
→ Some("png"), `Path::new(" ").as_os_str().is_empty()` → false.
Severity S4: bounded, host-visible as a write error.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SnapshotFormat::parse` | 58-65 | REAL | S4 (F-002) | Case-sensitive; typed None refusal |
| `SnapshotFormat::as_str` | 69-75 | REAL | — | Canonical names |
| `SnapshotFormat::is_supported_v1` | 79-81 | REAL | — | PNG only, honest |
| `DEFAULT_SNAPSHOT_PREFIX` | 85 | REAL | — | VLC's `vlcsnap-` |
| `SnapshotPlan` (struct) | 88-95 | REAL | — | Plain data |
| `SnapshotConfig` + Default | 98-121 | REAL | — | VLC defaults; counter host-owned |
| `SnapshotConfig::plan` | 131-167 | REAL (unreachable) | S3 (F-001), S4 (F-003) | Pure; explicit-wins, extension-selects, derived `<prefix><:06>.<ext>`; min-width padding (no truncation) |
| test `format_parses_the_inventory_enum...` | 176-185 | REAL | — | Ran ok (:3814) |
| test `derived_path_uses_vlc_prefix_counter...` | 188-201 | REAL | — | Ran ok (:3811) |
| test `explicit_path_wins_and_its_extension...` | 204-215 | REAL | — | Ran ok (:3813) |
| test `unknown_extension_is_refused_typed...` | 218-221 | REAL | — | Ran ok (:3816) |
| test `empty_explicit_path_or_prefix_is_refused` | 224-232 | REAL | — | Ran ok (:3812) |
| test `no_directory_keeps_the_plan_relative...` | 235-242 | REAL | — | Ran ok (:3815) |
| test `counter_advances_names_monotonically` | 245-252 | REAL | — | Ran ok (:3810) |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| format_parses_the_inventory_enum... | YES | Exact parse table incl. jpeg/PNG refusals |
| derived_path_uses_vlc_prefix... | YES | Exact PathBuf equality incl. zero-padding |
| explicit_path_wins... | YES | Three extension cases pinned |
| unknown_extension_is_refused_typed | YES | webp → None |
| empty_explicit_path_or_prefix_is_refused | YES | Both refusal arms |
| no_directory_keeps_the_plan_relative | YES | Relative PathBuf pinned |
| counter_advances_names_monotonically | YES | 000000 → 000001 |
All 7 ran ok in baseline (orchrun :3810-3816). No tautologies.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "playback-snapshot … status partial (all platforms)" | feature-matrix.toml:1594-1603 | SHALLOW as a feature status — the cited evidence (this file) is real and tested but unwired; no user-visible capture exists in any shipped binary | F-001: zero plan() callers, command undispatched, event unwired |
| "structural gap, honest: … no pixel-readback seam" | matrix notes :1603 + module doc :31-38 | HALF-TRUE — the module doc says no readback seam exists, but a host-level readback DOES exist (host.rs:430 screenshot_png reading the compositor surface, test-invoked); the gap is the WIRING, not the seam | host.rs:426-435 read; pipeline_host.rs:909 read |
| "wire v1 encodes PNG only (jpg/tiff parse, refused typed)" | module doc :26-29 + matrix notes | REAL | is_supported_v1 tested (:182-184) |
| "the video-engine sink hands frames to the platform surface and keeps no copy" | module doc :33-35 | FALSE in the current host — the CLI host's software compositor surface IS retained and readable (screenshot_png works in tests) | host.rs:430-435 + apps/cli/tests/pipeline_accounting.rs:59 |

## 7. Dead / duplicated / unreachable code
The entire public surface (`SnapshotFormat`, `SnapshotPlan`,
`SnapshotConfig`, `plan`) is production-dead (F-001): exported from
playback-engine's lib (lib.rs:105 `pub mod snapshot`), consumed by tests
only.

## 8. Dependencies & flags
`std::path` only — zero deps, no unsafe, no I/O, no feature gates. The
`// lumen-allow-literal: path/URI test vector` comments in tests show the
lint-literals discipline applied correctly (paths as data, not config).

## 9. Open questions for the next agent
- Whoever audits apps/cli/src/dispatch.rs or the command catalog execution:
  confirm no generic fallthrough executes class-A commands like
  VideoTakeScreenshot (my rg found no executor, but a generic
  serde-dispatch table would hide it — worth one targeted grep:
  `SessionCommandKind` match arms).
- The events_contract.rs `#[ignore]` gap tests (H-1..H-7) are a ready-made
  wiring checklist for the snapshot/playlist/settings events — the snapshot
  auditor of the UI wave should cross-reference them.
