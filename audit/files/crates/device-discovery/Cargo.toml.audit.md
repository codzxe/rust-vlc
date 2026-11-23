# AUDIT: crates/device-discovery/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-23 |
| Lines / functions | 58 / manifest (1 package, 1 dep, 1 dev-dep, lints) |
| File verdict | REAL (manifest works, dep hygiene recorded honestly) with one stale self-contradiction |
| Reachable from shipped entry point? | NO — the crate has zero dependents (monitor.rs audit F-002); a manifest cannot fix that |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg for testkit/UdpSocket usage in the crate; sed quote verification |

## 1. One-paragraph verdict
A clean, honest manifest: one workspace-internal dependency (media-types), testkit correctly dev-only, workspace lints inherited, publish=false, no features, no external crates — the "C5 std-only" claim holds. The interesting part is the recorded dependency decision: audit note T5 (2026-11-17) documents that plugin-api was REMOVED as dead weight after cargo-machete/cargo-shear flagged it, with the reference direction reversed (plugin-api's discovery.rs points AT this crate). That is exactly how a dep change should be recorded. The single defect: the header comment still opens with "deps media-types + plugin-api — exactly the Lead's registry row, no more" — a claim the [dependencies] section below it (and the T5 note) contradicts; the stale sentence survives at the top of the same file that corrects it.

## 2. What actually works (verified)
— The declared dep is real and used: `media-types` (EngineError/PipelineStage per volume.rs:229 `Result<Vec<Device>, EngineError>`); `testkit` dev-dep used at tests/ssdp.rs:19 `use testkit::Must;`; `std::net::UdpSocket` claim matches ssdp.rs:59. Baseline: crate compiles and 48 tests pass (38 lib + 10 e2e) under the workspace lint deny-lists — the manifest is load-bearing and correct.

## 3. Findings
F-001 [S4]
Where: crates/device-discovery/Cargo.toml:4-5 vs 43-53
Code (header):
```
# Registry: doc 01 §2.4 `device-discovery` (layer L2, deps
# media-types + plugin-api — exactly the Lead's registry row, no more).
```
Code (actual section, same file):
```
[dependencies]
# media-types: SourceUri (the Device vocabulary's uri slot) +
# EngineError/PipelineStage (the doc 01 §2.4 trait's error currency).
media-types = { path = "../media-types" }
```
What is wrong: the file's opening registry sentence asserts a two-dependency row ("media-types + plugin-api ... no more") that its own [dependencies] section and T5 note (L47-53) supersede — plugin-api is gone. A reader skimming the header takes away the wrong dependency contract; the honest correction sits 40 lines below in the same file. Why it matters: manifests are contract documents in this repo (the header exists precisely to pin the registry row); leaving the stale claim above the correction inverts the truth ordering. What correct looks like: header updated to "deps media-types (plugin-api removed by audit T5 — see note below)". Confidence: HIGH — Verification: read both sections.

## 4. Per-function table
Not applicable (manifest). Section inventory: [package] L33-38 (accurate description), [lints] L40-41 (workspace inherit — verified by baseline clippy-clean run), [dependencies] L43-53 (media-types only + T5 note), [dev-dependencies] L55-58 (testkit only).

## 5. Test honesty
Not applicable (no tests in a manifest). The crate's tests that this manifest builds all pass in baseline (device-discovery: 38 lib + fallback_paths 5 + ssdp 5 + volume_monitor 3).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "C5 discipline: std-only, zero external dependencies" | Cargo.toml:22 | REAL | only path deps: media-types (dep) + testkit (dev) |
| "SSDP needs no crate — std::net::UdpSocket carries multicast" | Cargo.toml:22-24 | REAL | ssdp.rs:59 `use std::net::{... UdpSocket}` |
| "deps media-types + plugin-api — exactly the Lead's registry row, no more" | Cargo.toml:4-5 | FALSE (stale) | plugin-api absent from [dependencies]; T5 note explains (F-001) |
| "until those cards land every lane reports the typed Unavailable verdict" | Cargo.toml:17-18 | REAL | volume.rs Unavailable fallback; but note the SSDP lane ships a real UdpSocket implementation — "every lane" overstates the fallback's coverage (monitor.rs's Scripted sources are test-only; no platform source exists). Partially stale phrasing. |

## 7. Dead / duplicated / unreachable code
No dead dependencies (the one that existed was removed, with a recorded rationale — good). Crate-level unreachability (zero dependents) is a wiring fact, not a manifest defect, and is filed in the monitor.rs audit.

## 8. Dependencies & flags
media-types (path), testkit (path, dev). No [features], no external crates, no [workspace] overrides. publish=false. Lints inherited (baseline clippy: CLEAN).

## 9. Open questions for the next agent
- doc 01 §2.4's registry row itself (outside this crate) still lists plugin-api as a dependency of device-discovery per the header quote — whoever audits docs/01 should confirm whether the registry doc was updated after T5 or whether the stale contract lives in two places.
