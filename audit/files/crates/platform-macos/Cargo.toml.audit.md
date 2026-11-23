# AUDIT: crates/platform-macos/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W08-A05 |
| Date | 2026-11-23 |
| Lines / functions | 46 / manifest: 1 package, 2 deps, 1 dev-dep, 0 features |
| File verdict | REAL (manifest claims verified against source and baseline; cross-target compile claim UNVERIFIABLE here) |
| Reachable from shipped entry point? | NO — zero workspace crates depend on platform-macos (rg over all Cargo.tomls: no matches outside the crate). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg media_types usage (exit 0); rg #[link] attributes in src/macos (exit 0, 5 frameworks/dylibs); rg dependents (exit 1 = none); rg features (exit 1 = none); diff vs platform-linux manifest (66 lines, header-only); baseline cite cargo-test-workspace.txt:4004-4042 |

## 1. One-paragraph verdict
Structurally identical to the platform-linux manifest (same 2 path deps, same dev-only testkit, same [lints] inheritance, 66-line diff that is header commentary only). Its claims hold: no objc2/core-audio crates (deps are path-only), the OS C-ABIs are hand-declared in src/macos/ (verified: objc.rs, dialogs.rs, metal.rs, coreaudio.rs), media-types is genuinely used (coreaudio.rs:60), and the host-runnable fallback/seam tests pass (16/16 baseline). The honest difference from its Linux sibling: this crate carries five link-time framework dependencies (`libobjc`, `System`, `Metal`, `CoreAudio`, `CoreFoundation` via #[link] attributes) where platform-linux brags zero link-time C deps — inherent to Apple platforms and always present, but a reader comparing the two headers should know "Std-only / NO external dependencies" means cargo deps only. The two real defects: the crate is an island (nothing depends on it), and the darwin compile-verification claim cannot be re-executed in this sandbox.

## 2. What actually works (verified)
- "NO objc2/core-audio crates" (line 14) — REAL: `[dependencies]` = audio-output + media-types only (lines 39-41).
- "the OS C-ABIs are hand-declared extern "C" in src/macos/" (lines 15-18) — REAL: #[link] declarations at objc.rs:75 (libobjc), dialogs.rs:54 (System), metal.rs:59 (Metal), coreaudio.rs:174/214 (CoreAudio, CoreFoundation).
- Both deps used: `media_types::frame::{AudioFormat, AudioLayout, SampleFormat}` at coreaudio.rs:60; audio-output vocabulary throughout.
- "the host-runnable tests cover the typed seams, capability negotiation and fallbacks" (lines 25-26) — REAL: baseline 16/16 (cargo-test-workspace.txt:4016-4034).
- No [features] section (verified rg) — no cargo feature gates; all gating is cfg(target_os="macos") in source.

## 3. Findings
F-001 [S3]
Where: manifest as shipped; evidence: rg over all Cargo.tomls (exit 1, no dependents)
Code:
```
[dependencies]
audio-output = { path = "../audio-output" }
media-types = { path = "../media-types" }
```
What is wrong: nothing in the workspace depends on platform-macos; the third island crate. All 344 lines of objc.rs, the CoreAudio HAL client, the Metal glue and the AppKit panels compile into nothing any shipped binary contains. Same finding as my dialogs.rs/objc.rs/lib.rs/audio.rs/events.rs audits and both sibling manifests — the entire platform-* family is unwired.
Confidence: HIGH — Verification: ran rg.
F-002 [S4]
Where: header lines 4-12 vs [dependencies] lines 39-41
Code:
```
# Registry: doc 01 §2.7 `platform-macos` (layer L5, deps: media-types,
# codec-api, audio-output, renderer-wgpu, device-discovery). This
```
What is wrong: registry promises five dependency edges, manifest ships two; explained in-header (allowlist until issues #41/#42 + renderer-wgpu admission), but the registry-vs-manifest drift is not CI-checked (xtask feature-audit is the natural gate and is currently failing on other grounds per baseline).
Confidence: HIGH — Verification: read.
F-003 [S4]
Where: header lines 22-27
Code:
```
# Sandbox honesty: this Linux sandbox has no macOS SDK, so the FFI
# surface is compile-verified via `cargo check --target
# aarch64-apple-darwin` and `x86_64-apple-darwin` (typechecked
```
What is wrong: not re-runnable in this sandbox (no rustup, no darwin targets installed) — UNVERIFIED; recorded at worklog.md:556. Typecheck-only: no linking against libobjc/CoreAudio/Metal occurred even when it ran, so the #[link] name/kind pairs are untested.
Confidence: MEDIUM — Verification: UNVERIFIED (toolchain absent).

## 4. Per-function table
Not applicable (manifest, 46 lines). Config-key inventory:
| Key | Verdict | Notes |
|---|---|---|
| [package] name/version/edition/publish | REAL | matches siblings |
| description | REAL | names CoreAudio/Metal/NSOpenPanel exactly as src/macos/ delivers |
| [lints] workspace = true | REAL | unsafe_code=deny inherited; the four FFI modules opt out via documented module-level #[allow(unsafe_code)] (objc.rs:48 etc.) |
| [dependencies] audio-output, media-types | REAL | both used |
| [dev-dependencies] testkit | REAL | dev-only, used by tests/seam_behaviour.rs |
| [features] | ABSENT | no cargo features — hostile-focus answer: platform feature gating is exclusively cfg(target_os) in source; nothing can be feature-flagged off/on |

## 5. Test honesty
Manifest carries no tests; crate tests (16/16 baseline) exercise only the non-macOS fallback arms — the macOS FFI surface this manifest exists to build has zero execution evidence anywhere in the repo (compile-typecheck claim aside). Consistent with the header's own `[VERIFY]` honesty label.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NO external dependencies, NO objc2/core-audio crates" | header:14 | REAL | deps = 2 path crates |
| "compile-verified via cargo check --target aarch64-apple-darwin and x86_64-apple-darwin" | header:22-24 | UNVERIFIED | worklog.md:556 records it; no darwin toolchain here |
| "host-runnable tests cover the typed seams, capability negotiation and fallbacks" | header:25-26 | REAL | baseline 16/16 |
| registry "deps: media-types, codec-api, audio-output, renderer-wgpu, device-discovery" | header:4-5 | PARTLY FALSE (aspirational) | manifest has 2 of 5 (F-002) |
| "Real-device behavior is [VERIFY]" | header:27 | REAL (honest label) | no macOS runtime evidence exists |

## 7. Dead / duplicated / unreachable code
Manifest is a near-copy of platform-linux's (66-line diff, header text only) — part of the declared triplication. Crate unreachable from shipped lane (F-001).

## 8. Dependencies & flags
No [features], no target-specific dep tables. Link-time surface (five #[link] attrs) lives in source, not the manifest — by design (hand-rolled FFI). Note the asymmetry vs platform-linux: Linux dlopens its C library (zero link-time deps); macOS links frameworks at build time — unavoidable on Apple, but the two headers' "Std-only" phrasing reads identical while meaning different link postures.

## 9. Open questions for the next agent
- If a darwin CI lane ever exists, a `cargo check --target aarch64-apple-darwin` step would convert F-003 from UNVERIFIED to verified and test the five #[link] declarations.
- The unsafe-fn fix for objc.rs::retain (my objc.rs F-001) should land before anything depends on this crate.
