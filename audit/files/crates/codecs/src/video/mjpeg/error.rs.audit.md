# AUDIT: crates/codecs/src/video/mjpeg/error.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-21 |
| Lines / functions | 213 / 9 (enum + into_engine_error + FeatureDetail + 3 constructors + 2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `pub(crate)` vocabulary of the shipped MJPEG decoder: constructors used 54× in parser.rs and 14× in decoder.rs (rg census); the mjpeg lane is the first vertical (`apps/cli/src/pipeline_host.rs:242-246`; baseline AVI 60/60 frames — mp4-smoke-bench-avi-mjpeg-pcm.txt) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg constructor usage census (parser.rs:54, decoder.rs:14); baseline test log :694-695 (both tests executed ok); read of media-types error types the mapping targets; Display-string cross-check of the mapping table vs the code |

## 1. One-paragraph verdict
A small, clean, total error taxonomy: five variants, each mapped one-to-one onto
the public EngineError contract with stage `Decode`, stable dotted context
codes, and a source-chain carrier that keeps the human-readable
unsupported-feature detail out of the no-secrets Display surface. The doc's
mapping table matches the code variant-for-variant (I cross-checked all five),
the tests pin the exact Display strings and the source-chain behavior and ran
in baseline (:694-695), and the constructors are used heavily by the shipped
decoder path. The only finding is an S4: the mapping table in the module doc
duplicates the code's match arms — a drift risk the taxonomy test does not
cover (it pins Display strings, not the doc table). What was checked and how:
every arm of `into_engine_error` read against `media_types::error`'s kinds
(Unsupported/Corrupt{Truncated,InvalidStructure}/Decoder{Bitstream}/
ResourceLimit); `offset as u64` verified lossless (usize→u64); the
concealment claim (25-30) cross-referenced to decoder.rs's Recovered path
(other agents' chunks); no panics, no allocations beyond the two `to_string`s,
no swallowed errors.

## 2. What actually works (verified)
— Baseline execution: `test-workspace.log.txt:694-695` —
`taxonomy_maps_each_variant_to_its_public_kind` and
`unsupported_detail_rides_in_the_source_chain` both ok: the five Display
goldens and the source-chain round trip are pinned.
— Mapping table (17-23) vs code (75-123): variant-for-variant identical
(read, arm by arm — UnsupportedFeature→Unsupported{FormatInfo mjpeg},
Truncated→Corrupt{Truncated{expected,found}}, BadMarker→Corrupt{InvalidStructure
{Some(offset)}}, CorruptEntropy→Decoder{Bitstream}, LimitExceeded→
ResourceLimit{limit,budget,attempted}).
— `bad_marker`'s `offset as u64` (145): usize→u64 is lossless on every
supported target — not a truncation `as`.

## 3. Findings

### F-001 [S4] The doc's mapping table duplicates the code's match arms with no drift guard
Where: error.rs:17-23 (doc table) vs 75-123 (code)
Code:
```rust
/// | variant | public kind | stage |
/// |---|---|---|
/// | [`MjpegError::UnsupportedFeature`] | [`ErrorKind::Unsupported`] | Decode |
/// | [`MjpegError::Truncated`] | [`ErrorKind::Corrupt`] (`Truncated`) | Decode |
```
What is wrong: the module doc restates the mapping the code implements. The
taxonomy test pins each variant's Display STRING (which encodes the kind
indirectly), so a mapping change that keeps the string would silently leave
the table lying — the table has no test reading it.
Why it matters: pure documentation drift risk on the one table consumers
(errors-surface audits, doc 01 §3.4) will trust.
What correct looks like: drop the table and link to the test as the
executable spec, or have the test assert `engine.kind` discriminants (it
asserts Display strings + stage only).
Confidence: HIGH — Verification: read; the test at 194-198 asserts
`to_string()` and `stage`, not `kind`.
Severity: S4.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MjpegError (enum, 5 variants) | 31-69 | REAL | — | payloads documented; pub(crate) |
| MjpegError::into_engine_error | 73-124 | REAL | — | all five arms verified against media-types kinds; stage Decode constant; no panic path |
| FeatureDetail (+Display/Error) | 130-139 | REAL | — | keeps detail out of EngineError's Display (no-secrets surface), rides the source chain — pinned by test 2 |
| bad_marker | 142-147 | REAL | — | lossless usize→u64 |
| corrupt_entropy | 150-152 | REAL | — | trivial |
| unsupported | 155-157 | REAL | — | trivial |
| test taxonomy_maps_each_variant… | 164-199 | REAL | — | executed (:694); pins Display goldens ×5 + stage; would fail on any mapping/wording change |
| test unsupported_detail_rides_in_the_source_chain | 202-212 | REAL | — | executed (:695); pins kind discriminant (matches!) + source string |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | Yes | exact Display strings for all five variants — any kind/context/stage change alters the string |
| unsupported_detail_rides_in_the_source_chain | Yes | asserts the kind discriminant AND the source-chain payload |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "typed, stage-tagged internal errors that map one-to-one onto EngineError kinds" | error.rs:1-3 | REAL | five arms verified; tests pin the rendering |
| Mapping table 17-23 | error.rs | REAL today, unguarded | F-001 |
| "Nothing here panics; every constructor is total" | error.rs:8 | REAL | read: no unwrap/expect/index in the file |
| CorruptEntropy/post-SOS Truncated "also concealable → Recovered{ConcealedFrame}" | error.rs:25-30 | REAL-UNPROVEN here | cross-ref decoder.rs (other agents' chunks own that logic); the concealment behavior is exercised by mjpeg corpus/fuzz lanes in baseline |

## 7. Dead / duplicated / unreachable code
- None. All variants constructed (54 parser.rs + 14 decoder.rs sites); all
  constructors used.

## 8. Dependencies & flags
- media-types only (crate policy honored); no features; no unsafe; cfg(test)
  for the test module. MJPEG is unconditional in the shipped build.

## 9. Open questions for the next agent
- decoder.rs owners: confirm post-SOS `Truncated` → `Recovered{
  ConcealedFrame }` (the doc's claim) is actually reachable — if the decoder
  only conceals CorruptEntropy, the doc's "and post-SOS Truncated" is stale.
