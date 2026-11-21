# AUDIT: crates/renderer-wgpu/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A01 |
| Date | 2026-11-21 |
| Lines / functions | 41 / config keys: package, lints, dependencies×2, dev-dependencies×2 |
| File verdict | REAL (software-compositor crate executed by shipped binary; name misleads) |
| Reachable from shipped entry point? | YES (call chain: shipped `lumen` bin → apps/cli/src/pipeline_host.rs:79-83,885 `use renderer_wgpu::{color, params, renderer, screenshot, surface, staging}` → baseline engine-info `features: … video.software-compositor` and AVI smoke `v_decoded=60` — the present path ran) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `ls docs/dependencies/records/` (wgpu-30.0.1.md exists — the cited deferral record); boundaries.toml:433-438 diff (exact match); `rg -l 'renderer_wgpu' apps/cli/src` (pipeline_host.rs); `rg -n 'renderer_wgpu::synthetic'` repo-wide (only apps/desktop tests/examples); media_clock dev usage (0 src / 1 test); lib.rs pub-mod list; surface.rs PresentMode read; baseline #20 + MP4/AVI smoke |

## 1. One-paragraph verdict
The smallest manifest so far — two internal deps, two dev-deps, zero externals — and the only one whose crate name is false advertising: "renderer-wgpu" contains no wgpu at all. The wgpu import is deferred behind a real, detailed conditional decision record (docs/dependencies/records/wgpu-30.0.1.md: no GPU stack in the sandbox — live `request_adapter` probe returning `No suitable graphics adapter found`; `libloading` banned from the default closure), the crate ships a CPU point-sampling `SoftwareCompositor` (backend.rs), and the shipped binary runs exactly that (engine-info `video.software-compositor`). Everything the manifest asserts checks out — deps match boundaries.toml:433-438 field-for-field, dev-dep discipline holds (media-clock: tests-only), the record it cites exists — but the name and the registry row ("the only GPU citizen") describe hardware that does not exist in this codebase.

## 2. What actually works (verified)
— Manifest/boundaries/lock coherence: deps {media-types, video-engine} == boundaries.toml:435; dev {testkit, media-clock} == :436; external_allowlist [] == no externals; C5 std-only claim true.
— The cited deferral record exists and is substantive: docs/dependencies/records/wgpu-30.0.1.md (class "deferral decision record", verdict "conditional (E2), NOT admitted in 2.D1", environment probes + crates.io API + throwaway-project adapter probe quoted).
— Execution: baseline AVI first-vertical `v_frames=60 v_decoded=60` and `features: video.software-compositor` — the software path this manifest links ran in the shipped binary.
— Dev-dep honesty: `media_clock::` appears in 0 src files, 1 test file (tests/presenter_seam.rs family) — the documented TEST-ONLY edge (:35-40) is real.
— The "typed + shape-tested" claim has concrete referents: surface.rs:31-44 `PresentMode` "mirrors wgpu's vocabulary: Fifo (vsync, always supported), Mailbox, Immediate"; lib.rs:131-141 exports the full seam (Compositor, Renderer, StagingUploader, HeadlessSurface, PresentMode…).

