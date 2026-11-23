# AUDIT: crates/codecs/src/video/h264/hw/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 628 / 12 prod + 3 tests |
| File verdict | UNREACHABLE (real, tested seam + policy machinery; no shipped entry point constructs it) |
| Reachable from shipped entry point? | NO — `installed_bridges`/`negotiate_lane` are consumed only by `H264Decoder::from_setting` (decode.rs:310), whose only callers are tests (crates/codecs/tests/h264_hw_path.rs:861,897,911,956); the shipped CLI hardwires `"h264" \| "avc1" => self.seeded(track, config, H264SwDecoder::new)` (apps/cli/src/pipeline_host.rs:346) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | rg reachability traces (prior-agent cross-check W06-A01 log:39 + this run); sed/od quote verification; baseline cites |

## 1. One-paragraph verdict
Well-built scaffolding around a void: the contracts (`H264HwDecoder`/`H264HwBridge`), the normalized feed model (raw NALs + parsed SPS/PPS/slice facts), the bridge-walk `H264HwPath`, and the policy negotiation are all real, type-honest, and unit-tested (baseline :665-667 pass, including the exact OFF-default event-trail shape). But the entire module is dead in the shipped product: the only consumer of its wiring functions is `H264Decoder::from_setting`, which nothing outside the codecs test suite calls; the CLI always constructs the software decoder for H.264 regardless of `playback.hw-decode-policy`, so the user-facing `--playback-hw-decode-policy` knob flips nothing on this path. The module documentation also still opens with the pre-software-track honesty boundary ("No software decode lives here or anywhere under `h264`") that the same file's later comments correctly retract (super::sw exists at h264/mod.rs:56).

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:647-667: capability (7), feed (6), frame_pool (5), and hw::tests (3: `an_enabling_policy_produces_a_typed_trail`, `installed_bridges_are_exactly_the_compiled_lanes`, `the_off_default_touches_nothing_and_says_so`) all ok — the policy trail assertions are exact enum-shape matches, they would fail if the negotiation logic broke.
- The MF-lane retraction (lines 51-56) is honest documentation of missing work: "Media Foundation (Windows, 7.B2): **NOT IMPLEMENTED** — no MF lane exists in this crate".
- Cross-file reachability: rg `H264Decoder::from_setting` → only decode.rs doc comments + tests/h264_hw_path.rs. apps/: only a comment (pipeline_host.rs:342). Confirmed independently and matches W06-A01's log entry.

## 3. Findings
F-001 [S2]
Where: crates/codecs/src/video/h264/hw/mod.rs:435-439, 442-466; apps/cli/src/pipeline_host.rs:342-346
Code:
```
// The settings-driven wiring (issue #41): lane installation + policy
// negotiation. This is what turns the lanes from "a bridge type that
// exists" into "the decode pipeline's flag" — the shipped default
// (`software-only`) performs zero device or library touches.
```
What is wrong: "the decode pipeline's flag" does not exist. `installed_bridges()` and `negotiate_lane()` are called only by `H264Decoder::from_setting` (decode.rs:310-317), which is invoked only from tests/h264_hw_path.rs. The shipped CLI's h264 arm constructs `H264SwDecoder::new` unconditionally (pipeline_host.rs:346) — the `playback.hw-decode-policy` product-config key (product-config/src/model.rs:137, merged at merge.rs:418-419) is validated, merged, and then never read on any h264 path in a shipped host. A user who sets `prefer-hardware` gets the software decoder with zero probe, zero event trail, zero indication. Why it matters: a user-visible setting knob and a module's whole reason for existing ("consumption point") are unwired; compiled-in VA-API lane code ships as dead weight. What correct looks like: pipeline_host reads the negotiated policy and constructs `H264Decoder::from_setting(policy)` when enabled, or the config key is removed. Confidence: HIGH — Verification: ran rg traces (see also W06-A01 log:39 and audit/files/.../vaapi/session.rs.audit.md F-entry agreeing).

F-002 [S3]
Where: crates/codecs/src/video/h264/hw/mod.rs:10-13
Code:
```
//! **No software decode lives here or anywhere under `h264`** —
//! ADR-001's honesty boundary: a machine without an OS decoder gets
//! a typed `unsupported` error from [`super::decode`], never a fake
//! success, never a silent skip.
```
What is wrong: false since the 2026-11-15 software-track landing. `crates/codecs/src/video/h264/mod.rs:56` declares `pub mod sw;` (the rust_h264-backed `H264SwDecoder`), and this very file's comment at lines 497-501 says "a no-hardware outcome now falls back to software TRUTHFULLY". The header contradicts the body of its own module. Any reader trusting the header (ADR-001 honesty boundary) would wrongly conclude H.264 decode refuses without hardware. Confidence: HIGH — Verification: read both sites; sw.rs exists in the directory listing.

