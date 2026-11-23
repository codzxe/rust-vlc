# AUDIT: crates/codecs/src/video/mjpeg/error.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 213 / 5 production fns + 2 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — MJPEG is the first-vertical decoder: `"mjpeg"` arm at apps/cli/src/pipeline_host.rs:242; engine-info lists `mjpeg` (baseline); this taxonomy is raised by decoder.rs on real decode failures |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. One-paragraph verdict
A clean, total, correctly-mapped error taxonomy for the shipped MJPEG decoder: five variants → five public kinds (Unsupported/Corrupt-Truncated/Corrupt-InvalidStructure-with-offset/Decoder-Bitstream/ResourceLimit), a chain-root carrier that keeps the human detail out of the no-secrets Display, and — unlike the DNx twin — an honest `Truncated { expected }` that documents itself as a lower bound rather than fabricating a sentinel. Both tests pass live. No findings.

## 2. What actually works (verified)
- `timeout 60 ./target/debug/deps/codecs-bae0f1756cc0a809 mjpeg::error --test-threads=2` → `2 passed; 0 failed`.
- Checked and found correct: every variant maps to the kind promised by the doc table (test pins the exact rendered strings incl. stage tag); every constructor is total; `bad_marker`'s `usize → u64` offset cast is lossless on all supported targets; the concealment contract (CorruptEntropy/post-SOS Truncated → `Recovered{ConcealedFrame}`) is declared here and implemented in decoder.rs (cross-checked by rg; decoder.rs is another agent's file).

## 3. Findings
None. (Checked: constructor totality — no unwrap/expect/index; kind mapping vs doc table — exact; Display no-secrets — payload is enum/numbers only; offset cast — lossless; test honesty — string-pinned per variant; dead code — none, all variants constructed in decoder.rs per rg; duplication with dnx/error.rs — same *pattern*, different vocabulary, no copy-paste bug.)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MjpegError::into_engine_error | 73-124 | REAL | — | 5 arms, kinds per table |
| FeatureDetail Display/Error | 130-139 | REAL | — | chain root, no-secrets |
| bad_marker | 142-147 | REAL | — | offset cast lossless |
| corrupt_entropy | 150-152 | REAL | — | |
| unsupported | 155-157 | REAL | — | |
| tests ×2 | 163-212 | REAL | — | exact renderings + source-chain text |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| taxonomy_maps_each_variant_to_its_public_kind | YES | exact per-variant Display strings + stage |
| unsupported_detail_rides_in_the_source_chain | YES | kind match + source text equality |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| doc table (variant → kind → stage) | error.rs:17-23 | TRUE | read + test pins |
| "Nothing here panics; every constructor is total" | error.rs:8 | TRUE | read — no panicking construct |
| "expected: a lower bound; segments declare their own length" | error.rs:38-41 | TRUE (honest doc) | read |
| Concealment semantics live in decoder.rs | error.rs:25-30 | REAL (cross-checked by rg; decoder.rs owned by another agent) | — |

## 7. Dead / duplicated / unreachable code
None. All variants + constructors used by decoder.rs (rg: raise sites present).

## 8. Dependencies & flags
media-types error vocabulary only; mjpeg module unconditional (shipped).

## 9. Open questions for the next agent
- None.
