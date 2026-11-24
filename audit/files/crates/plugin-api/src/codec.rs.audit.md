# AUDIT: crates/plugin-api/src/codec.rs
| Field | Value |
|---|---|
| Auditor | W12-A06 |
| Date | 2026-11-24 |
| Lines / functions | 216 / 3 prod items (trait CodecPlugin, fn codec_claim_mismatch, struct ClaimMismatch) + 2 tests + 2 fixtures |
| File verdict | STUB at the ecosystem level — real, compiling, unit-tested code for a plugin category the v1 host refuses to load and that no crate implements or consumes |
| Reachable from shipped entry point? | NO — `CodecPlugin`/`codec_claim_mismatch` have zero references outside this file (rg over crates/, apps/, tools/) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | `cargo test -p plugin-api --lib codec` -> 2 passed; `rg -n 'CodecPlugin|codec_claim_mismatch' crates/ apps/ tools/` (minus this file) -> 0 hits; `rg` of codecs/codec-api/transcode/playback-engine -> 0 hits; sed verification of quotes |

## 1. One-paragraph verdict
The module compiles, its two tests pass, and the error constructor builds a
correct Plugin-stage EngineError with a source-chained detail. But the entire
surface is scaffolding with no customers: `CodecPlugin` is implemented only by
this file's own test fixture, `codec_claim_mismatch` is called by nobody, and
the v1 host refuses decoder/encoder extension kinds outright
(plugin-api/src/kind.rs:153-165 `extension_allowed_in_v1` excludes them;
plugin-host-wasm/src/error.rs KindRefused: "byte-source, codec and output
kinds stay builtin"). The doc's framing that kind-4 impls "live in codecs
(L3)" is technically about the Decoder trait, not this marker — no crate
implements this trait. This is future-Phase-7 surface shipping as if it were
part of the v1 plugin API.

## 2. What actually works (verified)
— verified by `~/.cargo/bin/cargo test -p plugin-api --lib codec` ->
"test result: ok. 2 passed" (factory test drives a real pass-through decoder
through configure/decode/sink; claim-mismatch test asserts stage == Plugin and
source names both codecs).
— re-exports verified: lib.rs:74 `pub mod codec;`, lib.rs:89 re-exports
Decoder/Encoder from codec_api.

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/codec.rs:39-66
Code:
```
pub trait CodecPlugin: Send + Sync {
```
(whole trait: `codec()` + `instantiate()`)
What is wrong: `rg -n 'CodecPlugin' crates/ apps/ tools/` finds zero hits
outside this file — no production implementation, no consumer, no registry
hook. The trait exists solely so this file's own test fixture can implement
it. Meanwhile kind.rs:153-165 excludes decoder/encoder from v1 WASM
extensions, so no implementor can ever be loaded in v1. Why it matters: this
is dead API surface presented in the public plugin-api crate; a plugin author
reading it would believe codec plugins are a supported category. What correct
looks like: ship it behind the phase that consumes it, or document in the
module head that v1 refuses the kind (error.rs KindRefused) and the trait is
Phase-7 scaffolding.
Confidence: HIGH — Verification: ran (rg absence proof over the whole repo)
What correct looks like: gate or annotate as Phase-7.

F-002 [S3]
Where: crates/plugin-api/src/codec.rs:60-84
Code:
```
/// Typed registry-side error: the plugin's claimed codec does not
/// match what it serves (a manifest lie — doc 06 §7.2 blocks shipping
/// on it).
pub fn codec_claim_mismatch(claimed: &str, served: &str) -> media_types::EngineError {
```
What is wrong: The doc asserts a registry rule ("blocks shipping on it") but
the function enforcing it has zero callers — no registry code compares a
manifest's `codecs` claim against `CodecPlugin::codec()`. The enforcement the
doc describes does not exist anywhere; only the error constructor for it does.
Confidence: HIGH — Verification: ran (rg absence proof)
What correct looks like: the (future) registry calling this on first
instantiate, or the doc saying "reserved for the Phase-7 registry".

F-003 [S4]
Where: crates/plugin-api/src/codec.rs:6-7, 17
Code:
```
//! plugin author has one import surface. Kind 4 impls live in
```
and
```
//! - typed stage labels for registry-side codec errors.
```
What is wrong: "typed stage labels" (plural) — exactly one constructor exists
(codec_claim_mismatch). "Kind 4 impls live in codecs (L3)" — no CodecPlugin
impl lives there (0 hits in crates/codecs). Loose doc framing that reads as
current wiring.
Confidence: HIGH — Verification: read + rg

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| (re-exports) | 21-24 | REAL | — | codec_api Decoder/Encoder/config types |
| trait CodecPlugin | 39-66 | STUB (no impls/consumers) | S3 | F-001; v1 refuses the kind |
| codec_claim_mismatch | 60-84 | STUB (no callers) | S3 | F-002; error shape itself correct |
| ClaimMismatch + Display + Error | 86-102 | REAL | — | correct source-chain detail |
| tests: CopyDecoder/FlacPlugin/CountingSink | 105-175, 189-201 | REAL | — | honest fixtures |
| factory_constructs_a_working_decoder | 177-187 | REAL (unit) | — | real decode round-trip |
| claim_mismatch_is_a_plugin_stage_error | 203-216 | REAL (unit) | — | asserts stage + source text |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| factory_constructs_a_working_decoder | YES | real configure→decode→sink round trip; outcome and sink count must agree |
| claim_mismatch_is_a_plugin_stage_error | YES | asserts PipelineStage::Plugin and both names in the source |
Both executed by me: 2 passed, 0 failed. Caveat: they prove the unit, not any
production wiring (there is none).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Kind 4 impls live in `codecs` (L3)" | codec.rs:6-7 | MISLEADING | codecs implements codec_api::Decoder, not this CodecPlugin (rg: 0 hits) |
| "typed stage labels for registry-side codec errors" | codec.rs:17 | INFLATED | one constructor, zero callers |
| "doc 06 §7.2 blocks shipping on it" (claim mismatch) | codec.rs:60-62 | PAPER-ONLY | no registry code performs the check |
| "kind 5 is Phase 7 (transcode jobs only)" | codec.rs:8-9 | REAL (as policy) | kind.rs excludes encoder from v1 extensions |

## 7. Dead / duplicated / unreachable code
The whole module is dead surface in v1: no implementors, no consumers, host
refuses the kind. The re-exports (lines 21-24) are the only live part (they
serve plugin-api's re-export surface for authors).

## 8. Dependencies & flags
Depends on codec-api (re-exports), media-types (EngineError/Packet/Frame).
No feature gates. plugin-api is depended on by plugin-host-wasm and codecs —
neither uses this module's own items.

## 9. Open questions for the next agent
- transcode/Phase-7 auditor: is CodecPlugin the intended Phase-7 seam? If a
  registry design doc references it, F-001 downgrades to "early scaffolding"
  — but the v1 refusal of the kind stays a fact.
- plugin-api lib.rs auditor: the lib.rs:89 re-export of Decoder/Encoder — who
  consumes plugin_api::Decoder (vs codec_api::Decoder directly)? If nobody,
  the re-export is also dead.
