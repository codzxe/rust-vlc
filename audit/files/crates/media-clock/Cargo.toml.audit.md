# AUDIT: crates/media-clock/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 26 / 1 package manifest |
| File verdict | REAL (manifest matches every claim I could execute or cross-check; no drift) |
| Reachable from shipped entry point? | YES — media-clock is a release-lane dependency of lumen-cli (apps/cli/Cargo.toml; `MediaClock::audio_master` constructed at apps/cli/src/pipeline_host.rs:1605, `system_monotonic` at :1630) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (zero-findings disclosure in §3) |
| Commands run | `rg -c unsafe crates/media-clock/src/` → no hits; `rg -A5 '[crates.media-clock]' boundaries.toml` → mirrors the manifest exactly |

## 1. One-paragraph verdict
The smallest honest manifest in my set: one workspace-internal dep (media-types), one dev-dep (testkit), workspace lints, publish = false, no features. The header's claims are all verifiable and verified: "zero external dependencies" (single path dep), "no unsafe" (zero `unsafe` tokens in src/), and the boundaries.toml entry mirrors it exactly ([crates.media-clock]: layer 2, internal_deps [media-types], dev_deps [testkit], external_allowlist []). The package description's feature list (master-loss fallback, Ended seek-back re-arm) is not marketing: both behaviors are exercised by executed tests in this crate (tests/zero_alloc_backends.rs — my audit — drives the loss chain to FreeRunning and the Ended→seek_back→Paused re-arm, 1/1 pass).

## 2. What actually works (verified)
- Boundaries mirror: boundaries.toml:172–176 `internal_deps = ["media-types"]`, `dev_deps = ["testkit"]`, `external_allowlist = []` — exactly the manifest's dep sets.
- "no unsafe": `rg -c unsafe crates/media-clock/src/` → no matches at all.
- Shipped reachability: the CLI constructs this crate's clock in both lanes (audio-master at pipeline_host.rs:1605, system-monotonic fallback at :1630).
- Description claims ↔ executed evidence: the zero-alloc test's state pins (`FreeRunning` after the three `on_master_lost` calls; `ClockState::Ended` then `Paused` from `seek_back`) are the description's "deterministic master-loss fallback and an Ended seek-back re-arm" — verified by execution in my zero_alloc_backends.rs audit.

## 3. Findings
None. What was checked and how (zero-findings disclosure per spec §12):
- Dep sets vs boundaries.toml — exact match (commands above).
- External-dependency claim — [dependencies] is a single path dep; TRUE.
- no-unsafe claim — rg over src/, zero hits.
- Unused deps — media-types imported throughout src (Timestamp/Rate/NANO_BASE surface used by every consumer I audited); testkit used by tests.
- `[lints] workspace = true` present.
- No features section — nothing feature-gated in this crate, so no UNREACHABLE-by-flag code.
- Layering: L2 depending only on L0 (media-types) — consistent with boundaries.toml's downward-edges rule; the clock "observes the audio device through the injected SampleCounterSource trait" (no audio-output dep — the trait lives in-crate per the test's `impl media_clock::SampleCounterSource`).

## 4. Per-function table
N/A — config manifest (single [package] + [dependencies] + [dev-dependencies] + [lints] blocks; every key checked in §3).

## 5. Test honesty
N/A — no tests in this file. (tests/zero_alloc_backends.rs audited separately: 1/1 executed pass.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only by C5: zero external dependencies, no unsafe" | media-clock/Cargo.toml:9–10 | REAL | path-only dep; rg unsafe → 0 hits |
| "it never drives [the audio device], and never touches I/O, device APIs or UI" | media-clock/Cargo.toml:6–8 | REAL | no I/O or device crates in the dep set (a manifest-level proof: nothing to touch them with) |
| Description: "deterministic master-loss fallback and an Ended seek-back re-arm" | media-clock/Cargo.toml:17–18 | REAL | exercised and executed in tests/zero_alloc_backends.rs (my audit, 1/1 pass) |

## 7. Dead / duplicated / unreachable code
- None.

## 8. Dependencies & flags
- media-types (L0) only; dev testkit. No features, no external deps. Workspace member via crates/* glob.

## 9. Open questions for the next agent
- None for this file. crates/media-clock/src (the backends implementation) is another agent's; my test audit's mechanism questions (8-slot ring, Copy outcomes) route there.
