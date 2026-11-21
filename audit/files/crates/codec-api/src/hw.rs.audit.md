# AUDIT: crates/codec-api/src/hw.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-21 |
| Lines / functions | 73 / 4 (trait + 2 methods, MockAccelerator, 1 test) |
| File verdict | UNREACHABLE (real seam, real adapters exist, no shipped path constructs one) |
| Reachable from shipped entry point? | NO — the engine constructs every `DecoderConfig` with `accelerator: None` (playback-engine/src/pipeline.rs:1706, read); `H264Decoder::from_setting` — the only production constructor that negotiates one — has zero non-test callers (`rg "from_setting\("` → capability.rs def, h264_hw_path.rs tests only); product.toml `features.hw-decode = false` (:168) and `hw_decode_policy = "software-only"` (:191); the UI renders the policy control VISIBLE-BUT-DISABLED with the honest gate note (ui/src/screens/settings/areas/codecs.tsx:5-12, read). Trait exercised only by tests: `hw::tests::accelerator_identity_and_downcast_round_trip` ok (baseline log :255). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | Read tool full file; `rg "HwAccelerator for"` over crates/+apps/ (2 real impls: h264/hw/vaapi/mod.rs:151 VaApiAccelerator, h264/hw/videotoolbox/mod.rs:112 VideoToolboxAccelerator + 5 test mocks); `rg "from_setting\("` (no production caller); Read pipeline.rs:1695-1710; `rg "target_os" h264/hw/mod.rs` (:94 linux vaapi, :96 macos videotoolbox); `rg -c unsafe` over the two lanes (~84 sites); `rg "hw-decode-policy"` over apps/ui/settings-schema + Read codecs.tsx:1-30 (visible-disabled-honest pattern); baseline :255, :568-569 |

