# AUDIT: crates/platform-linux/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W08-A05 |
| Date | 2026-11-23 |
| Lines / functions | 46 / manifest: 1 package, 2 deps, 1 dev-dep, 0 features |
| File verdict | REAL (every manifest claim cross-checked; the crate's integration tests actually run on this host per baseline) |
| Reachable from shipped entry point? | NO — zero workspace crates depend on platform-linux (rg over all Cargo.tomls: no matches outside the crate itself). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg media_types usage (exit 0 — used); rg dlopen/unsafe in pulse.rs (exit 0); rg platform-linux dependents (exit 1 = none); rg features section (exit 1 = none); baseline cite cargo-test-workspace.txt:3943-4002 |

## 1. One-paragraph verdict
The cleanest manifest of the three platform crates: no external dependencies (path deps only), no [features] section, dlopen-based PulseAudio (no link-time C library), dev-only testkit, and — uniquely among the platform set — integration tests that genuinely execute against this Linux host (13/13: real sysfs DRM walk, real pulse dlopen probe, zenity actually run, absent-server typed errors; baseline cargo-test-workspace.txt:3958-3973). Every header claim I could check verified. The two defects: the crate is an island nothing depends on, and the header's registry dep list (5 edges) describes a future graph, not this manifest's (2 edges) — self-explained, but a reader of doc 01 §2.7 must cross-check two places to know which is real.

## 2. What actually works (verified)
- "Runtime-tested on Linux: capability negotiation, fallback chains, event-queue plumbing, dlopen probe + absent-server typed errors" (header lines 25-27) — REAL: baseline runs `pulse_probe_reports_this_hosts_truth`, `pulse_subscribe_without_a_server_is_the_typed_error`, `drm_scanner_reads_the_real_host_sysfs_honestly`, `xdg_dialogs_run_the_zenity_fallback`, `discover_assembles_this_hosts_report` (cargo-test-workspace.txt:3959-3971, 13 passed).
- "NO external dependencies" (line 14) — REAL: `[dependencies]` contains only path deps `audio-output`, `media-types` (lines 39-41).
- "hand-rolled extern "C" + runtime dlopen (no libpulse link-time dependency)" (lines 14-16) — REAL: pulse.rs declares the dlopen probe (`PulseAudioDevices::probe`, pulse.rs:8) and no libpulse/libpulse-sys appears in the manifest.
- Both declared deps are used: `media_types::frame::{AudioFormat, AudioLayout, SampleFormat}` imported at pulse.rs:42; audio-output vocabulary used throughout src/.
- testkit is dev-only (lines 43-46) — matches MP §3.7 claim; `[lints] workspace = true` (36-37) inherits the deny lists (workspace Cargo.toml:53-63).

## 3. Findings
F-001 [S3]
Where: manifest as shipped; evidence: rg over all Cargo.tomls
Code:
```
[dependencies]
audio-output = { path = "../audio-output" }
media-types = { path = "../media-types" }
```
What is wrong: no crate in the workspace lists platform-linux as a dependency. The adapter with the best runtime story of the three (real host-tested integrations) is exactly as unreachable as its Windows/macOS siblings: `cargo build --release -p lumen-cli -p desktop` never compiles it, so no shipped binary contains the PulseAudio probe, DRM scanner, or xdg dialog chain.
Why it matters: the only platform crate whose integrations were proven on real hardware is dead weight in the repo until something depends on it.
Confidence: HIGH — Verification: ran rg (exit 1, no dependents).
F-002 [S4]
Where: header lines 4-12 vs [dependencies] lines 39-41
Code:
```
# Registry: doc 01 §2.7 `platform-linux` (layer L5, deps per the
# platform-windows template: media-types, codec-api, audio-output,
# renderer-wgpu, device-discovery). This card's v0 surface uses the
# audio-output vocabulary (DeviceDesc/DeviceId — the seam the L4 trait
# layer minted) + media-types (AudioFormat); codec-api (hw decode) is
```
What is wrong: the registry row promises five dependency edges; the manifest carries two. The gap is explained in the same comment ("the manifest is an allowlist, those edges join when their cards land"), so it is honest — but the doc 01 §2.7 registry and this manifest now disagree until issues #41/#42 and the renderer-wgpu admission land, and nothing in CI checks the registry graph against the actual manifests (the xtask feature-audit would be the natural home).
Confidence: HIGH — Verification: read + rg deps.

## 4. Per-function table
Not applicable (manifest, 46 lines). Config-key inventory instead:
| Key | Verdict | Notes |
|---|---|---|
| [package] name/version/edition/publish | REAL | edition 2024, publish=false — consistent with siblings |
| description | REAL | accurately names PulseAudio dlopen + DRM + xdg fallback chain |
| [lints] workspace = true | REAL | inherits unsafe_code=deny etc.; FFI modules opt out via module-level allow (verified in pulse.rs:31 SAFETY-SCOPED header) |
| [dependencies] audio-output, media-types | REAL | both used (pulse.rs:42; DeviceDesc/DeviceId throughout) |
| [dev-dependencies] testkit | REAL | dev-only, used by tests/seam_behaviour.rs + linux_integrations.rs |
| [features] | ABSENT | no cargo feature gates exist in any of the three platform crates — all platform gating is `cfg(target_os=...)` in source (verified: rg features → exit 1 in all three manifests) |

## 5. Test honesty
Tests belong to the crate, not this manifest; for the record the baseline shows 13 integration + 15 seam tests passing (cargo-test-workspace.txt:3958-3988), including host-truth tests that would FAIL if the crate lied about the environment (pulse probe on a host without pulse, zenity actually executed). These are the strongest platform tests in the repo.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Std-only by C5: NO external dependencies" | header:14 | REAL | deps = 2 path crates only |
| "no libpulse link-time dependency" | header:15-16 | REAL | dlopen probe in pulse.rs; nothing links libpulse |
| "Dialogs probe the D-Bus session bus (std UnixStream) and fall back to zenity/kdialog" | header:18-19 | REAL | baseline runs xdg_portal_probe_reports_bus_truth + zenity/kdialog tests (3966-3968) |
| "Runtime-tested on Linux: ... dlopen probe + absent-server typed errors" | header:25-27 | REAL | baseline 3964-3965 |
| registry "deps: media-types, codec-api, audio-output, renderer-wgpu, device-discovery" | header:4-5 | PARTLY FALSE (aspirational) | manifest has 2 of 5; explained in-header (F-002) |

## 7. Dead / duplicated / unreachable code
Manifest mirrors the platform-windows/platform-macos manifests in shape (same dep allowlist, same testkit dev-dep) — same triplication family. Crate unreachable from shipped lane (F-001).

## 8. Dependencies & flags
No [features]; no target-specific dependency tables (no [target.'cfg(...)'.dependencies] — the dlopen approach makes even the Linux C ABI load-time-optional, which is why none is needed). Workspace lints inherited.

## 9. Open questions for the next agent
- Who is supposed to consume platform-linux (issue #42 device-discovery? the desktop app?) — if that card is cancelled, three crates' worth of FFI work is stranded; if it lands, the registry/manifest mismatch (F-002) resolves itself.
