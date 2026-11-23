# AUDIT: crates/codecs/src/video/hevc/hw.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 403 / 8 prod (contract-only module; no logic) |
| File verdict | UNREACHABLE (real, honestly-scoped contract definitions; only in-tree trait implementors are the test fakes; no shipped host constructs the HEVC decoder) |
| Reachable from shipped entry point? | NO — `HevcHwPath::open` is called only by `super::decode::HevcDecoder` (decode.rs:568), and `HevcDecoder` is constructed only in crates/codecs/tests/hevc_hw_path.rs; apps/ has zero `HevcDecoder` references and pipeline_host.rs has no "hevc" arm (opening HEVC yields the typed `no decoder for hevc` refusal, baseline mp4-smoke exit 65) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg for impls/callers/apps references (exit 0); baseline cite; sed quote verification |

## 1. One-paragraph verdict
This is the HEVC mirror of the H.264 hw seam, honestly scoped: unlike its H.264 sibling it claims nothing it does not have — the module docs say the per-OS adapters (VA-API/VideoToolbox/MF) are "not built here" and that "the in-tree proof is the fake session of the integration suite", which exists (tests/hevc_hw_path.rs, 17 tests green in baseline :1186-1205, covering the full session contract: AU feeding, damage counting, typed no-bridge/no-accelerator, stale drops, fuzz-lite). The definitions themselves are type-only (structs, two traits, two error enums, a thin `HevcHwPath` wrapper) with no logic to get wrong. The single real problem is the same one the H.264 lane has: none of it is reachable from a shipped entry point — the CLI has no HEVC decode arm at all, so this entire contract compiles into the release lane as dead weight until Phase 7.B exists.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:1184-1205: tests/hevc_hw_path.rs 17/17 ok — `annexb_stream_feeds_aus_and_emits_opaque_frames`, `hvcc_extradata_configures_and_length_prefixed_samples_decode`, `no_accelerator_is_typed_unsupported_never_silent`, `accelerator_without_bridge_is_typed_no_bridge`, `damage_is_counted_recovery_and_decoder_stays_usable`, `corpus_0044_annexb_full_stream_survives_and_feeds_every_picture` (0.03s — corpus-absent skip path or committed fixture; the test file is not in my scope), plus fuzz-lite.
- rg: `impl HevcHwDecoder|impl HevcHwBridge` → only crates/codecs/tests/hevc_hw_path.rs (the documented fake session).
- HEVC spec claim verified: no `field_pic_flag` in HEVC; interlaced carried via VUI — the omitted `interlaced` flag on `HevcHwFrame` is spec-correct.

## 3. Findings
F-001 [S3]
Where: crates/codecs/src/video/hevc/hw.rs:332-334; crates/codecs/src/video/hevc/decode.rs:568
Code:
```
/// The consumption point: an open hardware session fed by the
/// parser's output. Owns the [`HevcHwDecoder`] the bridge minted;
/// [`super::decode::HevcDecoder`] drives it.
```
What is wrong: the "consumption point" has no consumer. `HevcDecoder` (the only type that opens an `HevcHwPath`, decode.rs:568) is constructed exclusively by the test suite; `rg HevcDecoder apps/` returns nothing and pipeline_host.rs contains no "hevc" string — every shipped open of an HEVC stream ends in the registry refusal (`no decoder for hevc`, baseline mp4-smoke: exit 65). The module is compiled into the release lane (no feature gate on `pub mod hw`) as ~400 lines of unreachable contract code. Unlike the H.264 lane there is no user-visible knob that implies this works (h264's `--playback-hw-decode-policy` lie is absent here — HEVC simply refuses), which keeps this at S3 rather than S2: the failure mode is dead code, not a broken promise. What correct looks like: land the Phase-7 adapter lane or feature-gate the module out of the release lane. Confidence: HIGH — Verification: ran rg traces.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HevcHwAccessUnit::first_vcl | 172-178 | REAL | — | find_map |
| Display for HevcHwError | 227-235 | REAL | — | |
| Display for HevcHwOpenError | 316-328 | REAL | — | names ADR-001 truth |
| Debug for HevcHwPath | 339-347 | REAL | — | backend label, not pointer |
| HevcHwPath::open | 356-371 | REAL | S3 | bridge walk; typed NoAccelerator/NoBridge — unreachable in prod (F-001) |
| HevcHwPath::backend_id | 374-377 | REAL | — | |
| HevcHwPath::configure/decode/flush | 384-402 | REAL | — | thin delegation |
(No other functions: the rest is type/field definitions.)

## 5. Test honesty
No tests in this file. The contract's tests live in tests/hevc_hw_path.rs (other agents' scope) — the 17 baseline-green tests listed in §2 exercise these types through a fake session; they would fail if the trait contract or the AU normalization broke.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "per-OS adapter (7.B1 VA-API, 7.B2 VideoToolbox, 7.B3 Media Foundation — *not built here*)" | hw.rs:6-8 | TRUE (honest) | rg: no adapter impls outside tests |
| "the in-tree proof is the fake session of the integration suite" | hw.rs:243-244 | TRUE | tests/hevc_hw_path.rs is the only impl site |
| "No software decode lives here or anywhere under `hevc`" | hw.rs:10-11 | TRUE | ls hevc/: decode.rs is hw-session plumbing (parser+hw only); no sw engine module; CLI refuses hevc typed (baseline smoke) |
| "HEVC has no field pictures ... every emitted picture is a frame" | hw.rs:51-55, 187-188 | TRUE | spec-correct (no field_pic_flag in HEVC) |

## 7. Dead / duplicated / unreachable code
The entire module is production-dead (F-001) but test-alive. Deliberate structural duplication with h264/hw/mod.rs (frame/error/bridge types) is by design (per-codec contracts, documented mirror).

## 8. Dependencies & flags
No features; unconditional `pub mod hw` (hevc/mod.rs) → compiled into the release lane. Uses codec-api's HwAccelerator + the parser types only.

## 9. Open questions for the next agent
- decode.rs (hevc) auditor: confirm the AU boundary detection (§7.4.2.4.4) and whether `HevcDecoder` deserves a settings lane at all before 7.B lands.
- tests/hevc_hw_path.rs auditor: the corpus_0044 test passed in 0.03s — check whether it skips on absent corpus (same F-001 pattern as hevc_parser_corpus.rs) or runs on a committed fixture.