F-003 [S4]
Where: crates/codecs/src/video/h264/hw/mod.rs:507-519
Code:
```
    // The OS's lane label where one compiles; the nominal tag
    // elsewhere (every trail it appears in on those targets carries
    // `WrongOs`/`DisabledByPolicy` — the reason is the truth, not
    // this label). Windows's nominal tag is MediaFoundation: the
    // 7.B2 lane this crate has not implemented (its builds compile
    // no in-tree lane — see the module docs' honest status).
```
What is wrong: `nominal_lane_backend()` reports `MediaFoundation` on Windows — a backend that does not exist in this crate — as the lane label in probe trails. It is documented as nominal, and every trail carrying it also carries a typed reason, so no lie reaches the user outright; still, labeling an unimplemented backend into event trails where a `None`/`Unknown` label would do is a small name-vs-reality debt. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| H264HwAccessUnit::first_vcl | 202-208 | REAL | — | find_map over units |
| Display for H264HwError / H264HwOpenError | 257-265, 346-358 | REAL | — | |
| H264HwPath::open | 386-401 | REAL | — | bridge walk; typed NoAccelerator/NoBridge |
| H264HwPath::backend_id/configure/decode/flush | 405-432 | REAL | — | thin delegation to session |
| installed_bridges | 450-466 | REAL (unreachable in prod) | S2 | F-001; cfg-exact per-OS lanes (test-pinned) |
| negotiate_lane | 483-502 | REAL (unreachable in prod) | S2 | F-001; OFF touches nothing (test-pinned) |
| nominal_lane_backend | 507-520 | REAL | S4 | F-003 |
| discover_lane (3 cfg arms) | 526-551 | REAL | — | linux/mac arms delegate to lane probes; other-OS typed WrongOs |
| tests (3) | 558-627 | REAL | — | exact trail-shape assertions |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| installed_bridges_are_exactly_the_compiled_lanes | YES | per-OS count + backend_id equality |
| the_off_default_touches_nothing_and_says_so | YES | exact 2-event trail shape, accelerator None, fallback Software |
| an_enabling_policy_produces_a_typed_trail | YES (host-dependent arm) | first event Probed always; absence → Software fallback — but the accelerator-present arm can never run in this GPU-less sandbox, so the "yes" half is UNVERIFIED here (baseline ran it: ok) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No software decode lives here or anywhere under `h264`" | hw/mod.rs:10-13 | FALSE | h264/mod.rs:56 `pub mod sw;` (F-002) |
| "Media Foundation (Windows, 7.B2): NOT IMPLEMENTED — ... retracted" | hw/mod.rs:51-56 | TRUE (honest retraction) | no MF module under h264/hw/ (ls) |
| "the shipped default (`software-only`) performs zero device or library touches" | hw/mod.rs:438-439, 471-473 | REAL | the_off_default test (baseline :667) — but moot in production since the wiring is never called (F-001) |
| "This is what turns the lanes ... into the decode pipeline's flag" | hw/mod.rs:436-437 | FALSE in shipped product | no from_setting caller outside tests (F-001) |
| "[super::decode::H264Decoder::from_setting] installs this list" | hw/mod.rs:446 | TRUE-but-testonly | rg: only tests/h264_hw_path.rs |

## 7. Dead / duplicated / unreachable code
The whole module's production wiring (installed_bridges, negotiate_lane, H264HwPath::open via from_setting) is unreachable from lumen-cli/desktop (F-001). The trait contracts ARE consumed by the vaapi/videotoolbox lanes and the test fake session.

## 8. Dependencies & flags
No cargo features gate this module — `pub mod hw` is unconditional in h264/mod.rs (only per-OS cfg inside). codec-h264-sw feature gates `pub mod sw` (the contradiction subject). Release lane (Linux) compiles the vaapi lane in (baseline build log compiles codecs), shipping dead lane code.

## 9. Open questions for the next agent
- capability.rs / feed.rs / frame_pool.rs are separate files (other agents; baseline tests pass) — their reachability verdict inherits F-001.
- Is there any planned CLI flag wiring (issue #41) in a branch? The settings schema exports `cliFlag: "--playback-hw-decode-policy"` (per W06-A01), which makes F-001 user-visible.
