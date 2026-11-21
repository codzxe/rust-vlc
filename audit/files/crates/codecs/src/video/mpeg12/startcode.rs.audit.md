# AUDIT: crates/codecs/src/video/mpeg12/startcode.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2026-11-21 |
| Lines / functions | 217 / 8 (incl. 5 tests + 1 test helper) |
| File verdict | UNREACHABLE (correct, tested scanner feeding a facts layer nothing shipped consumes) |
| Reachable from shipped entry point? | NO — sole consumer chain: `mpeg12/facts.rs:24,80` (`Mpeg12Facts::scan`) → `tests/mpeg12_corpus.rs:29` only (repo-wide grep). No host dispatches any `mpeg*` descriptor (apps/cli/src/pipeline_host.rs grep `"mpeg"` → none; the codec-unavailable refusal list at :373-378 names no mpeg lane). The mpeg12 module has no Decoder impl at all (grep `impl Decoder` in mpeg12 → none). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Read full file; grep `StartCodeScan\|StartCode::` consumers (facts.rs only); grep `impl Decoder` in mpeg12 (none); baseline log :771-775 (all 5 tests ran, ok); hand-trace of next_prefix bounds and the 4-byte-prefix cut. |

## 1. One-paragraph verdict
A small, correct, exhaustively-tested byte scanner: Table 6-1 classification is pinned by a full 0..=255 round-trip test, the prefix search is bounds-safe (`saturating_sub(2)` + clamped start), the 4-byte `00 00 00 01` form is handled, and all five tests ran for real in baseline (:771-775). It feeds a facts scanner that no shipped entry point reaches — dead weight in the binary, not a defect in the code. Two documentation-precision nits only.

