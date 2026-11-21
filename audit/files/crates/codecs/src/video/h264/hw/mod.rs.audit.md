# AUDIT: crates/codecs/src/video/h264/hw/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-21 |
| Lines / functions | 628 (wc -l) / 32 items |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO — only consumer is `decode.rs::H264Decoder::from_setting` (codecs-internal, test-only callers); no shipped host constructs it |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 1 |
| Commands run | rg "H264Decoder\b" (callers: only decode.rs + tests + comments); rg "negotiate_lane\|installed_bridges" (only decode.rs:312,314); rg "videotoolbox\|vaapi" repo-wide (hw tree only); rg "hw_decode\|hw-decode" product.toml; rg "cargo build" release.yml; sed -n verifications of every quote; od -c line 449 (proved `#[must_use` — Bash display artifact ate `[m`, Read tool shows true bytes) |

## 1. One-paragraph verdict
This is the H.264 hardware-decode seam: session trait (`H264HwDecoder`), plug trait (`H264HwBridge`), consumption point (`H264HwPath`), per-OS lane installation (`installed_bridges`), and policy negotiation (`negotiate_lane`). The code is coherent, typed, panic-free and its three in-module tests + the 20-test `h264_hw_path.rs` integration suite all pass in baseline (`test-workspace.log.txt:568-570`, `:1029-1050`) — but every one of those tests drives a FAKE bridge/session; no real VA-API/VideoToolbox lane is ever executed (sandbox has no GPU; VideoToolbox is macOS-cfg'd out of the Linux build). More damning: **nothing in any shipped binary reaches this module**. The sole runtime consumer chain is `decode.rs::H264Decoder::from_setting` → `negotiate_lane`/`installed_bridges`, and `H264Decoder` (hw) is constructed nowhere outside codecs-crate tests; the shipped CLI hard-codes the software decoder (`apps/cli/src/pipeline_host.rs:346`), the product gate that would expose the policy is hard-off (`product.toml:168`), and the `playback.hw-decode-policy` setting is feature-gated invisible (`schemas/settings.toml:640-650`, `feature_gate` at :648). The module's repeated "honesty boundary" claims ("no software decode lives here or anywhere under `h264`") are stale falsehoods since the 2026-11-15 rust_h264 landing.

## 2. What actually works (verified)
— `rg -n "hw::tests::" audit/runs/baseline/test-workspace.log.txt` → `video::h264::hw::tests::{an_enabling_policy_produces_a_typed_trail, installed_bridges_are_exactly_the_compiled_lanes, the_off_default_touches_nothing_and_says_so}` all `ok` (:568-570). These prove: Linux build compiles exactly the vaapi lane (`hw.vaapi`), OFF policy mints no accelerator and records the typed `DisabledByPolicy` trail.
— `sed -n '1029,1050p' test-workspace.log.txt` → `h264_hw_path.rs` 20/20 ok, incl. `from_setting_software_only_default_is_off_and_refuses_typed`, `annexb_stream_feeds_aus_and_emits_opaque_frames` — the AU feed contract works against the suite's fake session.

## 3. Findings

### F-001 [S2] Entire hw subsystem unreachable from shipped builds; compiled into every Linux binary anyway
Where: crates/codecs/src/video/h264/hw/mod.rs:91-97, 450-466 (and the vaapi/videotoolbox subtrees)
Code:
```
pub mod capability;
pub mod feed;
pub mod frame_pool;
#[cfg(target_os = "linux")]
pub mod vaapi;
#[cfg(target_os = "macos")]
pub mod videotoolbox;
```
```
pub fn installed_bridges() -> Vec<Arc<dyn H264HwBridge>> {
    let bridges: Vec<Arc<dyn H264HwBridge>> = Vec::new();
```
What is wrong: No shipped entry point reaches this code. Chain of proof: (a) the only construction site of an H.264 decoder in any shipped host is `apps/cli/src/pipeline_host.rs:346` `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new),` — software, unconditionally; (b) the only callers of `negotiate_lane`/`installed_bridges` are `crates/codecs/src/video/h264/decode.rs:312,314` inside `H264Decoder::from_setting`, and `rg "H264Decoder\b"` shows `H264Decoder` (the hw one) is constructed only by codecs-crate tests — no app, no engine, no plugin; (c) the settings surface that would feed `from_setting` is gated off: `product.toml:168` `hw-decode = false` and `product.toml:191` `hw_decode_policy = "software-only"`; `schemas/settings.toml:640-650` puts `playback.hw-decode-policy` behind `feature_gate = "hw-decode"` (:648); (d) `lumen-tauri` doesn't even depend on codecs (apps/desktop-tauri/Cargo.toml deps = `desktop`, `commands`, serde; apps/desktop/Cargo.toml:43 pulls `video-engine` only).
Why it matters: The whole `hw/` tree — including the `unsafe`-dense vaapi FFI subtree (30 `unsafe` occurrences in `vaapi/session.rs` alone, per rg -c) — is compiled unconditionally into every Linux `lumen` binary and every workspace test build, yet is dead weight at runtime; reviewers and users are invited by its presence to believe hardware decode is a live lane. This is exactly the "feature flags gating real code that shipped builds never enable" pattern, inverted: no flag exists at all, the code is simply never called.
What correct looks like: Either a Cargo feature (`codec-h264-hw`, default-off) gating the vaapi/videotoolbox lanes like `codec-av1`/`codec-vp9` gate theirs (crates/codecs/Cargo.toml:60-81), or a shipped host actually consuming `from_setting` behind the `hw-decode` product gate.
Confidence: HIGH — Verification: read; rg caller traces; feature-flag trace (crates/codecs/Cargo.toml:47-59 `default` has NO hw feature — hw is unconditional by design comment lines 25-27); .github/workflows/release.yml:193/474/627/763/911 build `-p lumen-cli` with default features only (rg for `--features`/`--no-default-features` in release.yml: zero hits).

### F-002 [S2] The module's central honesty claims are now false: "no software decode lives here or anywhere under `h264`"
Where: crates/codecs/src/video/h264/hw/mod.rs:10-13, 316-319, 333-337, 349-351
Code:
```
//! **No software decode lives here or anywhere under `h264`** —
//! ADR-001's honesty boundary: a machine without an OS decoder gets
//! a typed `unsupported` error from [`super::decode`], never a fake
//! success, never a silent skip.
```
```
    /// No accelerator was negotiated at all (the config carried
    /// `None`): per ADR-001 there is no software path to fall back
    /// to — the error says exactly that.
```
```
            H264HwOpenError::NoAccelerator => f.write_str(
                "no hardware accelerator negotiated and H.264 has no software decode (ADR-001)",
            ),
```
What is wrong: Four sites assert H.264 has no software decode. Since 2026-11-15 it does: `crates/codecs/src/video/h264/sw.rs` (`H264SwDecoder`, cfg `codec-h264-sw` which is in `default`, crates/codecs/Cargo.toml:53) sits directly under `h264`, is the shipped CLI's chosen decoder (pipeline_host.rs:346), and baseline proves it plays (`audit/runs/baseline/INDEX.md` corrected matrix: sample-420.mp4 h264 CB 4:2:0 → "Buffering → Playing → Stopped → Idle"). The user-facing Display string at :349-351 states a falsehood any user hitting it would read.
Why it matters: A module whose entire raison d'être is "the honesty boundary" carries stale falsehoods; a maintainer reading `H264HwOpenError`'s message would wrongly conclude fallback is impossible. Doc/code drift of the exact class this audit exists to catch.
What correct looks like: Update all four sites to "no software fallback on THIS path; the shipped default resolves to `sw::H264SwDecoder` (2026-11-15 track)" — the correction `negotiate_lane`'s docs already half-made (see F-003).
Confidence: HIGH — Verification: read; sw.rs presence confirmed by ls; baseline INDEX.md v2 correction; test log `from_setting_software_only_default_is_off_and_refuses_typed ... ok`.

### F-003 [S3] `negotiate_lane` doc claims a resolution that no code performs
Where: crates/codecs/src/video/h264/hw/mod.rs:475-479
Code:
```
/// [`HwUnavailableReason::DisabledByPolicy`] refusal and the
/// accelerator is `None` (the session resolves to
/// [`super::sw::H264SwDecoder`] instead — the Lead-admitted software
/// track, 2026-11-15; the hw-only [`super::decode::H264Decoder`]
```
What is wrong: `negotiate_lane` returns a `Negotiation` record; nothing in this module, in `decode.rs`, or in any host "resolves the session to `H264SwDecoder`" based on it. The CLI picks `H264SwDecoder` unconditionally (pipeline_host.rs:346) without ever consulting `negotiate_lane`; `decode.rs::from_setting` merely records `negotiation.events`. The described decision procedure does not exist.
Why it matters: The doc narrates a wiring that isn't there — a reader auditing the fallback path would believe a mechanism enforces it.
What correct looks like: State the truth: "the record's `fallback: Software` is advisory; each host selects its decoder — the shipped CLI selects `H264SwDecoder` unconditionally."
Confidence: HIGH — Verification: read + rg caller trace (only decode.rs:312 consumes the Negotiation, and only for `events`/`accelerator` fields).
### F-004 [S3] VideoToolbox lane status "compile-verified via `cargo check --target aarch64-apple-darwin`" — no verifiable evidence
Where: crates/codecs/src/video/h264/hw/mod.rs:45-50
Code:
```
//! - **`videotoolbox`** (macOS, 7.B1): hand-rolled
//!   `VTDecompressionSession*` FFI + session, compile-verified via
//!   `cargo check --target aarch64-apple-darwin`; runtime hardware
//!   behavior NOT exercised — `[VERIFY]`.
```
What is wrong: The compile-verification claim is uncited and unreproducible from the repo's evidence set: baseline ran on Linux only (`audit/runs/baseline/INDEX.md`: no cross-target check; the only macOS-adjacent artifact is nothing). The `[VERIFY]` marker is on runtime behavior, not on the compile claim.
Why it matters: "Compile-verified" is the sole quality evidence offered for ~1100 lines of hand-rolled FFI (videotoolbox/mod.rs + session.rs + ffi.rs); if it never happened, the lane may not even parse for darwin. Trust-nothing rule applies.
What correct looks like: A committed CI lane or log showing the aarch64-apple-darwin check; until then the sentence should read "compile-verification UNVERIFIED".
Confidence: MEDIUM — Verification: read; absence of any darwin/cross-compile evidence in audit/runs/baseline/ (INDEX.md table has no such run). The claim could be true of a developer machine — UNVERIFIED.
### F-005 [S4] `H264Framing::Avc { length_size: u8 }` carries no 1..=4 invariant
Where: crates/codecs/src/video/h264/hw/mod.rs:119-124
Code:
```
    /// AVC length-prefixed samples (MP4/MKV), `length_size` =
    /// `lengthSizeMinusOne + 1` (1..=4).
    Avc {
        /// NAL length prefix width in bytes.
        length_size: u8,
    },
```
What is wrong: The doc promises 1..=4; the type accepts any u8 (0, 255). Enforcement is delegated to the parser (`feed.rs`/`nal.rs`, other agents' files) — the carrier itself is a lying-by-documentation type.
Why it matters: Limited blast radius (single consumer), but it is exactly the "naming that lies" class; a future constructor could mint an invalid framing that downstream code trusting the doc would mishandle.
What correct looks like: A `new` constructor validating the range, or a doc note "callers must validate; see avcC parse".
Confidence: HIGH — Verification: read; no validation code in this file.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `H264Framing` (enum) | 115-125 | REAL-UNPROVEN | S4 | Carrier enum; AnnexB/Avc; no length_size invariant (F-005). Consumed by decode.rs/feed.rs and lanes. |
| `H264HwSps` (struct) | 130-136 | REAL-UNPROVEN | — | nal bytes + parsed SPS; used by lanes/tests. |
| `H264HwPps` (struct) | 139-145 | REAL-UNPROVEN | — | Symmetric twin of Sps. |
| `H264HwStreamInfo` (struct) | 149-161 | REAL-UNPROVEN | — | `Default` derive; `framing: Option`; fed by decode.rs. |
| `H264HwVcl` (struct) | 165-171 | REAL-UNPROVEN | — | slice header + NAL projection. |
| `H264HwUnit` (struct) | 174-183 | REAL-UNPROVEN | — | raw NAL + type + optional vcl. |
| `H264HwAccessUnit` (struct) | 188-200 | REAL-UNPROVEN | — | units/idr/pts/display_size; built by decode.rs AU assembler. |
| `H264HwAccessUnit::first_vcl` | 202-208 | REAL-UNPROVEN | — | find_map over units; trivial, correct. |
| `H264HwFrame` (struct) | 217-227 | REAL-UNPROVEN | — | Opaque u64 surface handle + geometry; matches media-types FrameStorage::OpaqueHandle contract. |
| `H264HwDecoded` (struct) | 232-236 | REAL-UNPROVEN | — | frames vec; Default. |
| `H264HwError` (enum) | 240-255 | REAL-UNPROVEN | — | Configure/Decode{code}/DeviceLost; typed, no panic paths. |
| `impl Display for H264HwError` | 257-265 | REAL-UNPROVEN | — | Straightforward formatting. |
| `impl Error for H264HwError` | 267 | REAL-UNPROVEN | — | Empty impl, fine. |
| `trait H264HwDecoder` | 278-307 | REAL-UNPROVEN | — | backend_id/configure/decode/flush; synchronous, bounded; implemented only by fake (tests) + vaapi/videotoolbox sessions (compile-only on Linux). |
| `trait H264HwBridge` | 320-328 | REAL-UNPROVEN | — | backend_id/open; doc block :316-319 stale (F-002). |
| `H264HwOpenError` (enum) | 332-344 | REAL-UNPROVEN | — | NoAccelerator doc stale (F-002). |
| `impl Display for H264HwOpenError` | 346-358 | REAL-UNPROVEN | — | User-facing string asserts "no software decode" — false (F-002). |
| `impl Error for H264HwOpenError` | 360 | REAL-UNPROVEN | — | Empty impl. |
| `H264HwPath` (struct) | 365-367 | REAL-UNPROVEN | — | Owns Box<dyn H264HwDecoder>. |
| `impl Debug for H264HwPath` | 369-377 | REAL-UNPROVEN | — | Prints backend label, not pointer; honest. |
| `H264HwPath::open` | 386-401 | REAL-UNPROVEN | — | Walks bridges; None-accel → typed NoAccelerator; full miss → NoBridge{backend}. Correct; test `accelerator_without_bridge_is_typed_no_bridge` ok in baseline. |
| `H264HwPath::backend_id` | 404-407 | REAL-UNPROVEN | — | Delegates. |
| `H264HwPath::configure` | 414-416 | REAL-UNPROVEN | — | Delegates. |
| `H264HwPath::decode` | 422-424 | REAL-UNPROVEN | — | Delegates. |
| `H264HwPath::flush` | 430-432 | REAL-UNPROVEN | — | Delegates. |
| `installed_bridges` | 450-466 | REAL-UNPROVEN | S2 | cfg-per-OS push; on Linux installs vaapi bridge unconditionally — dead in shipped builds (F-001). Test-verified count (baseline :569). |
| `negotiate_lane` | 483-502 | REAL-UNPROVEN | S3 | OFF → typed DisabledByPolicy trail (baseline-verified); doc overclaims a resolution mechanism (F-003). |
| `nominal_lane_backend` | 507-520 | REAL-UNPROVEN | — | cfg-return; Windows → MediaFoundation nominal label, honest given no MF lane. |
| `discover_lane` (linux arm) | 526-530 | REAL-UNPROVEN | — | Delegates to vaapi::discover; never called in shipped runtime (policy off). |
| `discover_lane` (macos arm) | 534-538 | REAL-UNPROVEN | — | Not compiled on Linux baseline host. |
| `discover_lane` (other arm) | 542-551 | REAL-UNPROVEN | — | WrongOs truth; fine. |
| test `installed_bridges_are_exactly_the_compiled_lanes` | 558-576 | REAL | — | Ran ok (baseline :569); would fail if a lane were added/removed. |
| test `the_off_default_touches_nothing_and_says_so` | 578-604 | REAL | — | Ran ok (baseline :570); asserts full trail shape via `unreachable!` arm — would fail on drift. |
| test `an_enabling_policy_produces_a_typed_trail` | 606-627 | REAL | — | Ran ok (baseline :568); host-independent trail shape only — proves nothing about real hardware (and can't, honestly). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| installed_bridges_are_exactly_the_compiled_lanes | YES | len+backend_id equality against cfg'd expectation; removing vaapi or adding a lane flips it. |
| the_off_default_touches_nothing_and_says_so | YES | Destructures the exact 2-event trail; a `DisabledByPolicy`-on-enable bug or missing event hits `unreachable!` arm. |
| an_enabling_policy_produces_a_typed_trail | WEAK | Only asserts first event is `Probed` and absent-accel → fallback Software; a trail that records garbage after event 1 still passes. Mutation test: delete the `Inactive` event emission — this test still passes, `the_off_default...` catches it instead. |
| h264_hw_path.rs (integration, other file) | YES (for the fake lane) | 20 tests incl. AU assembly, damage counting, typed refusals — but ALL against the suite's fake bridge/session; zero coverage of vaapi/videotoolbox FFI. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "No software decode lives here or anywhere under `h264`" | hw/mod.rs:10-13 module docs | FALSE | sw.rs exists (h264/mod.rs:55-56 `#[cfg(feature = "codec-h264-sw")] pub mod sw;`), shipped CLI uses it (pipeline_host.rs:346), baseline plays 4:2:0 MP4 (INDEX.md v2). |
| "compile-verified via cargo check --target aarch64-apple-darwin" (videotoolbox) | hw/mod.rs:46-48 | REAL-UNPROVEN | No cross-check run exists in audit/runs/baseline/; claim is uncited. |
| "Media Foundation (Windows, 7.B2): NOT IMPLEMENTED" | hw/mod.rs:51-56 | REAL | rg for MF FFI in tree: absent; honest retraction note. |
| hw lanes "Activation [PROPOSAL] pending ADR-013" (vaapi) | hw/mod.rs:62-63 | REAL | Consistent with product.toml:168 hw-decode=false and F-001 unreachability. |
| features/feature-matrix row notes "hw row's lane (H264Decoder::from_setting) unchanged — software is the shipped-default path" | docs/feature-matrix.toml:429 | REAL | Matches pipeline_host.rs:346 + product.toml:191. |

## 7. Dead / duplicated / unreachable code
The entire runtime surface of this file (F-001): `installed_bridges`, `negotiate_lane`, `discover_lane`, `H264HwPath::*` are reachable only from `decode.rs::from_setting`, itself reachable only from codecs-crate tests (`tests/h264_hw_path.rs`, `from_setting_*` in baseline logs). Shipped binaries: `lumen` (CLI) — no; `lumen-tauri` — no (does not depend on codecs).

## 8. Dependencies & flags
- `codec_api::HwAccelerator`, `media_types::Stamp`, parser types — all in-crate/workspace; no external deps. No Cargo feature exists for hw (crates/codecs/Cargo.toml:42-81 — gates exist only for external engines + `codec-h264-sw`); hw tree is unconditional by documented choice (Cargo.toml:25-27, 71-72). product.toml:168 `hw-decode = false` is the only product-level gate, and it gates a setting, not compilation.
- Wave-6 hostile question, answered for this file: H.264 decode reachable from shipped builds = SOFTWARE only (`H264SwDecoder`, default feature `codec-h264-sw` ON via apps/cli/Cargo.toml:51 → crates/codecs/Cargo.toml:47-59; release.yml:193 `cargo build --release --locked -p lumen-cli -p desktop` with no feature subset; proven working by baseline open-session-420). HARDWARE decode (this file + vaapi/videotoolbox) = UNREACHABLE: no constructor in any shipped host; `playback.hw-decode-policy` is `feature_gate = "hw-decode"` (schemas/settings.toml:648) and product.toml:168 sets that gate false.

## 9. Open questions for the next agent
- decode.rs (hw consumer) audit: does `H264Decoder::configure`'s `h264.hw.required` refusal path and its AU assembler have panics? (Not assigned to me.)
- Does any future plugin/registry plan (doc 01 §2.5(b) `register()`) intend to expose `from_setting`? If yes, F-001's dead-code verdict changes from "remove" to "wire or gate".
- vaapi/session.rs (A10) + videotoolbox/session.rs (mine, part2): unsafe invariants — cross-check against F-001 (dead code lowers the severity of any unsafe bug, but not to zero: workspace `--all-features` builds and any future wiring resurrect it).
