# AUDIT: crates/platform-windows/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W08-A07 |
| Date | 2026-11-23 |
| Lines / functions | 43 / 7 config keys (name, version, edition, description, publish, lints, dependencies, dev-dependencies) |
| File verdict | REAL manifest — claims mostly verified; one UNVERIFIABLE compile-check claim; crate is an orphan |
| Reachable from shipped entry point? | NO — `platform-windows` has zero workspace dependents (rg over all Cargo.tomls) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | rustup target list --installed; rustc parse-checks x4 (0 syntax errors); rg dep-usage sweep; `cargo test -p platform-windows --test seam_behaviour` (exit 0, 16 passed, ran for file 4) |

## 1. One-paragraph verdict
The manifest is honest where it can be checked: std-only as claimed (two path
deps, zero external crates, no windows-s), both dependencies actually used
(audio_output at 5 sites, media_types in wasapi.rs), testkit is dev-only and
drives the 16 passing host tests. The two problems: the crate is the third
orphan platform adapter (nothing depends on it — the release lane never
builds it), and the "compile-verified via cargo check --target
x86_64-pc-windows-msvc" claim cannot be reproduced in this sandbox (no
Windows target installed); my standalone rustc parse-checks of all four FFI
files show zero syntax errors, which corroborates the claim at the parse
level only — typechecking remains UNVERIFIED.

## 2. What actually works (verified)
— Dependency claims: `[dependencies] audio-output, media-types` + `[dev-
dependencies] testkit` — usage verified by rg (audio.rs:16, events.rs:23,
wasapi.rs:47-48, lib.rs:10); no unused dependency.
— "NO external dependencies" (line 14): TRUE — no crates.io entries in the
manifest.
— FFI file sanity: rustc --edition 2024 standalone parse of com.rs,
dialogs.rs, dxgi.rs, wasapi.rs -> 0 syntax-class errors each (resolution
errors only, expected standalone).
— Host tests: 16/16 platform-windows seam_behaviour pass (run under file 4).

## 3. Findings
F-001 [S2]
Where: crates/platform-windows/Cargo.toml:26-38
Code:
```
[package]
name = "platform-windows"
version = "0.1.0"
```
What is wrong: orphan crate — no Cargo.toml in the tree lists platform-
windows as a dependency (rg sweep: only its own manifest and a comment in
platform-linux's). The "L5 registry row" (line 2) describes a layer that no
shipped binary contains. Fifth file of this assignment in the same dead-crate
family (platform-linux, platform-macos x2, platform-windows x2).
Confidence: HIGH — Verification: ran (rg).
F-002 [S4 — claim]
Where: crates/platform-windows/Cargo.toml:20-24
Code:
```
# Sandbox honesty: this Linux sandbox has no Windows SDK, so the
# FFI surface is compile-verified via `cargo check --target
# x86_64-pc-windows-msvc` (typechecked declarations, no linking, no
# runtime); the host-runnable tests cover the typed seams, capability
# negotiation and fallbacks. Real-device behavior is `[VERIFY]`.
```
What is wrong: UNVERIFIABLE here — `rustup target list --installed` shows
only wasm32-unknown-unknown and x86_64-unknown-linux-gnu, so the claimed
cross-check cannot be re-run; no CI lane for windows targets is visible from
this sandbox. Parse-level corroboration done (0 syntax errors); typecheck
UNVERIFIED (reason: no windows rust-std component; cargo-target-budget).
Confidence: HIGH that it is unverifiable; the disclosure of no-runtime is
honest. Verification: ran (rustup, rustc parse checks).
F-003 [S4]
Where: crates/platform-windows/Cargo.toml:4-12
Code:
```
# Registry: doc 01 §2.7 `platform-windows` (layer L5, deps:
# media-types, codec-api, audio-output, renderer-wgpu,
# device-discovery). This card's v0 surface uses the audio-output
```
What is wrong: the registry row claims five dependencies; the manifest
declares two. The comment discloses the gap (codec-api = issue #41,
renderer-wgpu Lead-gated, device-discovery = issue #42), but as shipped,
doc 01's L5 registry overstates the wiring — anyone reading the registry
believes hw-decode and renderer edges exist. Unverifiable whether the issues
are still open. Confidence: MEDIUM — Verification: read (manifest vs
comment); issue-tracker state UNVERIFIED.

## 4. Per-function table (config keys — file < 100 LOC)
| Key | Lines | Verdict | Notes |
|---|---|---|---|
| name/version/edition | 27-29 | REAL | edition 2024, consistent with siblings |
| description | 30 | REAL | accurate scope summary (WASAPI/DXGI/dialogs, typed fallback) |
| publish | 31 | REAL | false — workspace-internal |
| lints | 33-34 | REAL | workspace inheritance (the documented pattern) |
| dependencies | 36-38 | REAL | both used; std-only claim holds |
| dev-dependencies | 40-43 | REAL | testkit dev-only per contract; used by 16 passing tests |

## 5. Test honesty
No tests in this file. The dev-deps it declares back the crate's test suite
(executed under file 4: 16/16 pass).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only by C5: NO external dependencies, NO windows-s crate" | lines 14-15 | REAL | manifest + rg (0 external deps; FFI hand-declared in src/windows/) |
| "compile-verified via cargo check --target x86_64-pc-windows-msvc" | lines 20-23 | UNVERIFIED | no windows target installed; parse-checks only |
| "the FFI surface … hand-declared `extern "system"` in src/windows/" | lines 15-18 | REAL | 86 extern "system" declarations across 4 files (rg count) |
| Registry deps: media-types, codec-api, audio-output, renderer-wgpu, device-discovery | lines 4-5 | FALSE-as-shipped (2 of 5 exist) | manifest lists 2; gap disclosed in comment |

## 7. Dead / duplicated / unreachable code
The crate the manifest describes is unreachable from shipped entry points
(F-001). The manifest itself is internally consistent (no declared-but-unused
deps, no phantom features, no config keys read by nothing).

## 8. Dependencies & flags
No features, no flags, no targets table, no build.rs. Path-only deps.
Publish=false. This is the minimal honest manifest shape.

## 9. Open questions for the next agent
- Same as files 1-5: which card owns consuming the platform crates? If the
  answer is "none yet", the three orphans should be escalated together as one
  architecture finding, not four per-file S2s.
- doc 01 §2.7's registry rows for the three platform crates claim dependency
  edges that do not exist — the registry-vs-manifest drift is disclosed in
  comments but the registry doc itself may need a doc-audit pass (doc 01 is
  not in my assignment).
