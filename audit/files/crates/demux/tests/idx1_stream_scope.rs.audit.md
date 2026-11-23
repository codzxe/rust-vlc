# AUDIT: crates/demux/tests/idx1_stream_scope.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 448 / 14 (3 #[test]], 1 builder family, drivers) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — integration test; runs in the default lane (baseline cargo-test-workspace.txt:2282-2287: all 3 tests ok, none ignored). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | sed diff of builder vs strl_advisory.rs (chunk/end_sized byte-identical); baseline citation 2285-2287. |

## 1. One-paragraph verdict
A tight regression suite for the W5-14 idx1 stream-scope fix: all three
contract directions (filtered-stream entries silent, undeclared ordinals
still damage, and the two discriminating in one file) are pinned with exact
damage events, ledger totals, and byte-exact packet rows, including the
damage-record-first event ordering. Every assert is exact; any regression of
the fix fails loudly. Only hygiene debt: the ~80-line AVI builder is a
near-copy of the strl_advisory.rs builder, and `end_sized` carries a
theoretical `as u32` truncation.

## 2. What actually works (verified)
- Baseline cargo-test-workspace.txt:2285-2287: `filtered_and_unknown_idx1_
  entries_discriminate ... ok`, `genuinely_unknown_idx1_stream_is_damage ...
  ok`, `filtered_stream_idx1_entries_are_silent ... ok` — executed in the
  default lane.
- read: the pinned damage conventions match the header.rs audit's
  `filtered_ordinals` scope truth (HeaderList doc, header.rs:412-425) — the
  two files implement one contract.

## 3. Findings
F-001 [S4]
Where: crates/demux/tests/idx1_stream_scope.rs:39-118 vs crates/demux/tests/strl_advisory.rs:36-100
Code:
```
fn chunk(out: &mut Vec<u8>, fourcc: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(fourcc);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() & 1 == 1 {
        out.push(0);
    }
}
```
What is wrong: `chunk`/`end_sized` (and the avih/strh/strf family in adapted
form) are duplicated verbatim between this suite and strl_advisory.rs — two
copies of the RIFF byte rules that can drift apart (the module doc at 33-36
admits "the same byte rules the strl-advisory suite ... use"). tests/common
already exists as the shared home (ogg suite uses it).
Confidence: HIGH — Verification: read + sed comparison (byte-identical
chunk/end_sized).

F-002 [S4]
Where: crates/demux/tests/idx1_stream_scope.rs:48-51
Code:
```
fn end_sized(out: &mut [u8], size_pos: usize) {
    let size = out.len() - size_pos - 4;
    out[size_pos..size_pos + 4].copy_from_slice(&(size as u32).to_le_bytes());
}
```
What is wrong: `size as u32` truncates silently if a built list ever exceeds
4 GiB — impossible for these fixtures, but a builder that should fail loudly
in impossible territory instead of writing a corrupt size.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| chunk / end_sized | 39-51 | REAL | S4 | F-001 dup, F-002 cast |
| avih / strh / strf_video / strf_pcm | 53-118 | REAL | S4 | F-001 dup family; correct field offsets |
| Strl / IdxEntry / build_avi | 121-207 | REAL | — | movi-relative idx1 offsets, patched sizes |
| demuxer_over_bytes | 213-222 | REAL | — | |
| walk | 234-257 | REAL | — | fail-loud on any error/discontinuity |
| fmp4_pcm_movi / pcm_rows | 261-282 | REAL | — | sample-aligned PCM |
| filtered_stream_idx1_entries_are_silent | 294-325 | REAL | — | exact single damage event + ledger 0 |
| genuinely_unknown_idx1_stream_is_damage | 336-400 | REAL | — | damage-first ordering + 16-byte pin |
| filtered_and_unknown_idx1_entries_discriminate | 411-448 | REAL | — | both reasons in order, ledger 16 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all three | YES | exact reason vectors, ledger totals, byte-exact rows, event order |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| pre-fix damage 16 bytes/entry (101×16=1616 on the real file) | idx1_stream_scope.rs:8-11 | REAL-UNPROVEN | worklog-registered defect; pre-fix behavior not re-executed (mutation not run) |
| "damage-record-first: the Recovered row leads" | idx1_stream_scope.rs:374 | REAL | asserted at 375-384, baseline ok |

## 7. Dead / duplicated / unreachable code
F-001 duplication only.

## 8. Dependencies & flags
Integration test using demux::avi public API + test_support::mem_stream. No
feature gates, no external binaries.

## 9. Open questions for the next agent
- Does index.rs scope its unknown-stream damage by `filtered_ordinals`
  length or by ordinal membership? An idx1 entry with ordinal 2 in a file
  whose hdrl carried 2 strl lists (ordinals 0,1) hits the "≥ length" branch
  of the HeaderList doc — this suite covers ordinal 5/7 well past length;
  the boundary ordinal == len is untested.
