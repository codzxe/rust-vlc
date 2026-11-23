# AUDIT: crates/codecs/src/audio/opus/error.rs
| Field | Value |
|---|---|
| Auditor | W07-A03 |
| Date | 2026-11-24 |
| Lines / functions | 184 / 5 (into_engine_error, From, Detail::fmt, unsupported, corrupt) + 1 test |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:276-281 `"opus" => self.seeded(track, config, OpusDecoder::new)`; error.rs symbols used 17× in opus/head.rs, 22× in opus/decoder.rs (rg counts) — every typed refusal on the Opus path flows through here |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `codecs-bae0f1756cc0a809 'opus::error'` → 1 passed; rg usage counts across the opus module |

## 1. One-paragraph verdict
REAL and clean. A 60-line typed error taxonomy (Unsupported / Corrupt / BadState) with a one-to-one mapping onto `EngineError` kinds at stage `Decode`, structured `CorruptDetail` carried through, human-readable reasons riding the `source` chain-root (`Detail`, the documented 2.B4 pattern), total constructors, no unwrap, no panic. The doc's variant→kind→stage table matches the code exactly, and the single test asserts the full rendered strings, stage, and source text for all three variants — it would fail on any mapping drift. The one substantive cross-file claim (the ruopus pre-constructor panic gate making the engine's panic contract unreachable) lives in decoder.rs/head.rs, whose structure I spot-checked (channels ∈ {1,2} proven by `parse_head` before the engine is built — decoder.rs:123-133).

## 2. What actually works (verified)
— verified by `codecs-bae0f1756cc0a809 'opus::error'` → `taxonomy_maps_each_variant_to_its_public_kind ... ok` (asserts exact Display output for all three variants, e.g. `"[decode] truncated data: expected 4 bytes, found 1 (opus.packet.decode)"`, plus stage and source chain).
— verified by rg: the constructors/types are consumed throughout the opus module (head.rs 17, decoder.rs 22 references) — not dead scaffolding.

## 3. Findings
None. Zero findings — what was checked and how:
- Fakeness: no todo!/unimplemented!/unreachable!/unwrap/expect anywhere in the file (read in full); every constructor total.
- Correctness: variant→kind mapping read against the doc table (lines 35-39) — exact match; `BadState` hard-codes code `opus.not_configured`, consistent with the contract-misuse semantics; `From` impl delegates to `into_engine_error` (no divergence possible).
- Error-shape honesty: `Corrupt` carries the structured `CorruptDetail` (observable), `Unsupported` names the format (`opus`) — both verified by the test's rendered-string assertions.
- Tests: would fail if the mapping, Display format, stage, or source chaining broke (exact-string asserts).
- Dead code: none — all pub(crate) symbols consumed (rg counts above).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `OpusError::into_engine_error` | 69-101 | REAL | — | 1:1 mapping; stage Decode for all |
| `From<OpusError> for EngineError` | 104-108 | REAL | — | delegates |
| `Detail::fmt` / `impl Error` | 112-121 | REAL | — | chain-root carrier |
| `unsupported` | 124-129 | REAL | — | used by head/decoder gates |
| `corrupt` | 132-142 | REAL | — | used by the packet-decode path |
| test `taxonomy_maps_each_variant_to_its_public_kind` | 149-183 | REAL | — | exact rendered-string pins |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| `taxonomy_maps_each_variant_to_its_public_kind` | YES | exact Display strings + stage + source text for all 3 variants |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| variant→kind→stage table (Unsupported/Corrupt/BadState) | error.rs:35-39 | REAL | code matches; test pins rendered output |
| "Nothing panics; every constructor is total; no `unwrap` on any input path" | error.rs:25-26 | REAL | read in full — no unwrap/panic sites |
| "the ruopus constructors panic on channels∉{1,2}/rate∉{8..48}k … validate from OpusHead BEFORE the engine is ever built" | error.rs:12-16 | REAL (cross-file) | decoder.rs:123-133 documents + applies the gate (`parse_head` proves channels ∈ {1,2} pre-constructor); full verification of the panic-unreachability belongs to the decoder.rs/head.rs audits |

## 7. Dead / duplicated / unreachable code
None. (`Detail` deliberately repeats the 2.B4 `FeatureDetail` pattern — documented as such, pattern reuse not duplication drift.)

## 8. Dependencies & flags
Behind `codec-opus` (default-on). No external deps of its own; ruopus is the engine (decoder.rs). No unsafe.

## 9. Open questions for the next agent
- opus/decoder.rs + head.rs owner: the admission-record claim "6-case hostile class measured zero panics" — confirm the hostile-input suite exists and runs (I did not audit those files).
