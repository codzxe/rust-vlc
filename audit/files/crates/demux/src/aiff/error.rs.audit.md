# AUDIT: crates/demux/src/aiff/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| Lines / functions | 181 / 13 (11 public constructors + mint + 1 test) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — AIFF lane is CLI-wired (`("aiff") => aiff::open`), and 10 of 11 constructors have production minting sites in aiff/mod.rs / chunks.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `cargo test -p demux aiff::error` -> exit 0, 1 passed; grep for minting sites of all 11 constructors; sed quote verification |

## 1. One-paragraph verdict
A tight, consistent typed-error module: 11 constructors, single `mint` funnel at stage Demux, correct ErrorKind mapping (Unsupported for policy, Corrupt+Truncated/InvalidStructure for structure, ResourceLimit+Memory for the packet ceiling), no secrets (offsets/byte counts only), and — unlike asf/error.rs — the in-file taxonomy test covers ALL 11 codes including stage and Display format. Three small defects: `unsupported_compression` ignores its `_offset` parameter (the refusal cannot say where, unlike every sibling); `payload_too_large` is doc-admitted dead vocabulary (zero minting sites); and every `bad_comm` call site passes literal `0`, making that offset parameter vestigial in practice.

## 2. What actually works (verified)
— verified by `cargo test -p demux aiff::error` -> `test result: ok. 1 passed` (my run, exit 0).
— verified by grep: minting sites exist for 10/11 constructors (not_aiff, truncated, bad_structure, bad_comm, unsupported_compression, no_comm, no_ssnd, not_started, seek_out_of_range all minted in aiff/mod.rs / chunks.rs); the single exception is `payload_too_large` (tests only), exactly as its doc admits.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/aiff/error.rs:84-85
Code:
```
pub fn unsupported_compression(_offset: u64) -> EngineError {
    mint(
```
What is wrong: The offset parameter is ignored (`_offset`) — the refusal carries no position, while its siblings (bad_comm, bad_structure, bad_video_info's ASF twin) all embed the offset. Why it matters: lost diagnostic on the one error a user is most likely to hit with a real-world oddball AIFF (8-bit signed, ulaw, ima4); the doc even explains the honest-gap rationale but the offset is still accepted and dropped. What correct looks like: drop the parameter or thread it into InvalidStructure. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/demux/src/aiff/error.rs:137-141
Code:
```
/// A payload ask exceeds the demuxer's packet ceiling. Vocabulary
/// without a minting site today (the packetizer never asks above
/// [`crate::aiff::MAX_PACKET_BYTES`]; the `avi::payload_too_large`
/// precedent).
pub fn payload_too_large(budget: u64, attempted: u64) -> EngineError {
```
What is wrong: Dead constructor — zero production call sites (grep: error.rs + tests only); doc discloses it. Same pattern as asf/error.rs's three fossils. Confidence: HIGH — Verification: read + grep.

F-003 [S4]
Where: crates/demux/src/aiff/mod.rs:462-464 (with mod.rs:463-464, 579, 607)
Code:
```
        let codec = CodecDescriptor::new(codec_of(&comm)).map_err(|_| error::bad_comm(0))?;
        let base = TimeBase::new(1, comm.sample_rate).map_err(|_| error::bad_comm(0))?;
```
What is wrong: Every `bad_comm` minting site passes literal `0` — the offset-bearing error never carries a real offset. Vestigial parameter in practice. Confidence: HIGH — Verification: read + grep (all sites shown).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 20-22 | REAL | — | single funnel |
| not_aiff | 27-37 | REAL | — | sniffed carried; Display-capped |
| truncated | 42-49 | REAL | — | expected/found |
| bad_structure | 55-64 | REAL | — | denormal rate etc. |
| bad_comm | 68-77 | REAL | S4 | offset always 0 at every site (F-003) |
| unsupported_compression | 84-94 | REAL | S4 | `_offset` ignored (F-001); honest-gap doc is accurate |
| no_comm | 98-105 | REAL | — | |
| no_ssnd | 109-116 | REAL | — | |
| not_started | 121-126 | REAL | — | |
| seek_out_of_range | 130-135 | REAL | — | |
| payload_too_large | 141-150 | REAL | S4 | dead vocabulary, disclosed (F-002) |
| tests::errors_carry_demux_stage_and_aiff_codes | 156-180 | REAL | — | complete 11/11 table (better than asf's 12/20) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_aiff_codes | YES | stage, code, `[demux] ` prefix and ` (code)` suffix asserted for all 11 constructors |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Constructors take byte counts and offsets only — never paths, URIs or payload content" | error.rs:11-12 | REAL | all 11 signatures inspected |
| "Vocabulary without a minting site today" (payload_too_large) | error.rs:137-139 | REAL (honest disclosure) | grep: zero production sites |

## 7. Dead / duplicated / unreachable code
`payload_too_large` (F-002). Nothing else.

## 8. Dependencies & flags
media-types error taxonomy only; `#[cfg(test)]` in-file test; no feature gates. Reachable via the CLI aiff arm.

## 9. Open questions for the next agent
- If the AIFF packetizer ever gains a real ceiling refusal, wire the offset properly; otherwise delete payload_too_large with its asf twins in one sweep.

---
CORRECTION (orchestrator gate repair, logged in audit/verify/fabrications.md):
one citation attributed caller code (aiff/mod.rs:462-464) to aiff/error.rs:68.
Where: field corrected; quote verified verbatim against aiff/mod.rs:462-464 via sed.