## 3. Findings
### F-001 [S3]
Where: crates/renderer-wgpu/Cargo.toml:18 (package name) vs :27-29 (dependency table)
Code:
```
[package]
name = "renderer-wgpu"
```
What is wrong: The name promises wgpu; the dependency table contains zero GPU crates — no wgpu, no ash, no vulkan, nothing. The GPU compositor is "owed to the Lead-gated import" (backend.rs:16-20). The registry row cited in this very header (doc 01 §2.6) calls the crate "the only GPU citizen" — there are no GPU citizens in the workspace.
Why it matters: Misleading name (the severity scale's own S3 example). Every consumer of the workspace graph, every doc reader, and every packager reading `renderer-wgpu` in a dep list infers hardware rendering; the actual artifact is a CPU rasterizer with nearest-neighbor sampling and deliberately absent bilinear filtering (backend.rs:22-27). The deferral is honestly recorded — the name is still a lie.
What correct looks like: Rename to `renderer` (or `renderer-present`) now; re-take the wgpu name if/when the import is ratified and the GPU backend lands. Alternatively a one-line manifest header stating "no wgpu in this crate — see records/wgpu-30.0.1.md" (the current header does disclose this, but the package NAME cannot be un-read).
Confidence: HIGH — Verification: read (manifest + record + backend.rs + surface.rs).

### F-002 [S4]
Where: crates/renderer-wgpu/Cargo.toml:31-41 (dev-deps block) vs src/synthetic.rs (production module)
Code:
```
[dev-dependencies]
# testkit is dev-only by contract (MP §3.7 / ARCH-7): the lint-legal
# `Must` helper for fixture contracts in tests and doctests.
```
What is wrong: While the dev-deps block is disciplined, the crate's deterministic test-frame generator (`src/synthetic.rs`, "the card 2.D1 AC generator") is a `pub mod` in the production library (lib.rs:129), its only consumers being apps/desktop tests/examples (`rg renderer_wgpu::synthetic` → apps/desktop/examples/emit_stream.rs:29, apps/desktop/tests/common/mod.rs:31) — fixture machinery compiled into every production link of the crate.
Why it matters: Fixtures baked into production code (checklist item); dead weight in the shipped `lumen` binary's dependency graph (the CLI never calls `synthetic` — its own "synthetic" hits are unrelated timestamp/PS-lane comments, verified).
What correct looks like: `synthetic` behind a `test-fixtures` cargo feature enabled only by dev consumers, or moved to testkit.
Confidence: HIGH — Verification: read (lib.rs:129 + repo-wide rg).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package] name="renderer-wgpu"` | 17-22 | REAL / MISLEADING NAME | S3 (F-001) | Description (:21) accurately says "turns PresentRequests into pixels" but the name says wgpu; description's "sRGB present, vsync configuration" map to real modules (color.rs passthrough, surface.rs PresentMode) — unproven at runtime here, source agents' scope. |
| `[lints] workspace = true` | 24-25 | REAL | — | Verified inherited (file-1 loop-grep). |
| `media-types` | 28 | REAL | — | Used (synthetic.rs, color.rs etc. import media_types). |
| `video-engine` | 29 | REAL | — | Used; Presenter/PresentRequest re-exported from it (lib.rs:141) — the L4→L4-adjacent contract edge, registered. |
| `[dev] testkit` | 34 | REAL | — | Used by tests incl. the counting-allocator zero-alloc proof (tests/zero_alloc_present.rs:22-23 `CountingGlobalAlloc`). |
| `[dev] media-clock` | 41 | REAL | — | TEST-ONLY verified (0 src hits); presenter-seam suite constructs `Arc<dyn PlaybackClock>`. |

## 5. Test honesty
No tests in the manifest. tests/ contains backend_fallback.rs, png_hash.rs, presenter_seam.rs, zero_alloc_present.rs, common/ — zero_alloc_present.rs is a genuine process-global counting-allocator proof (head read: `#[global_allocator] static ALLOC: CountingGlobalAlloc`), and its header admits the history: "staging.rs claimed a 'counting-allocator proof binary' that did not exist — this is that binary" — a prior paper claim, now materialized. Included in baseline #3.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the wgpu import is a recorded Lead decision … the deferral record with the evidence" | Cargo.toml:5-7 | REAL | docs/dependencies/records/wgpu-30.0.1.md exists, 60+ lines, class deferral/E2, probes quoted. |
| "the software compositor linked … every wgpu integration point typed + shape-tested" | Cargo.toml:11-15 | REAL (shape) / REAL-UNPROVEN (behavior) | backend.rs SoftwareCompositor; surface.rs:15 "shape-pins the wgpu" PresentMode vocabulary; module list lib.rs:120-141. Behavior proofs belong to the crate's test suite (baseline #3). |
| registry row "the only GPU citizen" (cited at :2-3) | doc 01 §2.6 via Cargo.toml:3 | FALSE (at HEAD) | Zero GPU crates in the workspace dependency graph (this manifest has none; no other crate admits wgpu — records list). |

## 7. Dead / duplicated / unreachable code
`synthetic` pub module: production-compiled, consumed only by apps/desktop test/example code (F-002). Everything else in the crate is reachable from the shipped CLI present path.

## 8. Dependencies & flags
No `[features]` — the wgpu backend cannot even be opt-in built (there is no feature to enable; the import simply does not exist). No build-deps. Two internal deps, both used; layer-4 downward edges legal.

## 9. Open questions for the next agent
- If the Lead ratifies the wgpu import, this manifest gains an external dep that deny.toml currently bans the closure of (libloading) — the record says a full R1–R10 verification is owed; who tracks that debt?
- The description claims "measured staging" — staging.rs StagingStats + the zero-alloc proof cover the software path; matrix rows claiming hardware video output must be checked by matrix agents against the missing GPU backend (parity-report web=0/252 pattern suggests claims vs reality gaps elsewhere).