## 2. What actually works (verified)
— `next_prefix` bounds: `end = len.saturating_sub(2)`, candidate `i ≤ len−3`, checks `data[i..=i+2]` — no overread on any input incl. empty and 1-2 byte buffers (hand-traced; `garbage_yields_nothing`'s `[0;64]`, `[]`, and trailing-prefix cases all passed, baseline :773).
— `Iterator::next` prefix-at-end handling: `start + 1 >= len` → exhausted (a `00 00 01` with no code byte yields nothing, tested at :208).
— 4-byte prefix: scanner finds the trailing triple of `00 00 00 01`, code byte read correctly (`four_byte_prefixes_and_shared_zeros`, baseline :772).
— `StartCode::from_byte`/`to_byte`: exhaustive round-trip `for byte in 0..=u8::MAX` (:176-178) — would fail on any mapping typo.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/video/mpeg12/startcode.rs:137-141
Code:
```rust
        // search starts after this unit's code byte: a following
        // `00 00 01` (or the 4-byte `00 00 00 01` form, whose extra
        // zero the `idx - 2` cut correctly leaves to the next unit)
        // cannot begin earlier.
```
What is wrong: the comment's direction is wrong. For a mid-buffer 4-byte prefix `… AA 00 00 00 01 XX`, `next_prefix` finds the trailing triple (01 at index p+3 of the zero-run starting at p), so `body_to = idx − 2 = p+1` — the extra zero at index p stays in the **previous** unit's payload (which ends up one byte longer), not "left to the next unit". Hand-traced: previous payload = `data[body_from..p+1)` includes `data[p]`.
Why it matters: nothing functional (downstream header parsers stop at syntax end; the facts scan counts slice rows, not payload bytes), but the comment asserts a precise byte-attribution that is the opposite of the behavior — the kind of comment the next maintainer trusts.
What correct looks like: "…whose extra leading zero stays in the previous unit's payload (one trailing 0x00); the cut keeps the found triple's two zeros for the next unit."
Confidence: HIGH — Verification: read + hand-trace with concrete indices.
### F-002 [S4]
Where: crates/codecs/src/video/mpeg12/startcode.rs:117-122
Code:
```rust
    /// True when the buffer starts with a start-code prefix (probe
    /// signal: elementary streams begin with `00 00 01 B3`).
    #[must_use]
    pub fn starts_with_prefix(data: &[u8]) -> bool {
        data.len() >= 4 && data[0] == 0 && data[1] == 0 && data[2] == 1
    }
```
What is wrong: the name/doc promise "starts with a start-code prefix" but the implementation also requires a 4th byte (the code byte): `[0,0,1]` returns false although it literally starts with the prefix. The doc's own probe rationale justifies the stricter check — the name and doc just don't say it.
Why it matters: naming that lies (checklist category); a 3-byte-prefix buffer is a degenerate probe input anyway, so no functional consequence (and the consumer chain is unreachable regardless).
What correct looks like: rename to `starts_with_unit_prefix` or document "at least one complete prefix+code-byte unit".
Confidence: HIGH — Verification: read; negative case asserted by `prefix_detection` (:214 `!starts_with_prefix(&[0,0,1])` — the test pins the len≥4 behavior, so the code is intentional; only the doc lags).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `StartCode` (enum) | 12-33 | REAL | — | Table 6-1 buckets incl. honest `Other` for reserved/system bytes |
| `StartCode::from_byte` / `to_byte` | 35-66 | REAL | — | inverse pair; exhaustively round-trip tested |
| `StartUnit` (struct) | 71-78 | REAL | — | zero-copy borrowed slice |
| `next_prefix` | 80-92 | REAL | — | bounds-safe (§2); O(n) flat scan, correct for a format with no emulation prevention (module doc :6-9) |
| `StartCodeScan::new` | 107-115 | REAL | — | first prefix precomputed |
| `StartCodeScan::starts_with_prefix` | 117-122 | REAL | S4 | F-002 name/doc vs len≥4 |
| `Iterator for StartCodeScan` | 125-150 | REAL | S4 | F-001 comment direction; logic itself verified correct incl. end-of-buffer and code-byte-missing cases |
| tests ×5 + `units` helper | 152-216 | REAL | — | all ran in baseline (:771-775); each asserts exact (code, payload-len) tuples |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| classifies_every_table_6_1_bucket | YES | exact bucket asserts + exhaustive 256-value round-trip |
| walks_a_two_unit_stream | YES | exact unit sequence + payload lengths |
| four_byte_prefixes_and_shared_zeros | YES | the one tricky framing case, exact expectations |
| garbage_yields_nothing | YES | 4 hostile shapes (all-zeros, text, empty, trailing prefix) |
| prefix_detection | YES | 3 asserts incl. the len<4 negative |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "MPEG-1/2 has no emulation-prevention rule … scanning is a flat byte search" | :6-9 | REAL | ISO/IEC 11172-2 has no EPB; scanner is exactly that |
| "A buffer with no start codes yields nothing" | :96-98 | REAL | `garbage_yields_nothing` (baseline :773) |
| the 4-byte-prefix attribution comment | :139-141 | FALSE (direction) | F-001 hand-trace |
| "elementary streams begin with `00 00 01 B3`" | :118 | REAL | conventional MPEG-ES layout (sequence header first); not verifiable against corpus in-repo (no mpeg-es fixture found — the corpus tests use hand-built units) |

## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped entry points (header). No duplication: distinct from h264/parser/nal.rs's Annex-B scanner (that one handles EPB and 3/4-byte start codes for a different codec).

## 8. Dependencies & flags
No imports beyond itself — pure std. No unsafe, no feature gates, compiled unconditionally. `#[derive(Debug, Clone)]` on a `Copy`-able iterator struct is harmless (it is Copy-able by fields but Clone suffices for Iterator use).

## 9. Open questions for the next agent
- W06-A05/A06 (facts.rs / headers.rs parts): does any fact depend on payload LENGTH (F-001's one-trailing-zero could skew such a fact)?
- If the mpeg12 facts layer is ever wired into demux probing (e.g. ES sniffing via `starts_with_prefix`), F-002's 4-byte requirement becomes an API contract worth renaming for.
