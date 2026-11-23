# AUDIT: crates/playback-engine/src/snapshot.rs
| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2025-06-14 |
| Lines / functions | 253 / 12 (5 prod items + 7 unit tests) |
| File verdict | REAL code, UNREACHABLE and EXECUTION-LESS — the filename/format planning half of `video.take-screenshot` works and is unit-tested, but zero consumers exist anywhere and the pixel-readback half the plan exists for does not exist |
| Reachable from shipped entry point? | NO. `pub mod snapshot;` (lib.rs:105) compiles it into the shipped lib, but rg over the tree (excluding audit/) finds SnapshotConfig/SnapshotPlan/SnapshotFormat referenced ONLY inside snapshot.rs (+ a worklog line). No app serves `video.take-screenshot` (rg over apps/: zero `TakeScreenshot`/`take-screenshot` handlers). Nothing calls `plan()`. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `./target/debug/deps/playback_engine-ac37a1ec1a681c2d snapshot::` -> 7 passed, exit 0 (prebuilt lib test binary); rg consumer sweeps (0 external callers) |

## 1. One-paragraph verdict
The planning logic is correct and well-tested: explicit-path-wins with extension-selected format, derived `vlcsnap-NNNNNN.png` names, typed refusals for empty paths/prefixes and unknown extensions, and an honest `is_supported_v1` that reports jpg/tiff as unencodable so a host would refuse typed instead of writing a lie. All 7 unit tests pass (direct binary run). But this is planning for a capability that does not exist: the module's own header records the structural gap ("A real capture needs pixel readback from the presenting renderer ... No frame-grab seam exists there today", lines 31-38), no host serves the command, and no code anywhere calls `SnapshotConfig::plan`. The doc is honest; the code is a tested, shipped-in-the-lib, dead-end policy core waiting for two integrations (a host arm and a renderer seam) that have not landed.

## 2. What actually works (verified)
- verified by `./target/debug/deps/playback_engine-ac37a1ec1a681c2d snapshot::` -> 7 passed; 0 failed (exit 0): enum parsing + case sensitivity, derived VLC naming (prefix/6-digit counter/extension), explicit-path precedence, extension override + no-extension fallback, typed refusal for unknown extensions (webp), empty-path/prefix refusal, relative plan without directory, monotonic counter names.
- Hand-checked `plan()`: `{:06}` zero-pads the counter (VLC convention); explicit empty path refused before any format logic.

## 3. Findings
F-001 [S2]
Where: crates/playback-engine/src/snapshot.rs:1-10, 31-38 + tree-wide consumer sweep
Code:
```
//! The **pure planning half** of a snapshot, mirroring
//! [`crate::frame_step`]: given the command's optional explicit path and
//! the `snapshot-*` config family, produce the output file the host
//! should write the presented frame to. Execution — reading pixels back
//! from the presenting renderer — is the structural gap recorded below.
```
What is wrong: the entire module has zero callers. No shipped host serves `video.take-screenshot` (rg over apps/: no matches); no crate imports SnapshotConfig/SnapshotPlan/SnapshotFormat; nothing invokes `plan()`. The capability the module plans for cannot be triggered by any user of any shipped binary, and even if a host served the command, the execution half (pixel readback) does not exist, so the screenshot could not be written. Doc 05 §2.7's `video.take-screenshot` is PAPER-ONLY at the product level.
Why it matters: the module's presence (plus the `VideoScreenshotSaved` catalog event kind) creates the impression of a screenshot capability; a feature-matrix row citing this file as screenshot evidence would be FALSE.
What correct looks like: either the host arm + renderer readback seam land, or the module waits on a card behind an explicit note in the catalog/docs that the command is unserved.
Confidence: HIGH — Verification: rg sweeps + direct test run.
F-002 [S4]
Where: crates/playback-engine/src/snapshot.rs:22-25
Code:
```
//!   **[PROPOSAL: modern VLC names snapshots by wall-clock time
//!   (`vlcsnap-YYYY-MM-DD-HHhMMmSSs`); the padded counter is the
```
What is wrong: the naming scheme is a `[PROPOSAL]` v0 pick (counter vs VLC-modern wall-clock names) — fine and honestly marked; noted because the whole family of decisions is provisional while the module is unreachable anyway.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SnapshotFormat (enum) | 43-51 | REAL | — | Png/Jpg/Tiff (VLC inventory) |
| SnapshotFormat::parse | 57-65 | REAL | — | case-sensitive; typed None refusal; tested |
| SnapshotFormat::as_str | 68-75 | REAL | — | canonical name/extension |
| SnapshotFormat::is_supported_v1 | 78-82 | REAL | — | PNG only — the honesty gate for unencodable formats |
| DEFAULT_SNAPSHOT_PREFIX | 85 | REAL | — | "vlcsnap-" |
| SnapshotPlan (struct) | 88-95 | REAL | — | output_path + format |
| SnapshotConfig + Default | 98-121 | REAL | — | directory/prefix/format/counter; VLC defaults |
| SnapshotConfig::plan | 130-167 | REAL | S2-context | correct precedence + typed refusals; zero callers (F-001) |
| tests (7) | 175-252 | REAL | — | full coverage of parse/precedence/refusal/naming/monotonicity |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 7 | YES | exact-path/equality assertions on every behavior the module claims |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Execution — reading pixels back ... is the structural gap" | snapshot.rs:9-10, 31-38 | REAL (honest gap) | no frame-grab seam exists (no renderer-wgpu readback API consumed; module has no callers) |
| "the host refuses typed instead of writing a lie" (jpg/tiff) | snapshot.rs:26-29 | REAL | is_supported_v1 false for Jpg/Tiff; tested — but no host exists to do the refusing (F-001) |
| doc 05 §2.7 `video.take-screenshot` | snapshot.rs:2 | PAPER-ONLY at product level | no host serves it (rg apps/) |

## 7. Dead / duplicated / unreachable code
- The entire module is dead code in every shipped build (F-001) — compiled, unit-tested, never called.

## 8. Dependencies & flags
- std only (Path/PathBuf) + testkit (dev). No features. Plain `pub mod` (lib.rs:105).

## 9. Open questions for the next agent
- Check the feature matrix / docs for any row claiming screenshot capability with this file as evidence — downgrade to PAPER-ONLY/UNREACHABLE (F-001).
- The `VideoScreenshotSaved` catalog event kind exists in commands — if a matrix row cites it as evidence of shipped screenshots, same downgrade.
