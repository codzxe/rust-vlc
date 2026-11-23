# AUDIT: crates/codecs/src/video/mpeg12/bitreader.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 167 / 6 production fns + 6 tests |
| File verdict | REAL (total, bounds-checked MSB-first reader; 6/6 tests pass live) — transitively UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | **NO** — consumed only by mpeg12 headers.rs/facts.rs, which prior agents proved have zero production callers (see audit/files/crates/codecs/src/video/mpeg12/facts.rs.audit.md: Mpeg12Facts used only by tests; demuxers classify MPEG-1/2 with inline probes); no `mpeg1video`/`mpeg2video` decoder arm exists in apps/cli/src/pipeline_host.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. One-paragraph verdict
A correct, total MSB-first bit reader: width-guarded (1..=32), truncation-checked before any indexing (the `self.data[self.bit_pos / 8]` access is provably in-bounds behind the `remaining_bits` gate), exact `remaining_bits` partial-byte arithmetic (hand-checked: pos 5/24 → 19/0), spec marker-bit enforcement, and a peek that discards errors to a documented 0-fill ("peeks only steer dispatch, never assert facts"). All six tests pass live. Its only sin is inherited: the whole mpeg12 lane it serves is shipped-dark (no production caller), so this reader parses bytes for no one outside tests.

## 2. What actually works (verified)
- `timeout 60 ./target/debug/deps/codecs-bae0f1756cc0a809 video::mpeg12::bitreader --test-threads=2` → `6 passed; 0 failed`.
- `remaining_bits` math hand-verified at partial-byte boundaries; panic-freedom verified by reading the guard-before-index structure (plus the empty-input test).

## 3. Findings

F-001 [S3 — transitively unreachable: the mpeg12 lane has no production consumer]
Where: bitreader.rs:21-90 (the whole API)
Code:
```
impl<'a> BitReader<'a> {
    /// Start reading at the slice's first bit.
```
What is wrong: nothing in the reader — the lane is dark. Callers are headers.rs/facts.rs (in-module); `Mpeg12Facts` has zero production callers (prior agents' rg proof, facts.rs.audit.md header + F-entry); the CLI decode factory has no mpeg1/2 arm (the landed-codecs error string lists neither). Compiled into every shipped binary, executed by no shipped path.
Confidence: HIGH — Verification: rg + prior agents' independent audits (facts.rs, headers.rs — same conclusion).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| BitReader::new | 24-26 | REAL | — | |
| remaining_bits | 30-34 | REAL | — | saturating partial-byte math, tested |
| read_bits | 42-62 | REAL | — | width gate + truncation gate before indexing; per-bit loop (fine for header-sized reads; no macroblock layer exists) |
| read_bit | 68-70 | REAL | — | delegate |
| read_marker | 76-81 | REAL | — | typed Marker error, tested |
| peek_bits | 86-89 | REAL | — | clone + 0-fill, contract documented |
| tests ×6 | 97-166 | REAL | — | order, truncation, marker, width, remaining, empty — all pass live |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| reads_msb_first | YES | exact bit patterns |
| truncation_is_typed | YES | exact Truncated payloads |
| marker_bit_enforced | YES | 0-bit rejected, 1-bit accepted |
| width_bounds_rejected | YES | 0 and 33 → Invalid |
| remaining_counts_partial_bytes | YES | 24→19→0 ladder |
| empty_input_only_truncates | YES | Truncated + peek 0 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Total and bounds-checked ... no panic, no unbounded work on any input" | bitreader.rs:6-9 | TRUE | read + empty/width tests |
| "the `fuzz_codec_mpeg2` target's future guarantee" | :8-9 | forward-looking, honest | target doesn't exist yet (fuzz is the h264/av1 pattern) |

## 7. Dead / duplicated / unreachable code
Whole module transitively unreachable (F-001). Sibling-of-h264-reader design choice documented (opposite bit order) — not duplication.

## 8. Dependencies & flags
mpeg12 error vocabulary only; module unconditional in codecs.

## 9. Open questions for the next agent
- None beyond the lane-level wiring question already logged by the facts.rs/headers.rs auditors.
