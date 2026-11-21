# AUDIT: crates/platform-macos/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A09 |
| Date | 2026-11-21 |
| Lines / functions | 46 / 7 manifest keys (package, lints, 2 deps, 1 dev-dep) + 3 header claim blocks |
| File verdict | REAL (manifest's dep table is accurate and boundaries-mirrored) — **crate verdict: UNREACHABLE** (orphan: zero dependents in the whole workspace) |
| Reachable from shipped entry point? | NO / UNREACHABLE. Proof: `rg -n 'platform-macos' --glob 'Cargo.toml' .` matches ONLY `crates/platform-macos/Cargo.toml` itself; `rg -ln 'platform_macos' crates apps tools tests --glob '*.rs'` matches only `crates/platform-macos/tests/seam_behaviour.rs`. Neither `lumen` (apps/cli), `lumen-tauri`/`lumen-desktop`, `lumen-web`, nor `control-server` depends on it. It is compiled only as a bare workspace member by `cargo build --workspace` (baseline #1). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 0 |
| Commands run | `rg -n 'platform-macos' --glob 'Cargo.toml'` (zero consumers) · `rg -ln 'platform_macos' crates apps tools tests` (own tests only) · `sed -n '472,488p' boundaries.toml` ([crates.platform-macos] matches manifest) · `ls crates/platform-macos/src/macos` + `rg -n 'extern "C"'` (FFI surface real: coreaudio.rs:175,215; metal.rs:60; objc.rs:76; dialogs.rs:55) · `wc -l` (2,892 LOC src) · `ls crates/device-discovery/src` + its manifest head (issue #42 landed, no edge) · `rg -n 'extern "C"' crates/codecs/src/video/h264/hw/videotoolbox/ffi.rs` (parallel macOS FFI: 185,234,253) · `rg -n 'wgpu' crates/renderer-wgpu/Cargo.toml` (wgpu NOT imported — gpu.rs forward-claim still pending) · `rg -n 'testkit' crates/platform-macos/tests` |

## 1. One-paragraph verdict
The manifest is internally honest — two internal deps, zero externals, exactly mirrored by `boundaries.toml:482-487` — and the crate behind it contains a real, non-trivial hand-rolled macOS FFI surface (2,892 LOC: CoreAudio property listeners, Metal/CAMetalLayer glue, objc_msgSend panels). But the crate is an orphan: nothing in the repository depends on it, so no shipped entry point can reach a single line of it; the header's forward-looking edge story is doubly stale (issue #41's hw-decode and issue #42's device-discovery cards both landed without the promised platform-macos edges — VideoToolbox FFI was hand-rolled a second time inside `codecs`); and the "compile-verified via cargo check --target aarch64/x86_64-apple-darwin" assurance has no committed evidence anywhere. A macOS adapter that no app embeds is a library, not a platform layer of the product.

## 2. What actually works (verified)
- Dep table ↔ boundaries exact match: `boundaries.toml:482-487` `[crates.platform-macos] layer = 5, internal_deps = ["audio-output", "media-types"], dev_deps = ["testkit"], external_allowlist = []` — identical to manifest lines 39-46. Verified by sed.
- Std-only claim holds: no external deps; the FFI is hand-declared — `unsafe extern "C"` blocks exist at coreaudio.rs:175, 215, metal.rs:60, objc.rs:76, dialogs.rs:55 (rg verified).
- dev-dep `testkit` genuinely used: `crates/platform-macos/tests/seam_behaviour.rs:27` `use testkit::assert_err;`.
- The crate compiles in the workspace release build (baseline #1, exit 0 — includes all `crates/*` members).
- gpu.rs's forward claim is still accurate (not stale): renderer-wgpu does NOT import wgpu today (`rg -n 'wgpu' crates/renderer-wgpu/Cargo.toml` shows only the pending-admission comment; no `^wgpu` package in Cargo.lock).

## 3. Findings

### F-001 [S3]
Where: crates/platform-macos/Cargo.toml:1-12 (whole-manifest claim block)
Code:
```
# crates/platform-macos — the macOS platform adapter (card 7.A3,
# issue #40; doc 01 §2.7 L5 registry row).
#
# Registry: doc 01 §2.7 `platform-macos` (layer L5, deps: media-types,
# codec-api, audio-output, renderer-wgpu, device-discovery).
```
What is wrong: The crate is dead weight in the shipped product: no Cargo.toml in the repository declares a dependency on `platform-macos` (rg over every manifest: only its own), and no Rust file outside the crate references `platform_macos` (rg: only its own tests). Neither shipped entry point (`lumen`, `lumen-tauri`) nor any engine crate reaches it. The manifest presents the crate as "the macOS platform adapter" — a layer of the product — without disclosing that nothing wires it.
Why it matters: 2,892 LOC of `unsafe extern "C"` FFI (CoreAudio, Metal, Objective-C runtime) that no binary exercises is pure liability: it compiles in every `--workspace` lane (slower builds, larger audit surface, unsafe code that never runs) while contributing zero product behavior. An auditor or the owner tracing "macOS support" lands here and believes the product has an adapter; the shipped `lumen` binary contains none of it.
What correct looks like: Either an app/host declares the dependency (and the reachability chain is documented in this manifest), or the manifest header states "not yet consumed by any host; compiled as a library only".
Confidence: HIGH — Verification: read (two exhaustive rg sweeps) + baseline #1.

### F-002 [S3]
Where: crates/platform-macos/Cargo.toml:8-11
Code:
```
# (DeviceDesc/DeviceId — the seam the L4 trait layer minted, ADR-005)
# + media-types (AudioFormat); codec-api (hw decode) is issue #41,
# renderer-wgpu (wgpu import) is Lead-gated by its admission record,
# device-discovery is issue #42 — the manifest is an allowlist, those
# edges join when their cards land. Edges point downward: L5 → L4
```
What is wrong: The "edges join when their cards land" story is stale for two of the three deferred edges. Issue #41 (hw decode) landed 2026-11-10 as the VideoToolbox/VA-API lanes inside `crates/codecs/src/video/h264/hw/` — with its OWN second hand-rolled macOS FFI (`hw/videotoolbox/ffi.rs:185,234,253` unsafe extern "C" blocks) — and no codec-api edge ever joined platform-macos. Issue #42 (device-discovery) landed as `crates/device-discovery` (card 7.D1, its header says OS integrations live in L5 platform-* with events flowing the other direction) — again no edge to platform-macos. The promised joins never happened and never will in this shape.
Why it matters: The manifest's architecture narrative (hw decode and discovery route through the platform adapter) is contradicted by the tree: macOS FFI now exists in two parallel places (this crate and codecs' videotoolbox lane), doubling the unsafe surface an auditor must check and guaranteeing drift between two hand-rolled bindings of the same OS.
What correct looks like: Header updated to the real state: hw decode lives in codecs (own FFI, ADR-001 lane), device-discovery consumes platform events rather than being consumed; this crate currently serves only its own tests.
Confidence: HIGH — Verification: read (rg of both landed cards' locations + ffi.rs extern blocks + device-discovery manifest).

### F-003 [S3]
Where: crates/platform-macos/Cargo.toml:22-27
Code:
```
# Sandbox honesty: this Linux sandbox has no macOS SDK, so the FFI
# surface is compile-verified via `cargo check --target
# aarch64-apple-darwin` and `x86_64-apple-darwin` (typechecked
# declarations, no linking, no runtime); the host-runnable tests
# cover the typed seams, capability negotiation and fallbacks.
# Real-device behavior is `[VERIFY]`.
```
What is wrong: "compile-verified" is an executed-evidence claim with no committed evidence: the wave-0 baseline (audit/runs/baseline/INDEX.md, 26 runs) contains no `cargo check --target *-apple-darwin` run, and no log of such a run exists anywhere in the repo or audit tree. I cannot re-run it (hard rule 8). The claim may be true, but as evidence it is worth nothing — exactly the "evidence strings that prove nothing" pattern.
Why it matters: This manifest's only assurance that 2,892 LOC of unsafe FFI even typechecks on macOS is an unverifiable sentence. The h264 hw matrix row repeats the same claim ("compile-verified via cargo check --target aarch64-apple-darwin") — the project's macOS story rests on an uncommitted run.
What correct looks like: Commit the cross-check log (command + exit code + date) next to the claim, or mark it `[UNVERIFIED]`.
Confidence: HIGH — Verification: read + exhaustive absence check (INDEX table; `rg --files audit | rg '\.log'` → only my own log; ls audit/runs/baseline/).

## 4. Per-function table (manifest keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header: registry row claim (5 deps) | 4-12 | drift | S3 | 3 of 5 registry deps never joined; 2 landed elsewhere (F-002) |
| header: std-only/hand-FFI claim | 14-20 | REAL | — | verified: no externals, extern "C" blocks present |
| header: cross-target compile-check claim | 22-27 | REAL-UNPROVEN | S3 | no committed evidence (F-003) |
| `[package]` block | 29-34 | REAL | — | description accurate about the crate's contents |
| `[lints] workspace = true` | 36-37 | REAL | — | inherits workspace policy |
| dep `audio-output` | 40 | REAL | — | L4 seam (DeviceDesc/DeviceId), boundaries-mirrored |
| dep `media-types` | 41 | REAL | — | L0 vocabulary, boundaries-mirrored |
| dev-dep `testkit` | 43-46 | REAL | — | used (seam_behaviour.rs:27) |

## 5. Test honesty
Tests live in `crates/platform-macos/tests/seam_behaviour.rs` (436 lines). Read of the first three tests: `unavailable_audio_devices_returns_the_typed_reason` (line 35) asserts `enumerate`/`subscribe`/`poll` all fail with the SAME typed reason — real assertions on the fallback path, host-runnable on Linux. Mutation reasoning: if the typed-Unavailable fallback returned Ok or a different error, the `assert_err`/`assert_eq` pairs fail. What the suite CANNOT cover (by the manifest's own admission): any macOS FFI behavior — real-device behavior is `[VERIFY]`. Note the tests only prove the crate's refusal paths; the 2,892 LOC of actual FFI have zero runtime coverage anywhere.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the macOS platform adapter (card 7.A3, issue #40)" | manifest line 1 | SHALLOW | real code, zero consumers — an adapter adapted to nothing (F-001) |
| registry deps "codec-api … device-discovery" join "when their cards land" | manifest lines 8-11 | FALSE (stale) | both cards landed elsewhere; edges never joined (F-002) |
| "Std-only … NO objc2/core-audio crates" | manifest lines 14-16 | REAL | no external deps; hand extern "C" verified |
| "compile-verified via cargo check --target aarch64-apple-darwin and x86_64-apple-darwin" | manifest lines 22-25 | REAL-UNPROVEN | no committed run log exists (F-003) |
| "host-runnable tests cover the typed seams, capability negotiation and fallbacks" | manifest lines 25-26 | REAL | seam_behaviour.rs asserts typed refusals (read) |

## 7. Dead / duplicated / unreachable code
The entire crate is UNREACHABLE from shipped entry points (proof in header row). Duplication: the VideoToolbox/CoreMedia/CoreVideo FFI in `crates/codecs/src/video/h264/hw/videotoolbox/ffi.rs` parallels this crate's macOS-FFI-by-hand approach — two independent unsafe bindings to the same OS, one dead (this crate), one live but compile-verified only. Platform siblings platform-linux and platform-windows are equally orphaned (rg: no dependents) — pattern finding for whoever audits those manifests.

## 8. Dependencies & flags
Two internal deps (audio-output L4, media-types L0), both boundaries-mirrored; one dev-dep (testkit). No features, no externals, no build.rs. `[lints] workspace = true`. Nothing to flag in the dep table itself — the problem is who does NOT depend on this crate.

## 9. Open questions for the next agent
1. Who was supposed to consume platform-macos? If the answer is "the tauri shell" (apps/desktop-tauri), note that `apps/desktop-tauri/Cargo.toml:53` depends only on `desktop`, which depends on lumen-cli — the chain never touches platform-macos. The wiring card exists nowhere.
2. codecs' videotoolbox auditor: confirm the hw lane's FFI does not duplicate the objc.rs/msgSend machinery here (drift between the two bindings).
3. Is the same orphan pattern true for platform-windows (its only mention is inside platform-linux's manifest — a comment, not a dep edge)?
