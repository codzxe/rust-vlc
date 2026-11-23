# AUDIT: crates/renderer-wgpu/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 141 / crate root: 10 module decls + re-exports + 1 doctest |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — the crate is a dependency of apps/cli and apps/desktop; its re-exports feed the CLI present path (pipeline_host.rs:79-83) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
An unusually honest crate root: the headline doc states plainly that **v0 does not import wgpu** ("the crate is *named* renderer-wgpu because doc 01 names it so"), gives four evidence-backed reasons (no GPU stack in the dev sandbox, deny.toml's libloading ban vs wgpu's Linux graph, ungranted dependency envelope, gate economics), and enumerates what is NOT implemented (GPU YUV conversion, windowed surfaces, render-thread mailbox, subtitle compositing). Every factual claim I could check holds: Cargo.toml deps are exactly media-types + video-engine (std-only, zero external crates), the software compositor + typed wgpu seams are what actually ship, and the doctest (the four-call acceptance path incl. PNG magic/IEND assertions) passed in the orchestrator baseline. No findings.

## 2. What actually works (verified)
- Baseline (not re-run — doctests need cargo): `audit/runs/baseline/cargo-test-workspace.txt:5300-5306` → `Doc-tests renderer_wgpu ... 1 passed; 0 failed`, package exit 0. The lib unit tests: my live run earlier (60/60, includes this crate's modules).
- `crates/renderer-wgpu/Cargo.toml` [dependencies] = media-types + video-engine only → the "std-only by C5 / no external crates" claim is TRUE.
- The "no GPU stack" environment claim matches this sandbox's reality (no /dev/dri — independently confirmed during the VAAPI session audit).

## 3. Findings
None. (Checked: module list vs files on disk — all 10 modules exist (backend, color, error, negotiate, params, renderer, screenshot, staging, surface, synthetic); re-export names vs definitions — all resolve (crate compiles + tests green); doc's four wgpu-deferral reasons vs Cargo.toml/deny-record — consistent; the doctest's claims (backend_name()=="software", PNG header/IEND) — baseline-proven; "Never: unsafe" — no unsafe in the audited files of this crate (staging/error/params/surface paths read; rg unsafe in crate left to remaining file owners).)

## 4. Per-function table
Crate root — no functions. Modules re-exported are individually audited (staging.rs, error.rs done by me; backend_fallback.rs test suite done; negotiate/renderer/surface/screenshot/color/synthetic are other agents' or unassigned).

## 5. Test honesty
The doctest is the acceptance path (construct → present → readback → PNG encode with byte-level magic/IEND checks) — it would fail if present/readback/encode broke. Baseline-proven.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v0 does not import wgpu" + 4 reasons | lib.rs:18-46 | TRUE | Cargo.toml deps; records/wgpu-30.0.1.md; sandbox reality |
| "the software compositor ... is linked and carries the acceptance proof" | :48-51 | TRUE | backend_fallback suite 5/5 live |
| "v0 uploads RGBA8/Bgra8 and refuses the YUV family with a typed error" | :62-64 | TRUE | staging.rs audit (yuv typed-rejection test, ran live) |
| "std-only by C5 until the Lead-gated wgpu import" | :117-118 | TRUE | Cargo.toml |
| GPU/YUV-conversion "converts when that card lands" (re: MJPEG I420) | :64-65 | note | the CLI host converts I420→RGBA upstream of the renderer (pipeline_host.rs color-conversion seam) — renderer-side refusal is accurate; no doc lie |

## 7. Dead / duplicated / unreachable code
None at the root. (The wgpu-shaped typed seams — PresentMode::wgpu_present_mode etc. — are shape-pinned placeholders by design and disclosed as such.)

## 8. Dependencies & flags
media-types + video-engine (path deps), testkit/media-clock dev-only. No wgpu (Lead-gated). Unconditional in the shipped graph.

## 9. Open questions for the next agent
- None for this file. negotiate.rs and renderer.rs owners: my staging/error audits found cross-file stage-tag and saturation-semantics notes relevant to you.
