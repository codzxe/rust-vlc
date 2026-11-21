# AUDIT: crates/plugin-api/src/codec.rs
| Field | Value |
|---|---|
| Auditor | W12-A08 |
| Date | 2026-11-21 |
| Lines / functions | 216 / 3 API items (1 trait ×2 methods, 1 fn, 1 private error type) + re-export block + 2 test impls + 2 tests |
| File verdict | UNREACHABLE (the plugin seam is dead: `CodecPlugin` and `codec_claim_mismatch` have zero implementors/callers; the re-exported codec-api traits underneath are the REAL shipped decode lane) |
| Reachable from shipped entry point? | NO for this module's own items. `rg "CodecPlugin\|codec_claim_mismatch" --glob '!crates/plugin-api/**'` → docs/worklog only, zero code. `rg "plugin_api::codec\|plugin_api::.*Decoder" apps crates --glob '!crates/plugin-api/**'` → zero (the CLI's decoders come from the `codecs` crate implementing `codec_api::Decoder` directly — 8+ impl files). The re-exports point at REAL items (`codec_api::decode::Decoder` etc., verified: codec-api/src/decode.rs, encode.rs:34 `pub trait Encoder`) — but nothing imports them through plugin-api. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg "CodecPlugin\|codec_claim_mismatch"` repo-wide (docs + own tests only) · `rg "plugin_api::codec\|plugin_api::.*Decoder"` (no consumers of the re-export surface) · `rg -ln "impl Decoder for" crates/codecs/src` (the real lane: aac/mp3/mp2/opus/pcm/alac/adpcm/flac decoders implement codec-api directly) · `rg -ln "impl Encoder for"` (only codec-api's own contract test doubles — no production encoder) · `ls crates/codec-api/src/` (decode.rs/encode.rs exist; re-export paths verified by the CLEAN workspace build, INDEX #1) · `rg -n "conformance" crates/plugin-host-wasm` (plugin_examples_conformance.rs exists — example-plugin tests only, does not consume CodecPlugin) · `sed -n '13,17p;59,62p'` quote verification · baseline INDEX #1 (build CLEAN) + #3 (plugin-api tests green) |

## 1. One-paragraph verdict
Unlike playlist.rs, this module sits on top of a genuinely living contract: the `codec_api::Decoder` it re-exports is implemented by the codecs crate's real decoders and drives the shipped first-vertical lane. But everything THIS file adds — the `CodecPlugin` factory marker ("how a codec-kind plugin presents itself to the registry") and the `codec_claim_mismatch` typed error — is dead: no registry exists (doc 06 §3 is the unstarted Phase-2 card), no type implements `CodecPlugin` outside this file's own test toy, nothing calls the error constructor, and the one-import-surface re-export block serves a plugin-author population of zero (the real consumers import codec-api or codecs directly). The tests are honest toys that prove the marker→trait wiring and the error's stage/source chain. The file is a correct 60-line shim around a real contract, waiting for a consumer that no shipped binary contains.

## 2. What actually works (verified)
— Re-export fidelity: the three `pub use` lines (19-21) point at real codec-api items; the workspace build is CLEAN (INDEX #1), so the paths resolve; `codec_api::decode::Decoder` is the trait the shipped decoders implement (rg: 8+ impl files under crates/codecs/src).
— `factory_constructs_a_working_decoder` (153-190): instantiates via the marker, configures, decodes one packet, and asserts sink-count and outcome agree (`Emitted{frames:1}` + `counting.frames == 1`) — mutation: if `instantiate` returned a broken decoder or the outcome/sink disagreed, the double assert fails. Green in baseline (INDEX #3 plugin-api suite).
— `claim_mismatch_is_a_plugin_stage_error` (203-215): pins `PipelineStage::Plugin` AND the source-chain text naming both codecs (claimed + served). Bites on stage or source regressions.

## 3. Findings

### F-001 [S3] The plugin seam this module exists for is dead: `CodecPlugin` has zero implementors, `codec_claim_mismatch` zero callers, the registry it faces does not exist
Where: crates/plugin-api/src/codec.rs:13-16, 39-44, 59-62
Code:
```rust
//! - the [`CodecPlugin`] marker: how a codec-kind plugin *presents*
//!   itself to the registry (factory surface), keeping the decode
//!   contract itself untouched (doc 06 §1.2 separates the trait from
//!   the plugin registration seam);
```
and
```rust
/// Typed registry-side error: the plugin's claimed codec does not
/// match what it serves (a manifest lie — doc 06 §7.2 blocks shipping
/// on it).
```
What is wrong: `rg "CodecPlugin"` outside plugin-api → zero code hits (docs/worklog only). `rg "codec_claim_mismatch"` → same. The "registry" is doc 06 §3's Phase-2 card — no registry code exists in the tree (wave-12 corroborated: plugin-host-wasm is a dependency of no app). The §7.2 conformance kit that would *fire* this error exists only as plugin-host-wasm's example-plugin tests (`tests/plugin_examples_conformance.rs`), which do not consume this API.
Why it matters: The doc sells the marker as the registration seam and the error as the shipping-blocker — neither is wired to anything. A reader concluding "codec plugins register through CodecPlugin and manifest lies are blocked at §7.2" is wrong on both counts today: no codec plugin exists, and nothing enforces claim-mismatch anywhere. Dead contract with an enforcement narrative is worse than plain dead code.
What correct looks like: Either land the registry card that consumes this seam, or annotate the marker/error as staged-until-Phase-2 in the doc comment (the lib.rs scope fence already says "no registry" — this module's doc undermines it by describing the registry-facing behavior in the present tense).
Confidence: HIGH — Verification: read + repo-wide rg both symbols + doc 06 §3/§7 cross-read.

### F-002 [S4] The re-export block is an unused import surface
Where: crates/plugin-api/src/codec.rs:19-21 (with lib.rs headline re-exports + prelude)
Code:
```rust
pub use codec_api::decode::{DecodeOutcome, Decoder, FrameSink, RecoverReason};
pub use codec_api::encode::{EncodeOutcome, Encoder, PacketSink as EncodePacketSink};
pub use codec_api::{DecoderConfig, EncoderConfig};
```
What is wrong: Zero consumers import these through plugin-api (rg). The shipped lane uses codec-api directly. The `EncodePacketSink` rename and the entire Encoder half re-export a trait with no production implementors (only codec-api's own test doubles) — kind 5 is admitted Phase-7 work.
Why it matters: An API surface with no users drifts silently — if codec-api renames or re-shapes `DecodeOutcome`, this crate's re-export breaks or, worse, pins an old name. The lib.rs rationale ("a plugin author needs one import surface") describes a population of zero.
What correct looks like: Keep the re-exports only when the first plugin exists, or add a doc note that the surface is provided-for-future and currently unused in-tree.
Confidence: HIGH — Verification: read + rg.

### F-003 [S4] `codec()`'s contract promises a registry check that nothing performs
Where: crates/plugin-api/src/codec.rs:40-44
Code:
```rust
    /// The codec this plugin decodes (equality-matches the track's
    /// [`CodecDescriptor`](media_types::ids::CodecDescriptor) — the
    /// registry checks it against the manifest's `codecs` capability
    /// claim, doc 06 §7.2).
```
What is wrong: No registry exists to perform the check (F-001). The doc's present tense describes unimplemented enforcement.
Why it matters: Same class as F-001 — contract docs describing a phantom enforcement path; a plugin author would rely on a safety net that is not there.
What correct looks like: "…the registry (Phase 2, doc 06 §3) will check …" — future tense, or omit until wired.
Confidence: HIGH — Verification: read + rg (no check site).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Re-export block | 19-21 | REAL (targets real items) / dead surface | S4 | F-002. Paths verified by CLEAN build (INDEX #1). |
| `CodecPlugin` trait | 39-57 | REAL (definition) / UNREACHABLE | S3 | `Send + Sync` (registry-shared factory); `instantiate` returns `Box<dyn Decoder>`; doc 30-32 honestly cites ADR-001's refusal posture. Zero impls outside tests (F-001). |
| `CodecPlugin::codec` | 40-44 | REAL / UNREACHABLE | S4 | F-003 phantom-enforcement doc. |
| `CodecPlugin::instantiate` | 46-56 | REAL / UNREACHABLE | — | Errors typed at Decode stage; no I/O in contract doc. Sound shape. |
| `codec_claim_mismatch` | 62-74 | REAL / dead | S3 | Zero callers. Stage=Plugin (correct taxonomy vs playlist's Open); ErrorKind::Corrupt/InvalidStructure for what is semantically an integrity violation, not corrupt bytes — taxonomy stretch, noted not finding. |
| `ClaimMismatch` + `Display` + `Error` | 76-91 | REAL | — | Private, correct source-chain type; Display includes both codec names (test-pinned). |
| Test `CopyDecoder` | 103-134 | REAL (toy, honestly labeled) | — | Pass-through packet→audio-frame; labeled "proving the factory → trait wiring". |
| Test `FlacPlugin` | 136-150 | REAL (toy) | — | `unreachable!` at 144 with an inline justification comment — acceptable for a controlled constant. |
| Test `factory_constructs_a_working_decoder` | 153-190 | REAL | — | Bites (§2). |
| Test `CountingSink` + `claim_mismatch_is_a_plugin_stage_error` | 192-215 | REAL | — | Both bite. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `factory_constructs_a_working_decoder` (152-190) | YES | Double assert (outcome + sink count) catches a decoder that lies about emission; configure/instantiate errors are `unreachable!`-surfaced. Only proves toy wiring — no real decoder is exercised through this seam. |
| `claim_mismatch_is_a_plugin_stage_error` (203-215) | YES | Stage equality + source-chain contains both names. |
| Toy impls | — | Honest labels; the subject is a marker trait, so a toy impl is the correct test vehicle — but it also means the file's tests say nothing about any real codec plugin (none exists). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "These traits are NOT redefined here … re-exported" | codec.rs:3-5 | REAL | Verified against codec-api sources; no duplicate trait definitions in this file. |
| "Kind 4 impls live in `codecs` (L3)" | codec.rs:6-8 | REAL | 8+ `impl Decoder for` files in crates/codecs/src (the shipped lane). |
| "kind 5 is Phase 7 (transcode jobs only)" | codec.rs:9 | REAL (honest label) | No production Encoder impl exists (only codec-api test doubles). |
| "the registry instantiates one decoder per track" / "registry checks it against the manifest's codecs claim" | codec.rs:23-24, 42-43 | PAPER-ONLY | No registry code exists (rg; doc 06 §3 = Phase 2 card). F-001/F-003. |
| "a manifest lie — doc 06 §7.2 blocks shipping on it" | codec.rs:60-61 | PAPER-ONLY | §7.2 conformance kit exists only as plugin-host-wasm example tests; nothing invokes `codec_claim_mismatch`. |

## 7. Dead / duplicated / unreachable code
- `CodecPlugin`, `codec_claim_mismatch`: dead in-tree (F-001) — kept alive by lib.rs/prelude re-exports and this file's tests.
- No duplication: the module's entire point is NOT duplicating codec-api (honored — verified no trait redefinitions).

## 8. Dependencies & flags
- codec-api + media-types only (crate's declared L1 set). No features. Compiled unconditionally into every workspace build (INDEX #1) — shipped binaries carry the dead seam in the rlib.

## 9. Open questions for the next agent
- A07/A10: does `kind.rs`'s §1.4 extension-allowed table admit decoder/encoder WASM extensions? If yes, the phantom seam F-001 documents is also the security-relevant boundary a future registry must enforce — worth a cross-ref when the registry card lands.
- Wave-19 (docs): doc 06 §7.2's "blocks shipping" narrative vs the actual conformance surface (example-plugin tests only) — same audit should cover §7 as a whole.
- Whoever audits codec-api: the `EncodePacketSink` rename here is the only in-tree alias of `codec_api::encode::PacketSink` — if codec-api ever renames, this dead alias is where it will surface.
