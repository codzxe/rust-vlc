# AUDIT: apps/desktop/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W13-A10 |
| Date | 2025-06-27 |
| Lines / functions | 54 / 5 manifest sections (package, lints, bin, dependencies, dev-dependencies) |
| File verdict | REAL — every header claim verified against the manifest and the source tree; one dependency-classification defect |
| Reachable from shipped entry point? | YES — this manifest builds the release-lane `lumen-desktop` binary (baseline cargo-build-release-desktop.txt: `Finished 'release' profile [optimized] target(s) in 1.62s`, exit 0). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg `renderer_wgpu|media_clock|video_engine|media_types` over apps/desktop/src + tests + examples; sed wire.rs:985-1010 + cfg(test) fence check (tests mod starts wire.rs:682-683); sed crates/renderer-wgpu/Cargo.toml (std-only, wgpu deferred); boundaries.toml `[crates.desktop]` section read |

## 1. One-paragraph verdict
The manifest matches its own story: no tauri/wry edge (verified — [dependencies] lists only five path deps), no registry deps at all ("std-only by C5" holds at the manifest level), `publish = false`, an honest "skeleton" description, and the `lumen-desktop` bin that the release lane builds. The dev-dependency discipline is real and documented (media-clock TEST-ONLY with the ARCH-7 rationale — and its only src usage is inside `#[cfg(test)] mod tests` at frame_tap.rs:136-139, correctly). The one defect: `renderer-wgpu` sits in [dependencies] and in boundaries.toml `internal_deps` as a PRODUCTION edge, but the crate's only usage of it is test code — wire.rs:999 (`renderer_wgpu::uploads_format(format)`) lives inside the tests module (wire.rs:682-683), and every other hit is in tests/. By the crate's own MP §3.7 / ARCH-7 rule (the one the media-clock comment enforces), it belongs in [dev-dependencies].

## 2. What actually works (verified)
— verified by rg + sed: (a) `[dependencies]` = commands, lumen-cli, media-types, renderer-wgpu, video-engine, all `path =` — zero registry edges, zero tauri/wry; (b) media-clock used only under cfg(test) (frame_tap.rs:136-139 inside the tests module; tests/common/mod.rs:22); (c) baseline release build of this manifest exits 0 (cargo-build-release-desktop.txt).

## 3. Findings
F-001 [S3]
Where: apps/desktop/Cargo.toml:42 and boundaries.toml:540 (`internal_deps = ["commands", "lumen-cli", "media-types", "renderer-wgpu", "video-engine"]`)
Code:
```
renderer-wgpu = { path = "../../crates/renderer-wgpu" }
```
What is wrong: `renderer-wgpu` is classified as a production dependency, but nothing under apps/desktop/src uses it outside `#[cfg(test)]` — the sole src hit is `renderer_wgpu::uploads_format(format)` at wire.rs:999, inside the tests module that starts at wire.rs:682-683; all other usage is tests/ (pipeline.rs, protocol.rs, common/mod.rs, zero_alloc_transport.rs). The crate's own discipline (Cargo.toml:45-48: "testkit is dev-only by contract (MP §3.7 / ARCH-7)"; the media-clock TEST-ONLY precedent at :49-54) is applied to testkit and media-clock but not to renderer-wgpu, and boundaries.toml's `internal_deps` inherits the misclassification. Why it matters: the shipped `lumen-desktop` binary links a renderer it never calls; the boundary record overstates the production edge set (the doc comment even justifies "L6 → L4 (renderer-wgpu ...)" as if it were a runtime edge). What correct looks like: move the edge to [dev-dependencies] and to boundaries.toml `dev_deps`. Confidence: HIGH — Verification: read + rg (every renderer_wgpu hit located; none in production code paths).

## 4. Per-function table (manifest sections)
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header comment | 1-16 | REAL | — | no-tauri/std-only claims verified against the manifest itself |
| [package] | 18-23 | REAL | — | name desktop, 0.1.3, publish=false, honest description |
| [lints] | 25-26 | REAL | — | workspace = true |
| [[bin]] | 28-30 | REAL | — | lumen-desktop → src/main.rs; release-lane built (baseline) |
| [dependencies] | 32-43 | REAL | S3 | five path deps; renderer-wgpu misclassified (F-001); "no engine crate is touched" comment (L38-39) is about the lumen-cli adoption not modifying engine crates — ambiguous phrasing given three crates/* edges sit right below it |
| [dev-dependencies] | 45-54 | REAL | — | testkit + media-clock, both genuinely test-only (verified) |

## 5. Test honesty
No tests in this file. The manifest's test surface (63 tests per INFRA-BLOCKED's "63/0 desktop") is exercised in the baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO tauri/wry import" | Cargo.toml:11 | REAL | [dependencies] has no tauri/wry; the tauri closure lives only in the excluded apps/desktop-tauri/Cargo.lock |
| "Std-only by C5" | Cargo.toml:16 | REAL (manifest level) | all five prod deps + both dev deps are path deps; zero registry edges |
| "the desktop composition root ADOPTS it [lumen-cli] as a library dependency rather than re-implementing a second engine host" | Cargo.toml:34-40 | REAL | shell.rs/dispatch.rs route through lumen_cli::host::CliHost (shell.rs:51, 139-141); no second host exists in this crate |
| "media-clock, TEST-ONLY" | Cargo.toml:49-54 | REAL | only cfg(test) usage (frame_tap.rs:136+; tests/common) |
| boundaries.toml internal_deps incl. renderer-wgpu | boundaries.toml:540 | MISCLASSIFIED | production-usage scan: zero non-test uses (F-001) |

## 7. Dead / duplicated / unreachable code
- renderer-wgpu production edge: dead in the shipped binary (F-001).

## 8. Dependencies & flags
- No features, no optional deps, no registry deps. renderer-wgpu itself is std-only (its own Cargo.toml: wgpu import deferred, software compositor linked) — so F-001's cost is link-weight + record drift, NOT a GPU graph in the binary.

## 9. Open questions for the next agent
- boundaries.toml labels the section `[crates.desktop]` although the crate lives in apps/ — cosmetic, but check whether the xtask boundary checker keys on the prefix (a apps.* section might be silently unvalidated).
- Who consumes renderer-wgpu's `uploads_format` parity check (wire.rs:999)? If the wire table and renderer table drift, only that test notices — fine — but then the production wire table has no runtime coupling to the renderer at all, which is the point; the prod edge is even less justified.
