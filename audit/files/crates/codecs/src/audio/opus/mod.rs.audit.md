# AUDIT: crates/codecs/src/audio/opus/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-21 |
| Lines / functions | 37 / 4 module declarations + 1 re-export (module-map file) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open *.opus` → `containers/ogg.rs` OpusHead-chain harvest → `apps/cli/src/pipeline_host.rs:276` `"opus"` → seeded `OpusDecoder` (this module's re-export); module gated `#[cfg(feature = "codec-opus")]` at `crates/codecs/src/audio/mod.rs:57-58`, default ON. Executed in baseline: `containers/ogg.rs:496` `opus_fixture_opens_and_decodes` (21 packets) and the corpus 0047 lane (192,000 valid samples = granule 192,312 − pre-skip 312) passed in run #3.) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `ls crates/codecs/src/audio/opus/` (4 submodules exist); `head -60 opus/head.rs` + `sed -n 96-165 opus/decoder.rs` (claim spot-checks); `rg -n 'opus' audio/mod.rs` (feature gate :57-58); `rg -n 'order\|pre_skip\|granule' decoder.rs` (output-seam + trim claims); Cargo.toml vs mod.rs:24-25 pin comparison; baseline log citations. |

## 1. One-paragraph verdict
A 37-line module map whose every doc claim survives cross-examination: the four submodules exist and do what the map says (head.rs is a genuinely strict OpusHead validator that closes ruopus's documented constructor panic windows before any engine exists; order.rs's Vorbis→house permutation is applied at decoder.rs's output seam; decoder.rs implements pre-skip/end-trim granule arithmetic; error.rs is the taxonomy), the ruopus pin claim matches Cargo.toml verbatim, and the whole module is correctly feature-gated and shipped-reachable with passing end-to-end baseline evidence. Zero findings; checks itemized in §2.

## 2. What actually works (verified) — exhaustive check list for the zero-finding claim
— verified the module map against the submodules (existence + responsibility): `head.rs` (11,405 bytes — RFC 7845 §4.2 grammar: version-1 gate, family-0 channels ∈ {1,2}, family ≥1 multistream closure `streams ≥ 1 ∧ coupled ≤ streams ∧ streams+coupled = channels ∧ mapping entries in-slot`, `DECODE_RATE = 48_000` constant — exactly the "pre-constructor validation" mod.rs:8-12 promises); `order.rs` (`vorbis_to_house`, consumed at `decoder.rs:81` import and `:292` — the claimed output seam); `decoder.rs` (pre-skip head-trim + granulepos end-trim per its :2-3, :31-35 doc); `error.rs` (present; audited by W07-A01).
— verified the panic-closure claim structurally (mod.rs:10-12): `build_engine` (decoder.rs:121-144) constructs `RuopusMono::with_rate` only with parse-proven channels ∈ {1,2} and the 48 kHz constant, and `RuopusMultistream::with_rate` only with arithmetic-proven streams/coupled/mapping — the ruopus panic sites are unreachable behind `parse_head`.
— verified the engine-pin claim (mod.rs:22-29) verbatim against `crates/codecs/Cargo.toml`: `ruopus = { version = "=0.1.2", optional = true, default-features = false, features = ["std"] }` — exact pin, features off, matching the admission-record conditions quoted.
— verified feature gating: `audio/mod.rs:57-58` `#[cfg(feature = "codec-opus")] pub mod opus;` — mod.rs's own unconditional `pub mod decoder` lines are covered by the parent gate; `codec-opus` is in the default set (Cargo.toml default list), so the shipped build compiles it, and `--no-default-features` subsets (the wasm tree-shake) drop it cleanly.
— verified no public-surface leakage: only `OpusDecoder` is re-exported (mod.rs:36); `head`/`order`/`error` are private modules, `OpusHead` is `pub(crate)` — the wrapper's internal vocabulary stays internal.
— verified baseline execution: run #3 includes the ogg opus end-to-end decode (fixture 21 packets; corpus 0047: 192,000 samples — the granule-minus-preskip pin) — the module wiring this file declares ran green.

## 3. Findings
None. (Checked for: module-map claims contradicted by submodule reality, missing feature gate (would break subset builds), pin drift between doc and Cargo.toml, dead re-exports, public leakage of internal types, panic-window claims not backed by validation order — all negative.)

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc (the map) | 1-29 | REAL | — | Every claim cross-verified (§2). |
| `pub mod decoder` | 31 | REAL | — | Exists, implements the Decoder contract (W07-A01/A07 audit the body). |
| `mod error` | 32 | REAL | — | Exists (W07-A01). |
| `mod head` | 33 | REAL | — | Exists, strict validator (W07-A08). |
| `mod order` | 34 | REAL | — | Exists, applied at the output seam (W07-A05). |
| `pub use decoder::OpusDecoder` | 36 | REAL | — | Sole public export; consumed by `pipeline_host.rs:276` dispatch. |

## 5. Test honesty
No tests in this file (module map). The submodules' tests and the ogg-lane e2e tests (containers/ogg.rs:496, corpus 0047/0077) are the executing evidence — ran green in baseline run #3.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "pre-constructor validation that closes the ruopus constructors' documented panic windows (channels/rate) before any engine exists" | mod.rs:8-12 | REAL | head.rs grammar + decoder.rs:121-144 construction order (§2). |
| "RFC 7845 → house channel-order mapping … applied at the output seam" | mod.rs:13-15 | REAL | order.rs `vorbis_to_house` at decoder.rs:81/:292 (family-1 permutation). |
| "pre-skip head-trim, granulepos end-trim, packet-scoped damage isolation" | mod.rs:16-18 | REAL-UNPROVEN | decoder.rs docs declare it and the corpus 0047 pin (192,000 = 192,312 − 312) passed in baseline; the body is W07-A01/A07's to fully audit. |
| "`ruopus = { version = "=0.1.2", default-features = false, features = ["std"] }` — the zero-dependency shape" | mod.rs:22-25 | REAL | Cargo.toml verbatim match (read during this audit). |
| "the decode rate is ALWAYS 48 kHz" (RFC 7845 §4.2, head.rs) | head.rs:9-24 | REAL | Constant `DECODE_RATE = 48_000`; granule clock and decode both count 48 kHz samples — no rate conversion is ever needed on this lane (relevant to the wave-7 resampling question: audio-engine has NO rate converter and refuses mismatches typed; Opus lanes are 48 kHz end to end by construction). |

## 7. Dead / duplicated / unreachable code
None. (The `opus` note in `apps/cli/src/containers/es.rs` and demux_host's byte-9 channel sniff are Ogg-lane facts, not duplication of this module.)

## 8. Dependencies & flags
- `codec-opus` (default ON) → `ruopus =0.1.2` (optional, pinned, default-features off, std only). No unsafe in this file. The only re-export is `OpusDecoder`.

## 9. Open questions for the next agent
- decoder.rs auditors (A01/A07): the corpus 0077 (5.1 opus) lane asserts geometry + packet count but has NO decode pass (0047 does) — is multistream decode exercised by ANY test? If not, `Engine::Multi` is e2e-untested.
- head.rs auditor (A08): family-255 (discrete) heads with arbitrary channel counts — confirm the multistream closure math rejects impossible geometry there too.