## 1. One-paragraph verdict
A textbook opaque seam: two methods (a stable backend label for ADR-001's truthful active-decoder report, and an `as_any` downcast for the adapter that knows its concrete type), a documented `Send + Sync` supertrait with the actual compositional reason (Arc-shared across the decode worker and the reporting thread), and no platform types — exactly what the docs promise. The trait is real and implemented by two genuine per-OS adapters (VA-API on Linux, VideoToolbox on macOS). The defect is what ships around it: the Linux `lumen` binary compiles the entire VA-API lane — ~84 `unsafe` FFI sites — despite no shipped path ever constructing an accelerator (the engine hard-codes `None`; the only negotiating constructor has zero non-test callers). The product surface is honest about this (feature off, policy control visible-but-disabled); the binary is not (dead unsafe code linked in).

## 2. What actually works (verified)
— `hw::tests::accelerator_identity_and_downcast_round_trip` ok in baseline (test-workspace.log.txt:255): identity label, successful downcast to the concrete mock, and a negative downcast to `String` — the erasure pattern works in both directions.
— The downcast-consumer pattern is real end-to-end in tests: `VaApiBridge::open` (h264/hw/vaapi/mod.rs:162-170, read) downcasts `Arc<dyn HwAccelerator>` → `VaApiAccelerator` → mints a session; baseline `video::h264::hw::tests::installed_bridges_are_exactly_the_compiled_lanes` ok (:569) proves the compiled lane registers its bridge.
— `Send + Sync` bound verified necessary by usage: `DecoderConfig.accelerator: Option<Arc<dyn HwAccelerator>>` crosses onto the decode worker (config.rs:89 + the engine's construction path).

## 3. Findings

### F-001 [S3]
Where: crates/codec-api/src/hw.rs:32-44 (the seam) with the compiled-but-dead adapter surface at crates/codecs/src/video/h264/hw/mod.rs:94 (`#[cfg(target_os = "linux")]` vaapi) and :96 (macos videotoolbox)
Code:
```
pub trait HwAccelerator: Send + Sync {
```
What is wrong: not the trait — the shipping consequence. The VA-API lane (`#[cfg(target_os = "linux")]`, so compiled into the baseline Linux build and every Linux release artifact) carries ~84 `unsafe` FFI sites (vaapi/{ffi,library,session,dl,device}.rs unsafe counts 22+24+30+7+1), and the VideoToolbox lane likewise on macOS — while the runtime graph can never reach any of it: the engine constructs `DecoderConfig` with `accelerator: None` unconditionally (pipeline.rs:1706), and the only constructor that negotiates (`H264Decoder::from_setting`, h264/decode.rs:310) is called exclusively by tests. The unsafe is also never executed even by the test suite on this machine beyond unit-shape tests (the lane's own tests mock the FFI boundary).
Why it matters: the shipped binary links and carries a dead unsafe FFI surface (dynamic-loader machinery `dl.rs`, library bindings, session state) with zero user-reachable benefit — pure attack/linker surface and binary size, and an audit-surface tax (every unsafe review of the vaapi lane reviews code no user can reach). The honest-gating story (product.toml `hw-decode = false`, UI control disabled with the reason) is about the SETTING; nothing gates the CODE.
What correct looks like: gate the adapter lanes behind a `codec-h264-hw` Cargo feature (default-off) matching product.toml's feature flag — the tree already has the exact pattern (`codec-h264-sw` gates the sw engine; codecs/Cargo.toml:73) — so the unsafe lanes compile only when a build actually wires them. Alternatively accept the dead weight with a recorded decision.
Confidence: HIGH — Verification: read (cfg gates, unsafe census, engine construction, from_setting caller census); baseline build #1 compiled the Linux lane (workspace build would fail otherwise) — the unsafe count is from `rg -c`, exact site-by-site review belongs to the lane owners' audits (hw/vaapi, hw/videotoolbox are other agents' files).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `trait HwAccelerator` (decl + supertraits) | 32 | REAL | — | Send+Sync rationale documented and verified compositional |
| `HwAccelerator::backend_id` | 37 | REAL | — | label-only, no capability semantics — honest |
| `HwAccelerator::as_any` | 43 | REAL | — | `[PROPOSAL]` downcast seam; "unknown backend is a typed configure error, never a panic" — verified true at the consumer (VaApiBridge::open returns None → H264HwPath::open walks on) |
| `MockAccelerator` (test) | 51-61 | REAL | — | minimal |
| `accelerator_identity_and_downcast_round_trip` | 63-72 | REAL | — | positive + negative downcast; proportionate for a 2-method trait |

## 5. Test honesty
1/1 ok (baseline :255).
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| accelerator_identity_and_downcast_round_trip | YES (for the erasure pattern) | a broken `as_any` (returning e.g. `&dyn Any` of the wrong coercion site) fails the concrete downcast; the negative `String` downcast pins non-panicking rejection. It cannot catch trait-design issues (there are none to catch at 2 methods). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the trait names no API, no device, no surface format" | hw.rs:6-7 | REAL | the trait body is 2 methods, neither platform-typed |
| "Instances exist only behind the audited per-OS adapters (E2 exception); software codecs never see one" | hw.rs:19-21 | REAL | engine passes None (pipeline.rs:1706); the two real impls live in the per-OS lanes; no sw decoder reads `accelerator` except to refuse (h264/hevc hw decoders are the consumers) |
| "Concrete adapters (Phase 7.B) implement it" | hw.rs:8-9 | REAL | they exist: VaApiAccelerator (vaapi/mod.rs:151), VideoToolboxAccelerator (videotoolbox/mod.rs:112) — though no shipped path constructs them (F-001) |
| "`backend_id` … is a label for UI/engine queries, not a capability switch" | hw.rs:34-37 | REAL | no consumer branches on the string beyond display (config.rs Debug :122-125) |

## 7. Dead / duplicated / unreachable code
The seam itself: production-unreachable (header row) — by design until Phase-7 wiring, honestly disclosed at the product layer. The compiled adapter lanes are the S3 dead-weight concern (F-001). `MockAccelerator` is duplicated identically in config.rs:192-202 (test-only duplication, trivial).

## 8. Dependencies & flags
None beyond std (`std::any::Any`). No unsafe IN THIS FILE — the unsafe lives in the adapter lanes that implement it (F-001). No feature gate exists for the hw lanes (the gap F-001 names).

## 9. Open questions for the next agent
- h264/hw/vaapi + videotoolbox owners: your unsafe-site audits are auditing code that cannot execute in any shipped build (F-001) — worth stating in your file verdicts so the wave summary doesn't double-count the risk surface.
- Whoever owns the Phase-7 wiring: when `hw-decode` flips true, `H264Decoder::from_setting` needs a production caller AND the adapter lanes an execute-audit; until then the cargo-feature gate is the cheap fix.
- product.toml audit (already done, audit/files/product.toml.audit.md): cross-check whether it flags the compiled-vs-exposed gap; if not, F-001 here is the citation.
