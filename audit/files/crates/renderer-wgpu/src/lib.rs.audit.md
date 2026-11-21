# AUDIT: crates/renderer-wgpu/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-21 |
| Lines / functions | 141 / 12 module decls + 22 re-exports + 1 doctest (no functions — crate root) |
| File verdict | REAL (the crate-root's own doctest executed and passed in the baseline workspace run; every verifiable doc claim checks out) |
| Reachable from shipped entry point? | YES — `apps/cli/Cargo.toml` + `apps/cli/src/pipeline_host.rs` use renderer_wgpu (the CLI's video present path); also apps/web/src/render/*, apps/desktop. The shipped binary's lane is the **software compositor** (baseline `lumen engine-info`: `features: … video.software-compositor`). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cat` (full file); `rg -n "renderer_wgpu" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (doctest + unittests executed); `rg -c "unsafe" crates/renderer-wgpu/src/` (one hit = the word in this file's doc "Never" list — zero unsafe code); `rg -n "wgpu" src/*.rs` non-comment hits (only shape-pinning name helpers in surface.rs); `rg -n "\[dependencies\]" -A3 Cargo.toml` (media-types + video-engine only); `ls docs/dependencies/records/` (wgpu-30.0.1.md exists, 143 lines); `rg -n "Yuv\|yuv\|unsupported"` in error/renderer/staging (typed-refusal claim verified). |

## 1. One-paragraph verdict
This is a crate root whose entire job is honest framing — and it does that job:
it declares up front that **v0 imports no wgpu** (verified: zero wgpu in the
dependency graph, zero unsafe in the crate, the only `wgpu` strings are
shape-pinning helpers returning enum-variant names for the future backend),
documents the four verified reasons with a real decision record on disk, and
backs the core path with a doctest that the baseline actually executed
(present → byte-exact RGBA readback → PNG with magic/IEND assertions). The
re-export surface resolves (the doctest compiles against it). The YUV-refusal
and no-windowed-surface "not yet" claims are backed by typed errors and tests
in the child modules. The single finding is the inherited name lie: a crate
called renderer-wgpu that contains no wgpu — documented honestly here, but
the name still misleads every consumer-facing artifact.

## 2. What actually works (verified)
— Doctest executed: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:6507-6510 → `Doc-tests renderer_wgpu … test crates/renderer-wgpu/src/lib.rs - (line 81) ... ok` — the acceptance path (headless surface, present, readback, PNG encode) ran green.
— Unit tests executed: same log line 4766 (`Running unittests src/lib.rs (…/renderer_wgpu-…)`) — no renderer-wgpu failure among the 7.
— Dependency claims verified: `[dependencies]` = media-types + video-engine only (Cargo.toml:27-29) — "no external crates … std-only by C5" holds; the PNG encoder is in-crate (screenshot module).
— Zero unsafe in the crate: `rg -c "unsafe" src/` → exactly one hit, the word inside this file's doc "Never" list.
— YUV typed refusal exists: staging.rs:398 `yuv_formats_are_typed_rejections_pointing_at_the_shader_path` + renderer.rs:320-323 (Nv12 → `UploadFormatUnsupported`).

## 3. Findings

### F-001 [S4]
Where: crates/renderer-wgpu/src/lib.rs:15-18
Code:
```
//! The crate is *named* renderer-wgpu because doc 01 names it so; **v0
//! does not import wgpu**, for four verified reasons:
```
What is wrong: the crate's public name promises a wgpu renderer; the crate
ships a CPU rasterizer. The doc corrects the expectation in its first
paragraph (honestly, with a decision record), but the name still surfaces
misleadingly in every artifact that does not read this doc: Cargo.toml,
Cargo.lock, the doc 01 registry, and any matrix/UI text that says
"renderer-wgpu".
Why it matters: naming that lies is on the audit checklist; here the lie is
institutional (doc 01 mandates the name) and self-corrected in-situ — but a
user reading `engine-info` or the SBOM sees "renderer-wgpu" and reasonably
believes a GPU path exists. The shipped feature string
(`video.software-compositor`) is the honest counterweight.
What correct looks like: rename at the next breaking window (`renderer-core`
+ a future `renderer-wgpu` that actually imports it), or Lead-ratify the name
as forward-declared architecture in doc 01 with the deferral linked.
Confidence: HIGH — Verification: read + Cargo.toml + baseline engine-info.

## 4. Per-function table (crate root: modules, re-exports, doctest)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module decls (backend, color, error, negotiate, params, renderer, screenshot, staging, surface, synthetic) | 124-133 | REAL | — | all modules exist (doctest + unit tests exercise them) |
| re-export block | 135-141 | REAL | — | resolves — the baseline doctest compiled against `Renderer`, `screenshot`, `color::SurfaceFormat`, `params::Extent2D`, `surface::{PresentMode, SurfaceConfig}`, `synthetic::synthetic_frame` |
| `pub use video_engine::{PresentInfo, PresentRequest, Presenter}` | 141 | REAL | — | the seam re-export — consumers (apps/web, apps/desktop) confirmed via rg |
| doctest (acceptance path) | 81-119 | REAL | — | EXECUTED green in baseline (log line 6510); asserts PNG magic + IEND tail |
| module docs (wgpu deferral, "not yet" list, "Never" list) | 1-122 | REAL | — | every verifiable claim checked (§2); the four deferral reasons cite docs/dependencies/records/wgpu-30.0.1.md which exists (143 lines; its content is another agent's doc-audit scope) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| lib.rs doctest (line 81) | YES | asserts backend_name()=="software", present() info, byte-exact readback length/content, PNG magic 8 bytes + IEND tail — breaks on any upload/encode regression |
| (module tests audited in their own files by their owners — staging.rs/surface.rs/renderer.rs are not in my assignment) | — | — |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v0 does not import wgpu" | lib.rs:15-16 | REAL | Cargo.toml deps: media-types, video-engine only; no wgpu anywhere in the graph (Cargo.lock audited W01-A04) |
| "refuses the YUV family with a typed error naming this path" | lib.rs:91-93 | REAL | staging.rs:398 test + renderer.rs:320-323 Nv12 refusal |
| "no windowed surface creation … ships the headless surface" | lib.rs:97-99 | REAL | surface module's HeadlessSurface is the only surface ctor used in the doctest |
| "unsafe … Never" | lib.rs:121-122 | REAL | rg: zero unsafe in crates/renderer-wgpu/src/ |
| "deps `media-types` + `video-engine`" | lib.rs:4-5 | REAL | Cargo.toml:27-29 |
| sandbox GPU-stack absence + deny.toml libloading ban reasons | lib.rs:19-46 | REAL-UNPROVEN (environment claims) | decision record exists (docs/dependencies/records/wgpu-30.0.1.md); the record's probe outputs are its auditor's scope; cargo-deny bans-ok baseline (INDEX.md #25) corroborates the libloading-ban consistency |

## 7. Dead / duplicated / unreachable code
- None at the crate root. All re-exports have consumers (doctest, apps).

## 8. Dependencies & flags
- media-types + video-engine only; dev-deps testkit + media-clock (TEST-ONLY,
  documented in Cargo.toml:31-35). No features. The crate compiles into the
  shipped CLI (software-compositor lane).

## 9. Open questions for the next agent
- feature-matrix owner: does any row claim GPU/wgpu presentation as
  implemented? The crate's own honesty makes such a row the residual FALSE
  claim to hunt.
- surface.rs owner: `PresentMode::wgpu_present_mode()` returns variant NAME
  strings ("Fifo"/"Mailbox") — confirm the mapping stays in lockstep with the
  real wgpu enum when the Lead-gated import lands (the shape-pinning test at
  surface.rs:355 exists; it pins names, not enum identity).
