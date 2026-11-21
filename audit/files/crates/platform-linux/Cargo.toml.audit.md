# AUDIT: crates/platform-linux/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A02 |
| Date | 2026-11-21 |
| Lines / functions | 47 / 7 config keys ([package]×5, [lints], [dependencies]×2, [dev-dependencies]×1) |
| File verdict | UNREACHABLE (manifest internally consistent; crate has zero dependents and its central design — hand-rolled libpulse FFI — sits outside every recorded decision) |
| Reachable from shipped entry point? | NO — no Cargo.toml declares a dependency on `platform-linux` (proof below) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | Grep `^\s*platform-linux\s*=` over `**/Cargo.toml` → 0 matches (no dependents); Grep boundaries.toml `[crates.platform-linux]` → internal_deps ["audio-output","media-types"], dev_deps ["testkit"] (:472-477) = exact manifest match; Grep `allow\(unsafe_code\)` in src → exactly 3 sites (pulse.rs:34, pulse_ffi.rs:26, dl.rs:24) + Grep `#\[allow\(` → 4th escape `#[allow(clippy::struct_field_names)]` (pulse_ffi.rs:199); Grep docs for `dlopen\|libpulse\|PulseAudio` → ADR-005:149 ("weaker fit, daemon dependency"), platform-synthesis.md:53-54,152, doc 13:705 (card 7.A2); `sed -n '12,27p'` manifest (quote verify); baseline #1/#3/#5/#11 |

## 1. One-paragraph verdict
The manifest's declared edges are exact (audio-output + media-types, matching boundaries.toml:472-477 both directions), `[lints] workspace = true` binds, and the unsafe-exception claim is verifiable — exactly three module-level `#![allow(unsafe_code)]` sites, all in FFI-bearing modules, plus one style escape. The problem is governance: the crate's centerpiece, a hand-rolled `extern "C"` + runtime-dlopen PulseAudio client, matches none of the recorded decisions — card 7.A2 (doc 13:705) prescribes "`[VERIFY]` crates … same audit bar", ADR-005 (the ADR the manifest cites for "the audio seam") offers cpal or metadata-generated ALSA/CoreAudio/WASAPI bindings and its own survey ranks the libpulse path as "weaker fit, daemon dependency" (ADR-005:149) — and because hand-rolled FFI is not a crate, it is invisible to the entire dependency-admission regime (deny.toml bans, allowlist rows, records, F1–F7 scans all key on crates). On top of that the crate is an orphan: nothing depends on platform-linux, so the Linux platform adapter ships in no binary.

