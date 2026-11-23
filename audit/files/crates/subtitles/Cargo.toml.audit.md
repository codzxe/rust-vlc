# AUDIT: crates/subtitles/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-23 |
| Lines / functions | 54 / 5 manifest sections |
| File verdict | REAL (one stale scaffold comment contradicting the actual dev-dependencies; dep table matches boundaries.toml exactly) |
| Reachable from shipped entry point? | The crate it defines is NOT a dependency of any shipped app (only fuzz/Cargo.toml:50) — the manifest is honest about its own deps but the crate is shipped-dead. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | rg dep usage across src/ and tests/ (exit 0); sed/rg boundaries.toml (exit 0); rg cfg(feature) (0 hits) |

## 1. One-paragraph verdict
A minimal, honest manifest: exactly one lib dependency (`media-types`, path), two dev dependencies (`testkit`, `media-clock`, path), zero external crates, no features, no unsafe, workspace lints inherited — and the table matches `boundaries.toml`'s `[crates.subtitles]` row field-for-field. Both dev-deps are real and used (testkit in 8 test files, media-clock in exactly one — `timing_clock.rs` — honoring the documented dev-only edge). The defects are comment-level: the 5.L2 scaffold note still claims "No dev-dependencies either" while the 5.L1 block two sections later declares two of them (a merge artifact, an internal contradiction), and the package description still describes only the 5.L1 compositing half of a crate that now hosts seven format parsers.

## 2. What actually works (verified)
- Dep-usage scan: `media_types` imported in 8 src files (ass/render.rs:66, dvb/mod.rs, pgs/mod.rs, vobsub/mod.rs, cc/render.rs, compositing/{bitmap,compose,target}.rs); `testkit` in 8 test files; `media_clock` in tests/timing_clock.rs only — no unused dependencies, dev edges do not leak into lib (rg over src/ for media_clock: 0 hits).
- boundaries.toml:259-264 matches: `internal_deps = ["media-types"]`, `dev_deps = ["testkit", "media-clock"]`, `external_allowlist = []`, `external_dev_allowlist = []`.
- `rg "cfg\(feature"` over src/: 0 hits — nothing feature-gated, so the release lane compiles everything (no UNREACHABLE-by-feature code in this crate).
- Baseline: crate compiles clean (cargo-test-workspace.txt:5735-5737, "Finished `test` profile ... in 3.67s") and clippy CLEAN (cargo-clippy.txt:90).

## 3. Findings
F-001 [S4]
Where: crates/subtitles/Cargo.toml:27-30 vs 46-53
Code:
```
# Std-only by design (doc 11: zero external dependencies — the model is
# pure data + pure functions; any future dependency needs the doc-11
# audit). No dev-dependencies either: the roundtrip/fuzz suites are
# std-only and deterministic.
```
What is wrong: this 5.L2 scaffold note says "No dev-dependencies either", but the manifest declares two (`testkit`, `media-clock` at L49/L53) and the 5.L1 comment block (L46-53) explains them. The two worktree scaffold notes contradict each other — a merge artifact left in tree. Confidence: HIGH — Verification: read. Fix: drop the stale sentence.

F-002 [S4]
Where: crates/subtitles/Cargo.toml:36
Code:
```
description = "Lumen subtitle compositing contract: safe-area placement, scaling with the video extent, z-ordered alpha blending onto a renderer-implemented ComposeTarget, and sample-exact [t0,t1) cue timing against the shared clock (doc 04 §5 compositing contract; card 5.L1)."
```
What is wrong: the description covers only the 5.L1 compositing half; the crate now ships ass/srt/vtt/dvb/vobsub/cc/pgs parser modules (lib.rs:74-145). `cargo search`/registry metadata and any tooling reading the description get the 2025-era scope. Confidence: HIGH — Verification: read (lib.rs module list).

F-003 [S4]
Where: crates/subtitles/Cargo.toml:11-13
Code:
```
# Deps (5.L1 half): media-types only — the registry row also lists
# plugin-api, which joins when the SubtitleParser seam lands (5.M1);
# the manifest is an allowlist, missing allowed-edges are fine.
```
What is wrong: the "plugin-api joins when the SubtitleParser seam lands (5.M1)" — 5.M1 never landed (no `SubtitleParser` trait in lib.rs; rg: 0 hits), so the promised seam is vaporware the comment still describes as pending. Minor staleness. Confidence: HIGH — Verification: rg SubtitleParser (0 hits in crate).

## 4. Per-function table (manifest sections)
| Section | Lines | Verdict | Notes |
|---|---|---|---|
| header comments (×3 scaffold blocks) | 1-31 | STALE in parts | F-001, F-003 |
| `[package]` | 32-38 | REAL | F-002 description; edition 2024; publish=false |
| `[lints]` | 40-41 | REAL | workspace = true (unwrap_used deny etc. inherited — crate has zero unwrap in src, verified by clippy CLEAN) |
| `[dependencies]` | 43-44 | REAL | media-types only, used |
| `[dev-dependencies]` | 46-53 | REAL | testkit + media-clock, both used, media-clock dev-only honored |

## 5. Test honesty
N/A (manifest).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "media-types only" lib dep | L11, L44 | REAL | Cargo.toml:44 + 8 src importers |
| "media-clock is DEV-ONLY ... a dev edge cannot leak into the public API" | L14-17 | REAL | rg media_clock in src/: 0 hits; only tests/timing_clock.rs |
| "Std-only by C5 / zero external dependencies" | L27-28 | REAL | no external crates in any dep table |
| "No dev-dependencies either" | L29-30 | FALSE | [dev-dependencies] at L46-53 (F-001) |
| "plugin-api ... joins when the SubtitleParser seam lands (5.M1)" | L12-13 | PAPER-ONLY | seam never landed (rg: 0 hits) |
| boundaries.toml registration | L9, L25 | REAL | boundaries.toml:259-264 matches exactly |

## 7. Dead / duplicated / unreachable code
No dead deps. Crate-level note: nothing in `apps/*` depends on `subtitles` — the manifest is fine but the crate it defines is unreachable from every shipped entry point (see parser.rs audit for the full chain).

## 8. Dependencies & flags
media-types (path, L0, std-only), testkit (dev, path), media-clock (dev, path). No features, no bins, no build.rs. Workspace lints inherited (unsafe_code/unwrap_used deny).

## 9. Open questions for the next agent
- If/when the crate gets wired into playback (the "Phase-5 card" the CLI dispatch mentions), the dependency direction will need a boundaries.toml edit + a `HostDecoderProvider` subtitle arm — none exists today.
