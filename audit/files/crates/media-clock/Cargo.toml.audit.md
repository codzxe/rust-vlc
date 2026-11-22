# AUDIT: crates/media-clock/Cargo.toml

| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-14 |
| Lines / functions | 26 / manifest (no functions) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — media-clock is on the `lumen` open path (`Arc<MediaClock>` constructed in apps/cli/src/pipeline_host.rs:1605/1630; `MediaClock` re-exported through playback-engine session config). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | `cat` manifest (exit 0); spot-check greps in crates/media-clock/src/{clock,drift}.rs for the description's feature vocabulary. No cargo. |

## 1. One-paragraph verdict

Minimal and honest: one path dependency (media-types), testkit dev-only, std-only, publish=false, workspace lints. The `description` field makes five feature claims — audio-master timing, video/external master backends, master-loss fallback, Ended seek-back re-arm, bounded drift correction — and all five have corresponding vocabulary in the source (spot-checked: `ClockState::Ended` + `seek_back` re-arm at clock.rs:540-541; master-loss fallback at clock.rs:103; `MAX_CORRECTION_PPM = 1000` at drift.rs:35). The one claim-ledger entry: "bounded drift correction" is computed but never applied in the shipped build (the consumer-side governor is never armed — cross-ref my sync.rs audit F-001); the crate itself is not at fault.

## 2. What actually works (verified)

- Dependency set: media-types only — consistent with the L2 registry claim; no external crates; matches the "Std-only by C5" comment.
- Description-vs-code spot checks (grep): "Ended seek-back re-arm" → clock.rs:541 "only [`MediaClock::seek_back`] re-arms it"; "deterministic master-loss fallback" → clock.rs:103 "Free-running wall drift — the terminal master-loss fallback"; "bounded drift correction" → drift.rs:35 `pub const MAX_CORRECTION_PPM: i64 = 1000;`. All present.

## 3. Findings

### F-001 [S4] Description advertises drift correction as a shipped behavior; its application path is production-dead
Where: crates/media-clock/Cargo.toml:13 (description) — "bounded drift correction (doc 03 §4.7, card 3.B2)"
What is wrong: the crate computes `DriftCorrection` correctly (drift.rs), but the only consumer that applies it to audio (audio-engine `DriftGovernor`/`MicroResampler`) is never armed by any shipped host (see sync.rs audit F-001 for the caller enumeration). The manifest description is accurate about the crate's contents and silent about the fact that nothing uses them end-to-end. Claim-ledger entry, not a manifest defect.
Confidence: HIGH — Verification: cross-ref sync.rs audit (rg evidence).

## 4. Per-function table

N/A (manifest). Keys checked: `[package]` (name/version/edition 2024/publish=false), `[lints] workspace = true`, `[dependencies]` media-types path dep only, `[dev-dependencies]` testkit. No `[features]` — the crate compiles wholly in every build.

## 5. Test honesty

N/A here; the crate's tests live in its source files (sync.rs's 15 tests audited above; clock/drift/mapping/backends/source belong to other agents).

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only by C5: zero external dependencies, no unsafe" | Cargo.toml:8 | REAL for deps (manifest); "no unsafe" UNVERIFIED (other agents' files; rg unsafe in sync.rs: none) |
| "audio-master timing over an injectable sample counter" | Cargo.toml:13 | REAL | `SampleCounterSource` re-exported (lib.rs:126), consumed by the CLI host |
| "video/external master backends with deterministic master-loss fallback" | Cargo.toml:13 | REAL-UNPROVEN | vocabulary present in clock.rs/backends.rs; behavior not executed (no cargo) |
| "Ended seek-back re-arm" | Cargo.toml:13 | REAL-UNPROVEN | clock.rs:540-541 latch + re-arm read; not executed |
| "bounded drift correction" | Cargo.toml:13 | REAL (computation) / UNREACHABLE (application) | drift.rs constant + sync.rs audit F-001 |

## 7. Dead / duplicated / unreachable code

N/A (manifest). The unreachable-application note is F-001.

## 8. Dependencies & flags

No features, no externals. Note for wave 1: media-clock sits below both engines (L2) and is the dependency edge that makes sync.rs's "re-implemented beside the estimator" duplication (ProgressWindow vs RateWindow) structurally necessary — see sync.rs module doc lines 21-25.

## 9. Open questions for the next agent

- clock.rs/backends.rs agents: does the video-master backend have any shipped consumer, or is it another arm_governor-style unused surface? (apps/cli uses audio_master + system_monotonic only, per pipeline_host.rs:1605/1630.)
