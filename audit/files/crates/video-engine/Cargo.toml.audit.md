# AUDIT: crates/video-engine/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W03-A02 |
| Date | 2026-11-23 |
| Lines / functions | 32 / 1 package manifest |
| File verdict | REAL (manifest matches every claim I could execute or cross-check; no drift) |
| Reachable from shipped entry point? | YES — video-engine is a release-lane dependency of lumen-cli (apps/cli/Cargo.toml `video-engine = { path = ... }`; used at apps/cli/src/pipeline_host.rs:84) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (zero-findings disclosure in §3) |
| Commands run | `rg -c unsafe crates/video-engine/src/` → 2 hits, both comment text (adjust.rs:36 doc line, lib.rs:173 doc line) — no unsafe blocks; `rg -A5 '[crates.video-engine]' boundaries.toml` → mirrors the manifest exactly |

## 1. One-paragraph verdict
A minimal, honest manifest: two workspace-internal deps (media-types, media-clock), one dev-dep (testkit), workspace lints, publish = false, no features section. Every concrete claim in the header comment checks out: "zero external dependencies" (both deps are path deps — TRUE), "no unsafe" (the only two `unsafe` tokens in src/ are inside doc comments — verified), "no GPU/render APIs" (no renderer/wgpu edge; renderer-wgpu depends DOWNWARD on this crate, not vice versa), and the boundaries.toml entry mirrors the manifest exactly ([crates.video-engine]: layer 3, internal_deps [media-types, media-clock], dev_deps [testkit], external_allowlist []). The disclosed divergence from the doc 01 §2.2 registry (plugin-api listed there but not needed until the video-filter chain) is legal under the allowlist rule and stated in the comment itself.

## 2. What actually works (verified)
- Boundaries mirror: boundaries.toml:342–347 `internal_deps = ["media-types", "media-clock"]`, `dev_deps = ["testkit"]`, `external_allowlist = []` — byte-for-byte the manifest's dep sets.
- "no unsafe": `rg -c unsafe crates/video-engine/src/` → adjust.rs:1 (a doc line listing what the module avoids: "Allocation per pixel, panics, `unsafe`.") and lib.rs:1 (the crate doc's "dependencies, no unsafe.") — zero unsafe blocks.
- Shipped reachability: the crate's `PresentationScheduler` is constructed in the release-lane CLI (pipeline_host.rs:1324 — verified in the scheduling.rs audit).

## 3. Findings
None. What was checked and how (zero-findings disclosure per spec §12):
- Dep sets vs boundaries.toml allowlist — exact match (command above).
- External-dependency claim — [dependencies] contains only path deps; TRUE.
- no-unsafe claim — rg over src/, only comment-text hits.
- Unused deps — media-types and media-clock both imported by src (scheduler/queue/sink use MediaClock/PlaybackClock/media_types types; the scheduling.rs test audit confirmed the imports); testkit used by tests (Must).
- `[lints] workspace = true` present (the workspace inheritance pattern; clippy baseline CLEAN).
- No features section — nothing feature-gated, so nothing UNREACHABLE-by-flag in this crate.
- Registry divergence (plugin-api absent vs doc 01 §2.2) — disclosed in the manifest comment with the allowlist justification; boundaries.toml agrees with the manifest.

## 4. Per-function table
N/A — config manifest (single [package] + [dependencies] + [dev-dependencies] + [lints] blocks; every key checked in §3).

## 5. Test honesty
N/A — no tests in this file. (The crate's tests were audited separately: tests/scheduling.rs — 21/21 executed pass.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only by C5: zero external dependencies, no unsafe, no GPU/render APIs" | video-engine/Cargo.toml:11–12 | REAL | path-only deps; rg unsafe → comments only; no renderer dep edge |
| "The boundary manifest is an allowlist, so the narrower dep list is legal (boundaries.toml mirrors it exactly)" | video-engine/Cargo.toml:8–9 | REAL | boundaries.toml:342–347 exact mirror (verified) |
| "plugin-api joins with the video-filter chain (a later card)" | video-engine/Cargo.toml:6–7 | REAL-UNPROVEN | consistent with the absence of any plugin-api usage in src; the filter chain's landing is future work |

## 7. Dead / duplicated / unreachable code
- None (no unused deps, no feature gates).

## 8. Dependencies & flags
- media-types (L0), media-clock (L2) — layering downward, consistent with boundaries.toml's "Edges point downward: L3 → L2 (media-clock) → L0 (media-types)". dev testkit. No features, no external deps.

## 9. Open questions for the next agent
- None for this file. The crate's src (scheduler.rs/queue.rs/policy.rs/sink.rs/adjust.rs) belongs to another agent; my test-file audits (scheduling.rs) defer implementation semantics there.
