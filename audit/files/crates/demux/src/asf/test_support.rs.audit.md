# AUDIT: crates/demux/src/asf/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Lines / functions | 283 / 10 prod helpers (pub: 4) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (test-support surface, documented as such :5-6) — consumed by asf/tests.rs:41-42, asf/mod.rs doctests, asf/fuzz.rs; the demuxer itself never calls it |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg usage map (asf/tests.rs 20+ call sites, mod.rs doctests, fuzz.rs); sed quote verification; baseline cite for the ASF suite |

## 1. One-paragraph verdict
Real byte builders, not stubs: correct ASF object framing (Header 30-byte prefix, child
objects GUID+QWORD size, Data Object with patched size/count, measured Simple Index
layout), correct 80-byte File Properties payload, and size fields patched to real totals
(the walk must accept consistent truth). Used heavily by the ASF suite (baseline
cargo-test-workspace.txt:1767-1772 shows asf tests green). One piece of vestigial
dead code with a tautological assert.

## 2. What actually works (verified)
— Layout math checked by hand: header prefix 16+8+4+2 = 30; child frame 16+8 = 24;
  FPO payload 16 + 6×8 + 4×4 = 80; Stream Properties 16+16+8+4+4+2+2+2 = 54 + tsd —
  all match the pushed bytes.
— Consumers verified by rg: asf/tests.rs:41-42 imports all four pub builders;
  mod.rs doctests and fuzz.rs use mem_stream. Not dead.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/asf/test_support.rs:150-157
Code:
```
    // Patch the FPO's File Size to the real total.
    let header_size = 30 + children.len();
    let fpo_payload_at = 30 + 24;
    let total = end as u64;
    out[fpo_payload_at + 16..fpo_payload_at + 24].copy_from_slice(&total.to_le_bytes());
    debug_assert_eq!(header_size, 30 + children.len());
    let _ = header_size;
    Arc::from(out)
```
What is wrong: `header_size` is assigned `30 + children.len()` and then
`debug_assert_eq!`d against the literal same expression — a tautology that can never
fire — followed by `let _ = header_size;` to silence the unused-variable lint. Dead
code pretending to be a consistency check. (Contrast: the size fields that MATTER —
Data Object size at :147-149, FPO File Size at :153-154 — are genuinely patched.)
Confidence: HIGH — Verification: read.
What correct looks like: delete the three vestigial lines, or assert something
falsifiable (e.g. FPO child size == 24 + 80).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mem_stream | 19-21 | REAL | — | delegates to avi's BytesStream (one seam, not two) |
| push_guid/push_qword/push_dword/push_word | 24-41 | REAL | — | LE append helpers |
| file_properties_payload | 45-59 | REAL | — | 80-byte layout verified |
| audio_stream_properties_payload | 64-88 | REAL | — | 54+tsd; stream number as Flags WORD low bits (ffmpeg-measured split documented) |
| child | 91-97 | REAL | — | GUID + size(24+len) + payload |
| minimal_asf_audio | 105-158 | REAL | S4 | F-001 vestigial assert; Data/FPO sizes patched correctly |
| waveformatex | 163-173 | REAL | — | 18-byte WAVEFORMATEX, avg-bps = align×rate |
| SimpleIndexSpec / IndexEntry | 179-192 | REAL | — | ffmpeg-measured 6-byte entries |
| asf_with_packets | 201-283 | REAL | — | end-to-end builder; Data/Simple-Index/FPO sizes all patched to real totals |

## 5. Test honesty
N/A (builder module — no tests of its own; the ASF suite that consumes it is green in
the orchestrator baseline, cargo-test-workspace.txt:1767-1772).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "nothing in the demuxer itself uses it" | :5-6 | REAL | rg: only tests.rs/doctests/fuzz.rs consume it |
| "the ffmpeg-measured 6-byte entry form" (Simple Index) | :176-178 | REAL-UNPROVEN | layout matches the ASF spec form; ffmpeg measurement not re-run here |

## 7. Dead / duplicated / unreachable code
F-001's three lines (150, 155-156). No duplication — mem_stream reuses avi's.

## 8. Dependencies & flags
plugin-api, media-types, std Arc. No feature gates; not compiled into release
(test-support module wired under cfg(test)/doc only per mod.rs).

## 9. Open questions for the next agent
- asf/tests.rs (not mine): confirm the FPO File-Size patch offset (30+24+16) is also
  what the walk reads — a builder/walk offset disagreement would make every test
  fixture carry a subtly lying File Size that the walk tolerates.
