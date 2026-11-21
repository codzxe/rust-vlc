# AUDIT: crates/codec-api/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A03 |
| Date | 2026-11-21 |
| Lines / functions | 28 / 8 config keys (package×6, lints, dependencies×1; dev-dependencies deliberately empty) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. 11 dependents incl. apps/cli/Cargo.toml (`codec-api = { path = … }`, direct) and via crates/codecs + crates/playback-engine. Runtime: baseline engine-info decoders (mjpeg, pcm_s16le) are consumed through this crate's `Decoder` trait (decode.rs:86); AVI smoke `a_decoded=60`/`v_decoded=60` exercises the contract end-to-end |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | Grep `codec-api = { path` in **/Cargo.toml (11 dependents) · sed -n boundaries.toml [crates.codec-api] (exact match incl. empty dev_deps) · `head -45 src/hw.rs` (no platform types — abstract trait) · grep `pub trait` in decode.rs/encode.rs (Decoder:86, FrameSink:186, Encoder:34, PacketSink:99) · grep `cfg(test\|cfg(feature` in contract.rs (single gate at line 733) · grep `panic!\|assert\|unwrap()\|expect(` in contract.rs (asserts only inside the line-733 cfg(test) unit tests; public API returns Results) · grep callers of `codec_api::contract` (crates/codecs/tests/contract.rs) |

## 1. One-paragraph verdict
A 28-line manifest that makes five claims and keeps all five: single
dependency (media-types, used), zero dev-dependencies (by documented design —
the contract suite lives in the library), no codec implementations (verified:
traits/types only), no platform types (hw.rs:27-32 `HwAccelerator` names no
API/device/surface), no selection policy. Every config key matches
boundaries.toml exactly, including the empty dev_deps list. The one design
decision a hostile reviewer would poke — 868 lines of test-suite code
compiled unconditionally into the library — is documented in the manifest,
consumed by a real caller (crates/codecs/tests/contract.rs), and panic-free
outside cfg(test). Zero findings; every claim survived verification.

## 2. What actually works (verified)
— "media-types is the ONLY dependency" (line 8): TRUE — [dependencies] has
exactly one path entry (sed -n '22,23p'); lib.rs/decode.rs/encode.rs consume
it (EngineError, Packet, Frame vocabulary).
— "Std-only … no serde/thiserror" (line 9-10): TRUE — no external crates.
— "NO codec implementations" (line 4): TRUE — src is 2,170 lines of
traits/config/capability/contract modules; `pub trait Decoder` (decode.rs:86),
`pub trait Encoder` (encode.rs:34); the concrete decoders live in crates/codecs.
— "NO platform types (hardware stays abstract per ADR-001)" (line 5): TRUE —
hw.rs:27-32 `pub trait HwAccelerator: Send + Sync` with `backend_id(&str)` +
`as_any()` only; doc comment: "the trait names no API, no device, no surface
format".
— "No dev-dependencies: the contract suite lives in the library itself"
(lines 25-27): TRUE — src/contract.rs is a public module (not cfg-gated; its
own unit tests gate at line 733), called by
crates/codecs/tests/contract.rs via `codec_api::contract`.
— Crate builds and its dependents link: baseline #1 exit 0; contract
exercised at runtime by the first-vertical smoke (baseline #20 PASS, 6/6).

## 3. Findings
None. Checklist applied with the following results (what was checked and how):
- Fakeness: no stubs possible at manifest level; the only "seam" (contract
  suite in-library) verified real + consumed (grep caller).
- Dep hygiene: 1 prod dep used, 0 unused, 0 duplicated, 0 external
  (matches boundaries.toml [crates.codec-api]: internal_deps=["media-types"],
  dev_deps=[] — empty list matches the absent [dev-dependencies]).
- Features: none declared → nothing for release.yml to never enable.
- Doc claims: description's four nouns (Decoder/Encoder traits, capability
  descriptors, negotiated configs, contract test-suite) map 1:1 to
  capability.rs / config.rs / contract.rs / decode.rs / encode.rs (grep pub
  trait / module list).
- Panic shipping: contract.rs asserts exist ONLY inside its cfg(test) block
  (line 733+); public contract API is Result-typed (contract.rs:83
  `detail(&self)`, `decode_outcome_agrees_with_sink(...) -> Result`).
- Reachability: 11 dependent manifests incl. the shipped CLI.

## 4. Per-key table
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `package.name = "codec-api"` | 13 | REAL | — | Matches boundaries.toml |
| `package.version = "0.1.0"` | 14 | REAL | — | Repo convention |
| `package.edition = "2024"` | 15 | REAL | — | Workspace-consistent |
| `package.description` | 16 | REAL | — | All four claimed surfaces exist (verified module-by-module) |
| `package.publish = false` | 17 | REAL | — | — |
| `[lints] workspace = true` | 19-20 | REAL | — | Inherits deny tables |
| `media-types` dep | 23 | REAL | — | Used throughout (Packet/Frame/EngineError vocabulary) |
| absent `[dev-dependencies]` (+comment) | 25-27 | REAL | — | Deliberate; boundaries dev_deps=[] agrees; contract suite consumed by codecs' tests without a graph edge |

## 5. Test honesty
No manifest tests. The in-library contract suite (contract.rs) is consumed by
crates/codecs/tests/contract.rs — would it fail if a Decoder broke its
contract? The suite checks decode-outcome/sink-count agreement and flush
drain semantics (contract.rs:742-763 test style shows the checks); mutation
reasoning: a decoder reporting `Emitted { frames: 2 }` while the sink accepts
1 fails `decode_outcome_agrees_with_sink` — non-tautological. Final proof is
the codecs-crate auditor's scope.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "media-types is the ONLY dependency" | Cargo.toml:8 | REAL | Single path dep (sed -n 22,23p) |
| "NO codec implementations" | Cargo.toml:4 | REAL | Traits only; concrete codecs in crates/codecs |
| "NO platform types (hardware stays abstract per ADR-001)" | Cargo.toml:5 | REAL | hw.rs:27-32 abstract trait, no platform imports |
| "NO selection policy" | Cargo.toml:6 | REAL | No registry/selection code in src (module list) |
| "the contract suite lives in the library itself … without a new edge" | Cargo.toml:25-27 | REAL | contract.rs public; consumed by codecs/tests/contract.rs (grep) |

## 7. Dead / duplicated / unreachable code
None in the manifest. Note for context: the in-library contract suite
(868 lines) has exactly one in-repo consumer today
(crates/codecs/tests/contract.rs); the manifest's stated other consumer (the
MP §6/§8 conformance kit) is future work. This is a documented, working
design, not dead code — the alternative (media-probe's feature-gated contract
module) exists in-repo and shows the team knows the tradeoff.

## 8. Dependencies & flags
1 internal prod dep; 0 dev deps; 0 external; 0 features. boundaries.toml
agreement exact, including the empty dev_deps list. Layer L1 → L0 edge
only, as the registry row demands.

## 9. Open questions for the next agent
- The src auditor should verify the contract suite's checks are actually
  invoked by codecs' tests (not just imported) — my grep proves the import,
  not the call depth.
- If the conformance kit (MP §6/§8) lands outside codecs/, re-examine whether
  the in-library placement still holds.
