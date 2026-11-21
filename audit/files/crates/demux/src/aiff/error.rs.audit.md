# AUDIT: crates/demux/src/aiff/error.rs (full file, 181 lines)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-11-21 |
| Lines / functions | 181 / 11 constructors + mint + 1 test |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (constructors minted by `aiff/mod.rs` walk ← `AiffDemuxer` ← `apps/cli/src/containers/aiff.rs` ← `lumen` open path; error test executed ok in baseline `orchrun-test-workspace-nofailfast.log.txt:1530`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n '"demux\.(aiff\|wav\|avi)\.[a-z.-]+"'` across the three error modules (code-string convention check — `seek.out-of-range` dot-form is the FAMILY convention, avi error.rs:191 and wav error.rs:143 identical); `rg -n "payload_too_large\(" crates/demux/src/aiff/` (zero production callers); `sed -n` on quoted ranges; baseline grep |

## 1. One-paragraph verdict
A small, disciplined typed-error module: eleven constructors over the
media-types taxonomy (never redefining kinds), all at stage Demux with
stable `demux.aiff.*` codes, byte-counts-and-offsets-only payloads ("no
secrets" policy stated and held), and a test that pins every code, the
stage tag, and the Display shape. The code-string convention matches the
family (wav/avi use the same `seek.out-of-range` dot form — verified, so
no inconsistency finding). Two hygiene defects: `payload_too_large` is
dead in production (admitted in its own doc, but it is vocabulary nothing
mints), and `unsupported_compression` takes an offset parameter it
silently discards, implying a localization its `Unsupported` kind cannot
carry.

## 2. What actually works (verified)
- `aiff::error::tests::errors_carry_demux_stage_and_aiff_codes ... ok` in
  baseline (:1530) — all 11 constructors pinned for stage, code, Display
  prefix `[demux] ` and suffix `(code)`.
- Honesty in docs verified by read: classic AIFF 8-bit *signed* named as a
  real vocabulary gap (`unsupported_compression` doc :80-83) rather than a
  fabricated mapping.
- Error-kind mapping is semantically right per constructor (Unsupported
  for not-aiff/unsupported-compression/not-started/seek-range; Corrupt
  Truncated for cuts; Corrupt InvalidStructure for structure/COMM; the
  sole ResourceLimit for the ceiling).

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/aiff/error.rs:137-150
Code:
```rust
/// A payload ask exceeds the demuxer's packet ceiling. Vocabulary
/// without a minting site today (the packetizer never asks above
/// [`crate::aiff::MAX_PACKET_BYTES`]; the `avi::payload_too_large`
/// precedent).
pub fn payload_too_large(budget: u64, attempted: u64) -> EngineError {
```
What is wrong: Dead in production — `rg "payload_too_large\("` over
`crates/demux/src/aiff/` finds only the definition and the test call; no
AIFF code path mints it. The doc admits this ("without a minting site
today").
Why it matters: Dead vocabulary in a stable-code surface: the code string
`demux.aiff.payload-too-large` is now part of the observable contract (the
test pins it), so removing it later is a breaking change for nothing
gained, and a reader of the codes table believes a refusal path exists
that never fires.
What correct looks like: Either mint it at the one place the packetizer
computes its ask (defensive assert-style refusal), or remove it until the
site exists.
Confidence: HIGH — Verification: rg (zero callers) + read.

### F-002 [S4]
Where: crates/demux/src/aiff/error.rs:84
Code:
```rust
pub fn unsupported_compression(_offset: u64) -> EngineError {
```
What is wrong: The parameter is discarded (`_offset`). Every sibling
Corrupt constructor localizes (`bad_structure(offset)`,
`bad_comm(offset)`); this signature suggests the error carries the offset
but the `Unsupported` kind has nowhere to put it — the caller computes and
passes a value that dies here.
Why it matters: Misleading API symmetry: a future caller (or auditor)
assumes the compression-refusal site is observable in the error, and the
underscore is invisible at call sites.
What correct looks like: Drop the parameter (call sites pass nothing), or
switch to a kind/detail that carries the offset if localization matters.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` (private) | 20-22 | REAL | — | Single construction point, STAGE const. |
| `not_aiff` | 27-37 | REAL | — | Unsupported + FormatInfo{aiff, sniffed}; sniffed payload is caller-bounded (probe prefixes); "no secrets" holds. |
| `truncated` | 42-49 | REAL | — | Corrupt::Truncated{expected, found}. |
| `bad_structure` | 55-64 | REAL | — | InvalidStructure{offset: Some}. |
| `bad_comm` | 68-77 | REAL | — | Same kind, distinct code from bad_structure (observability split — good). |
| `unsupported_compression` | 84-94 | REAL | S4 (F-002) | `_offset` discarded. |
| `no_comm` / `no_ssnd` | 98-116 | REAL | — | offset: None — honest (no site exists). |
| `not_started` | 121-126 | REAL | — | Unsupported{format: None}. |
| `seek_out_of_range` | 130-135 | REAL | — | Dot-form code matches family convention (wav error.rs:143, avi error.rs:191 — verified). |
| `payload_too_large` | 141-150 | REAL | S4 (F-001) | ResourceLimit::Memory; dead in production. |
| `tests::errors_carry_demux_stage_and_aiff_codes` | 156-181 | REAL | — | Pins stage + code + Display for all 11; would fail on any code-string or Display-shape change. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_aiff_codes | YES (for what it covers) | Exact code strings, stage tag, Display prefix/suffix all asserted per constructor. It cannot catch a *missing* refusal path (that is the demuxer tests' job — e.g. `missing_comm_and_ssnd_are_typed` :1542, `non_normalizable_rate_is_structural` :1543, both ok in baseline). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The error taxonomy is media-types' seven kinds — never redefined here" | error.rs:3-5 | REAL | All constructors use media_types::{ErrorKind, CorruptDetail, LimitKind}; no local kind. |
| "No secrets: Constructors take byte counts and offsets only" | error.rs:9-12 | REAL | Verified per signature; `not_aiff`'s sniffed is caller-provided prefix bytes, Display-capped per its doc. |
| Classic AIFF 8-bit signed has no descriptor ("the honest gap") | error.rs:80-83 | REAL | The vocabulary refusal is `unsupported_compression`; the AIFF fixtures/tests confirm 8-bit lanes are refused (aiff tests, A-auditor's range). |

## 7. Dead / duplicated / unreachable code
- `payload_too_large` — dead in production (F-001), test-pinned.
- Structure mirrors `wav/error.rs` constructor-for-constructor (not_aiff/
  truncated/bad_structure/not_started/seek_out_of_range/payload_too_large)
  — family-template duplication, deliberate (per-demuxer code namespaces),
  not flagged.

## 8. Dependencies & flags
media_types only. No unsafe, no feature gates, no I/O. `#[cfg(test)]` mod
is self-contained.

## 9. Open questions for the next agent
- aiff/mod.rs owner: confirm the packetizer truly never exceeds
  MAX_PACKET_BYTES (F-001's premise) — if a mutated SSND can drive an ask
  above the ceiling without a typed refusal, the dead constructor becomes
  an S2 (unbounded allocation) instead of S4.
