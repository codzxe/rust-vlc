# AUDIT: crates/video-engine/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-21 |
| Lines / functions | 32 / config keys: package, lints, dependencies×2, dev-dependencies×1 |
| File verdict | REAL (cleanest manifest audited so far; zero defects found in the file itself) |
| Reachable from shipped entry point? | YES (call chain: shipped `lumen` bin → apps/cli/src/pipeline_host.rs (video_engine types; renderer-wgpu re-exports `Presenter` from it, renderer-wgpu/src/lib.rs:141) → baseline AVI smoke `v_decoded=60` exercised the present path this crate schedules) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | boundaries.toml:342-347 diff (exact match); `rg -l '<crate>::'` per dep (media_types 6 src/4 tests, media_clock 6/3, testkit in cfg(test)-gated modules only); `grep -n cfg(test)` (adjust.rs:196, lib.rs:181, policy.rs:466, queue.rs:234, scheduler.rs:1085, test_support.rs:1 header); `rg AdjustParams` outside crate → 0 consumers; lib.rs + adjust.rs + module-map read; docs/feature-matrix.toml:1840-1846 |

## 1. One-paragraph verdict
Zero findings — and I tried. Two internal deps, one dev-dep, no externals: every claim in the header survives tracing. The registry row it cites (doc 01 §2.2, "deps: media-types, media-clock, plugin-api") is narrower in reality, and the header discloses exactly that with the allowlist rationale; boundaries.toml:342-347 mirrors the manifest field-for-field; C5 std-only is true; the testkit dev-dep is provably test-scoped (all five src-file usages sit behind `#[cfg(test)]`); both deps are used in six src files each; and the crate runs in the shipped binary. The notable cross-file facts (adjust.rs has zero consumers outside the crate; the "video filter chain" remains unbuilt while its first filter's parameter model is merged) belong to source/doc agents and are recorded below.

## 2. What actually works (verified)
— Boundaries match: deps {media-types, media-clock} == boundaries.toml:344; dev {testkit} == :345; external_allowlist [] == no externals.
— Dep usage: media_types in 6 src files, media_clock in 6 src files; both genuinely imported (scheduler, queue, policy, sink, adjust, test_support).
— Dev-dep discipline: testkit appears in 5 src files, every one `#[cfg(test)]`-gated (grep list above); test_support.rs:1-2 documents the ARCH-7 no-leak rule.
— Description accuracy: "PTS-vs-clock frame scheduling" = scheduler.rs `PresentationScheduler`; "late-frame drop policy" = policy.rs `DropPolicy`; "bounded in-order presentation queue" = queue.rs `PresentationQueue` — all real modules in the module map (lib.rs:56-63).
— Execution: present path exercised in baseline AVI first-vertical (`v_decoded=60`, INDEX MP4 §contrast).

## 3. Findings
None. What was checked, exhaustively: name/version/edition/publish fields (consistent with workspace norms); `[lints] workspace = true` present and inherited (file-1 loop-grep); every dependency for (a) boundaries registration, (b) actual use, (c) layer direction (L3→L2 media-clock, L3→L0 media-types — both downward, legal); dev-dep for cfg(test) containment; header claims against src reality (std-only: no GPU/window/surface imports exist in src — module list is adjust/policy/queue/scheduler/sink/test_support only); registry-row dep-list deviation (disclosed in header, see §6).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` | 14-19 | REAL | — | Name accurate (this IS the video engine's scheduling core); description makes no filter claim — correctly narrower than registry row. |
| `[lints] workspace = true` | 21-22 | REAL | — | Inherited (verified file-1). |
| `media-types` | 25 | REAL | — | Used 6 src files; L0 downward edge. |
| `media-clock` | 26 | REAL | — | Used 6 src files; L2 downward edge. |
| `[dev] testkit` | 32 | REAL | — | cfg(test)-contained (grep proof above). |

## 5. Test honesty
No tests in manifest. The crate's acceptance harness is "deterministic, explicitly stepped core … injected virtual time with zero sleeps" (lib.rs:29-31) — virtual-clock tests, included in baseline #3. Per-test honesty: source agents.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Registry: doc 01 §2.2 (`video-engine`, layer L3, deps: media-types, media-clock, plugin-api)" | Cargo.toml:4-5 | REAL for this manifest / PAPER-ONLY for the plugin-api edge | plugin-api: 0 src/test usages, not a dep, no filter chain exists (lib.rs:19-22 "a later card; the pipeline seam is reserved"). |
| "plugin-api joins with the video-filter chain (a later card)" | Cargo.toml:8-9 | REAL (as a promise, honestly labeled future) | lib.rs:19-25 same statement; adjust.rs is the adjust filter's parameter model only, not the chain. |
| "Std-only by C5: zero external dependencies, no unsafe, no GPU/render APIs" | Cargo.toml:11-12 | REAL | No externals in manifest; src module list has no GPU/surface code; clippy-clean baseline #5 with unsafe_code=deny (root Cargo.toml:54). |
| matrix row `vfeat-colors-adjust` status partial: "parameter model + CPU reference only … UI sliders render inert-with-reason until the command lands" | docs/feature-matrix.toml:1845-1846 | REAL (honest partial) | `rg AdjustParams` outside crates/video-engine → 0 hits in apps/ and crates/ — nothing consumes the model at HEAD; the matrix admits it. |

## 7. Dead / duplicated / unreachable code
In the manifest: none. Cross-file note for source agents: `adjust.rs` (adjust filter parameter model, issue #33/D-012) has zero consumers outside the crate — no command, no UI, no renderer uniform wiring (the "production path is a renderer-wgpu shader" — a renderer that currently contains no wgpu at all, see renderer-wgpu audit F-001). Wired-to-nothing model at HEAD; matrix honestly marks it partial.

## 8. Dependencies & flags
No `[features]`, no build-deps, no externals. Minimal and clean.

## 9. Open questions for the next agent
- Who owns the `video.set-adjust` catalog command (D-012, "pending" per matrix row) — if it never lands, adjust.rs is a permanently orphaned parameter model compiled into every build.
- lib.rs:13-14 calls renderer-wgpu "the 2.D1 wgpu renderer" — the naming lie propagates into this crate's docs (cross-ref renderer-wgpu audit F-001).
- DropPolicy::Auto deviates from doc 03 §4.10 b by resolving to Late at the type level (lib.rs:66-70 "Deviations from the doc sketches") — deviation is recorded, Lead review pending; doc-03 agent should track ratification.