## 2. What actually works (verified)
— Manifest ↔ boundaries.toml exact two-way match: `[crates.platform-linux] internal_deps = ["audio-output", "media-types"]`, `dev_deps = ["testkit"]` (boundaries.toml:472-477) equals the manifest's `[dependencies]`/`[dev-dependencies]` (manifest:39-46). Both are a strict subset of doc 01 §2.7's target row (5 deps), and the manifest honestly marks the missing three as future cards (lines 8-11).
— Unsafe-exception claim verified: exactly 3 `#![allow(unsafe_code)]` sites (src/linux/pulse.rs:34, pulse_ffi.rs:26, dl.rs:24) — every one in an FFI-bearing module (pulse.rs holds `unsafe extern "C" fn` callbacks at :174/:189/:231, pulse_ffi.rs the extern declarations, dl.rs the dlopen shim); since `unsafe_code = "deny"` binds and baseline #5 clippy is CLEAN, no unsafe exists outside these modules.
— Crate compiles and its tests run in the workspace suite (baseline #1 build CLEAN, #3 1681 passed/1 unrelated fail).

## 3. Findings
### F-001 [S3]
Where: crates/platform-linux/Cargo.toml:14-19,21-25
Code:
```
# Std-only by C5: NO external dependencies. The PulseAudio client is
# hand-rolled `extern "C"` + runtime dlopen (no libpulse link-time
# dependency — the typed Unavailable path is the honest fallback when
# the library/server is absent). DRM adapter enumeration is pure std
# sysfs walking. Dialogs probe the D-Bus session bus (std UnixStream)
# and fall back to zenity/kdialog commands.
```
What is wrong: The substrate decision is unrecorded and contradicts the plan of record. Card 7.A2 (docs/13-agent-wave-program.md:705) prescribes "PipeWire/PulseAudio (`[VERIFY]` crates) device enum + notifications … same audit bar" — i.e. a verified crate candidate (the inventory lists pulseaudio 0.3.2 pure-Rust and libpulse-binding 2.30.1, platform-synthesis.md:53-54). ADR-005 — which the manifest cites as the module's ADR reference — decides between cpal as substrate (decision 3, explicitly "no jack/pulse/pipewire features") and metadata-generated direct APIs (decision 4), and its alternatives survey dismisses the libpulse path as "weaker fit, daemon dependency" (ADR-005:147-153). The implementation instead hand-rolls the entire libpulse C surface — a path no doc decides, and one the crate-keyed audit machinery cannot even see: there is no dependency record, no allowlist row, nothing for deny.toml to ban, no F1–F7 scan (all of those gate crates, and this crate declares zero).
Why it matters: The repo's central governance claim — every C surface passes the dependency-admission bar (doc 11 §1.1, deny.toml:32-37) — has a bypass-by-construction: write the FFI by hand and no gate fires. This crate is the precedent; the same pattern in a decoder would be exactly the "dlopen path into C code" deny.toml:176-178 says is forbidden for media engines. The unsafe is disclosed and module-scoped, which keeps this at S3 — but the "same audit bar" the card promised was never met for the substrate choice.
What correct looks like: An ADR-005 amendment (or a new ADR) recording the hand-rolled-dlopen decision, its alternatives (pulseaudio 0.3.2 pure-Rust protocol crate), the libpulse API surface adopted, its stability/version posture, and the safety-review result — plus an explicit statement in doc 11 that hand-rolled extern "C" surfaces are audited at the module level (the mem-audit precedent the manifest cites, made normative).
Confidence: HIGH — Verification: read of manifest, ADR-005, doc 13 card 7.A2, platform-synthesis.md; Grep confirmed no dependency record exists for any hand-rolled pulse surface.

### F-002 [S3]
Where: crates/platform-linux/Cargo.toml:29-46
Code:
```
[dependencies]
audio-output = { path = "../audio-output" }
media-types = { path = "../media-types" }
```
What is wrong: Nothing depends on this crate. Grep for `platform-linux =` as a dependency key across every Cargo.toml → 0 matches; the `lumen` CLI's dependency list (apps/cli/Cargo.toml:48-63) contains no platform crate; `lumen-web` neither (apps/web/Cargo.toml:54-71); `lumen-tauri` depends only on desktop/commands/tauri/serde. The whole "Linux platform adapter" — PulseAudio enumeration, DRM sysfs walking, D-Bus/zenity dialogs — is compiled into every `cargo build --workspace` and linked into nothing. The manifest's own "Runtime-tested on Linux: capability negotiation, fallback chains, event-queue plumbing, dlopen probe + absent-server typed errors" (lines 25-27) is recorded claim only — no per-crate baseline run exists.
Why it matters: Third orphan crate found in this batch (plugin-host-wasm, device-discovery, platform-linux): L5 adapter work is landing ahead of any consumer, so the default workspace lanes keep paying compile/test cost for code with zero product reach. Card 7.A2's AC ("device-enum tests on the … runner") is CI-lane shaped; no committed run log exists.
What correct looks like: Wire the adapter into a host (the doc 01 §2.7 composition) or gate it out of default-members until its consumer card lands; commit a CI run log for the runtime claims.
Confidence: HIGH — Verification: read + Grep dependency scan + baseline INDEX (no platform-linux-specific run).

### F-003 [S4]
Where: crates/platform-linux/Cargo.toml:8-11
Code:
```
# admission record, device-discovery is issue #42 — the manifest is an
# allowlist, those edges join when their cards land. Edges point
```
What is wrong: The manifest descopes silently against its card: card 7.A2 says "GPU via wgpu adapter" — the manifest implements "DRM adapter enumeration is pure std sysfs walking" (line 17) with the wgpu edge "Lead-gated" (line 9). The card text in doc 13 was never updated, so plan-of-record and implementation disagree on what "GPU adapter enumeration" means (wgpu vs sysfs).
Why it matters: Card-vs-code drift compounds: an auditor reading doc 13 believes the Linux GPU lane is wgpu-based; the code walks sysfs. Same class as the device-discovery registry drift (its audit F-001) — the prose artifacts go stale and no gate diffs them.
What correct looks like: Doc 13 card row annotated "descoped: sysfs v0, wgpu edge Lead-gated (see manifest)"; or a card-status column maintained by the xtask feature-audit instead of free text.
Confidence: HIGH — Verification: read of both files.

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `[package]` name/version/edition/publish | 29-34 | REAL | — | `platform-linux` 0.1.0, edition 2024, publish=false; description matches scope (PulseAudio enum + DRM + dialogs). |
| `[lints] workspace = true` | 36-37 | REAL | — | Binds (baseline #11 39/39); crate clippy-clean (baseline #5) — all unsafe confined to the 3 allowed modules (verified). |
| `audio-output` dep | 40 | REAL | — | L5→L4 downward edge; declared in boundaries:474; used by the AudioBackend impls (src auditor verifies). |
| `media-types` dep | 41 | REAL | — | L5→L0 downward; declared; used (AudioFormat per header). |
| `testkit` dev-dep | 46 | REAL | — | Matches boundaries dev_deps:475. |
| (unsafe escapes, context) | src/linux/{pulse,pulse_ffi,dl}.rs | REAL | — | 3× `#![allow(unsafe_code)]` + 1× `#[allow(clippy::struct_field_names)]` (pulse_ffi.rs:199, style escape inside an FFI module — within the manifest's stated scope). |

## 5. Test honesty (if tests exist in/for this file)
No tests in this file. The crate's suite ran inside baseline #3 (workspace); the "absent-server typed errors" path is testable headless (dlopen fails → typed Unavailable) — plausible in the suite, per-crate breakdown NOT in baseline. Src auditor must confirm the dlopen-failure test actually executes without libpulse present rather than asserting on a stub.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "PulseAudio (`[VERIFY]` crates) device enum … same audit bar" | doc 13:705 (card 7.A2) | FALSE (as implemented) | No crate admitted; hand-rolled FFI (manifest:14-16); no record/allowlist entry exists (Grep docs/dependencies → none for a pulse surface) — see F-001. |
| "GPU via wgpu adapter" | doc 13:705 | FALSE (descoped) | Manifest:17 says sysfs walking; wgpu edge Lead-gated (manifest:9) — F-003. |
| "module-level `#[allow(unsafe_code)]` on the FFI modules only" | manifest:22-24 | REAL | Exactly 3 sites, all FFI modules (pulse.rs:34, pulse_ffi.rs:26, dl.rs:24); deny binds elsewhere; clippy CLEAN (baseline #5). |
| "Runtime-tested on Linux: capability negotiation, fallback chains, event-queue plumbing, dlopen probe + absent-server typed errors" | manifest:25-27 | REAL-UNPROVEN | Recorded claim; workspace suite green (baseline #3) but no per-crate or on-Linux-device run log committed. |
| Registry row: platform-linux deps = media-types, codec-api, audio-output, renderer-wgpu, device-discovery | doc 01 §2.7 (:245-246 "same as above") | REAL (target) / partial now | Manifest declares 2 of 5, each missing edge tied to a named card/issue (manifest:8-11) — honest allowlist posture, boundaries.toml:472-477 matches the 2. |

## 7. Dead / duplicated / unreachable code
- The whole crate is unreachable from shipped entry points (F-002): zero dependents.
- No duplicated deps; no bins; no build.rs.

## 8. Dependencies & flags
- Internal: audio-output (L4), media-types (L0) — both downward, both declared both places. Dev: testkit. External: none (hand-rolled FFI is the "external" surface — the point of F-001: it bypasses the external-dependency machinery by construction).
- Features: none. deny.toml interplay: nothing to check — no crates. The libloading ban (deny.toml:189) does not fire because the dlopen shim is hand-rolled (dl.rs) — the ban's stated intent ("no dlopen path into C code" for media engines, deny.toml:176-178) is enforced only against crate-shaped dlopen.

## 9. Open questions for the next agent
- Src auditor for crates/platform-linux/src/linux/**: verify "every block safety-commented" — count unsafe blocks vs safety comments; verify the zenity/kdialog fallback chain can't hang (subprocess timeouts).
- Governance owner (doc 11 / deny.toml auditor): is hand-rolled extern "C" + dlopen anywhere else? Grep `extern "C"` across crates outside platform-*/mem-audit to size the invisible-to-deny C surface. The codecs hw/{videotoolbox,vaapi} paths (baseline hygiene: unsafe concentrated there) deserve the same question.
