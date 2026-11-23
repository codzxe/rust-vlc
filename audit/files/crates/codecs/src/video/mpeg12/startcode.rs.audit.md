# AUDIT: crates/codecs/src/video/mpeg12/startcode.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 217 / 6 (StartCode::{from_byte,to_byte}, next_prefix, StartCodeScan::{new,starts_with_prefix}, Iterator::next) + 5 tests |
| File verdict | REAL (UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO — consumed only by mpeg12::headers/facts; the mpeg12 module has no Decoder and no media-probe/demux/registry consumer (rg: mpeg12 appears only in a comment in apps/web/Cargo.toml). Module doc honestly scopes it as substrate with decode "blocked" |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read + bounds trace; baseline test results cited; rg consumers |

## 1. One-paragraph verdict
A small, correct, exhaustively-tested zero-copy start-code scanner: Table 6-1 classification with a 256-byte round-trip test, bounds-safe prefix search (window arithmetic verified by trace: `i < len-2` with `data[i+2]` access is in-bounds), total on garbage (empty iterator), correct handling of the 4-byte `00 00 00 01` form and shared zeros, and a trailing-prefix-without-code-byte edge pinned by test. No panics, no allocation, no unsafe. The only finding is reachability: nothing shipped consumes the mpeg12 module yet (decode is honestly documented "blocked"; probe does not wire facts::scan).

## 2. What actually works (verified)
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:868-872 -> all 5 startcode tests ok (classifies_every_table_6_1_bucket incl. the 0..=255 round-trip, walks_a_two_unit_stream, four_byte_prefixes_and_shared_zeros, garbage_yields_nothing, prefix_detection).
— verified by manual trace of next_prefix (L82-92) and Iterator::next (L128-149): every index access is guarded (start+1 < len; search window ends at len-2); the 4-byte-prefix cut `idx - 2` cannot underflow (hit > body_from >= 2).

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/video/mpeg12/startcode.rs (module reachability) + mpeg12/mod.rs:8-16
Code: (verbatim, mod.rs L10-13)
```
//! There is **no software MPEG-1/2 decode here** — this module
//! parses and classifies bitstream structure only: start codes,
```
What is wrong (status, not defect): the file is shipped-unreachable — no Decoder impl under mpeg12, no probe/registry consumer. The module doc is honest about this ("Software decode remains `blocked` in the feature matrix"), so this is documented substrate, not a false claim; the code itself is real and tested. Confidence: HIGH — Verification: rg consumers (none outside the module).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| StartCode::from_byte | 38-50 | REAL | - | Table 6-1; exhaustive test |
| StartCode::to_byte | 54-66 | REAL | - | inverse; round-trip pinned 0..=255 |
| next_prefix | 82-92 | REAL | - | bounds-safe; overlapping-prefix safe (i += 1) |
| StartCodeScan::new | 110-115 | REAL | - | |
| StartCodeScan::starts_with_prefix | 120-122 | REAL | - | len>=4 guard |
| Iterator::next | 128-149 | REAL | - | trailing-prefix edge pinned; 4-byte form pinned |
| tests (5) | 156-216 | REAL | - | behavioral, incl. totality |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 5 | YES | direct behavioral pins incl. exhaustive round-trip and edge shapes |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "MPEG-1/2 has no emulation-prevention rule ... a `00 00 01` sequence *is* a start code wherever it appears" | L6-9 | REAL | consistent with ISO 11172-2 §6.2.2; scanner implements flat search |
| module: "no software MPEG-1/2 decode here" | mpeg12/mod.rs:10 | REAL | no Decoder impl in tree; registry refusal mpeg12.decode.blocked |

## 7. Dead / duplicated / unreachable code
The entire module is currently unconsumed outside codecs (F-001) — documented substrate for the blocked decode and Phase-7 hw seam.

## 8. Dependencies & flags
std-only, no unsafe, no allocation in the scan path. No feature gates.

## 9. Open questions for the next agent
- Who will wire facts::scan into media-probe (the doc says "probe/track-info facts")? Until then the whole module is evidence-irrelevant to shipped behavior.
