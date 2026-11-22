# AUDIT: crates/video-engine/Cargo.toml

| Field | Value |
|---|---|
| Auditor | W03-A03 |
| Date | 2026-11-22 |
| Lines / functions | 32 / manifest: package metadata, lints, 2 deps, 1 dev-dep |
| File verdict | **REAL** — every comment claim verified |
| Reachable from shipped entry point? | YES: `video-engine` is a dependency of apps/cli (Cargo.toml:63), apps/web (Cargo.toml:56 area), apps/desktop (Cargo.toml:43) — the scheduler runs in the shipped CLI path. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg (`unsafe \{` over crates/video-engine/src — 0 hits; `video-engine` in apps/*/Cargo.toml — 3 hits); ls of src/ and tests/. |

## 1. One-paragraph verdict

A manifest whose comment block makes four checkable claims, and all four hold: (1) "zero external dependencies" — `[dependencies]` is exactly `media-types` + `media-clock`, both workspace path deps; (2) "no unsafe" — rg `unsafe {` over crates/video-engine/src returns nothing; (3) "no GPU/render APIs … those belong to renderer-wgpu" — the dep list contains no wgpu/gfx crates and src/ holds scheduler/queue/policy/sink modules only; (4) testkit is dev-only — it sits under `[dev-dependencies]`. `publish = false`, `[lints] workspace = true`. Nothing to flag.

## 2. What was checked (per §13 — no clean bill without proof)

- Dependency allowlist vs the comment's claim: `media-types`, `media-clock` only — matches "needs media-types + media-clock only" (lines 4-10). ✓
- "Std-only … zero external dependencies, no unsafe" (11-12): rg `unsafe \{` in crates/video-engine/src → 0; no external crates in the manifest. ✓
- "no GPU/render APIs" (11-12): no renderer/wgpu dependency; crates/renderer-wgpu exists separately (L4), consistent. ✓
- Dev-dep discipline (29-31): `testkit` under `[dev-dependencies]` only. ✓
- Crate reachability: consumed by all three shipped app crates (apps/cli:63, apps/web, apps/desktop:43). ✓
- Module list sanity: src/ = adjust, lib, policy, queue, scheduler, sink, test_support; tests/ = common, scheduling, sync_scenarios, zero_alloc_scheduler — matches the description "PTS-vs-clock frame scheduler, late-frame drop policy, bounded presentation queue". ✓

## 3. Findings

None. All five comment claims verified true; metadata (edition 2024, publish=false, workspace lints) consistent with the workspace's other manifests.

## 4-9. Not applicable

No functions, no tests in this file, no doc claims beyond the verified comment block, no dead code, no flags beyond `[lints]`, no open questions — except one cross-reference: adjust.rs (this agent's audit) is a member of this crate and is unreachable; that is adjust.rs's finding, not the manifest's (the manifest does not advertise adjust).

Per-function table: N/A (manifest). Items audited: package block (14-19) ✓, lints (21-22) ✓, dependencies (24-26) ✓, dev-dependencies (28-32) ✓, comment claims (1-12) ✓ (all verified, §2).
