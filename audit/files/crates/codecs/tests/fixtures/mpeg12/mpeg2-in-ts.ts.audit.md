# AUDIT: crates/codecs/tests/fixtures/mpeg12/mpeg2-in-ts.ts
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 54,520 bytes (binary, exactly 290×188) / n/a |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (test fixture; consumed by mpeg12_corpus.rs fixture lane — ran and passed in baseline — and media-probe registry_roundtrip) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> MPEG transport stream data; python3 TS parse (packets/PAT/PMT/PES, below); `rg mpeg2-in-ts` -> 2 test files; baseline citation |

## 1. One-paragraph verdict
Genuine, structurally clean MPEG-2 transport stream. Direct parse: exactly 290 packets of 188 bytes (zero trailing bytes), all 290 sync-aligned on 0x47. PAT (PID 0, 5 repeats) declares one program (1) with PMT on PID 4096 (5 repeats); PMT (table_id 0x02) carries one elementary stream of type 0x02 (ITU-T H.262 / MPEG-2 video) on PID 256 with PCR on 256; PID 256 holds 278 packets of PES with stream_id 0xE0 (MPEG video), PTS present, PES length 0 (TS convention); PID 17 carries an SDT (table_id 0x42). The filename `mpeg2-in-ts` describes exactly what the bytes are. The consumer test demuxes it through the real TsDemuxer and asserts header facts against pinned ffprobe truth — decode is explicitly not attempted (honest boundary, mpeg12 decode is typed-blocked and not in the shipped decoder set).

## 2. What actually works (verified)
— python3: `size 54520 %188 = 0`, `sync-aligned packets: 290 of 290`; PIDs {0:5, 4096:5, 256:278, 17:2}.
— python3 PAT: program 1 → PMT PID 4096 (`f0 00` → 0x1000); PMT: stream_type 0x02, ES PID 256 (`e1 00`), PCR PID 256, program_info_length 0; first PES on 256: `00 00 01 e0` stream_id 0xE0, header flags 0x80 0xC0, PTS present (adaptation field 0x07 0x50 with PCR).
— Baseline: `test fixture_transport_lanes_ts_and_ps ... ok` (audit/runs/baseline/cargo-test-workspace.txt:1285, ran 0.56s — fixture lane has no corpus gate; the corpus sibling lane `corpus_mpeg2video_lanes_field_assertions ... ok` at 1284 is corpus/ffprobe-gated and vacuous in this sandbox, consistent with that suite's documented loud-skip).

## 3. Findings
None. Checked: packet alignment, no truncation, sane PID distribution (PAT/PMT repeated ~every 70 packets, 96% payload), program/ES/stream-type consistency (0x02 = MPEG-2 video on 256), PES stream_id 0xE0, table CRC bytes present, SDT present.

## 4. Per-function table
N/A (binary asset).

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mpeg12_corpus::fixture_transport_lanes_ts_and_ps (this file's lane) | YES | demuxes the committed bytes through the real TsDemuxer and asserts width/height/profile/level/chroma/fps/interlace against pinned ffprobe truth (mpeg12_corpus.rs:9-20, 26-28); ran in baseline |
| media-probe registry_roundtrip | YES (weakly) | uses this file as a crate fixture for probe roundtripping |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "transport: `mpeg2-in-ts.ts` through the real TsDemuxer" | crates/codecs/tests/mpeg12_corpus.rs:7-8 | REAL | baseline 1285 ok with the fixture lane ungated |
| "Decode is not attempted — the module is honest about that boundary" | mpeg12_corpus.rs:18-19 | REAL | mpeg2 not in the shipped decoder set (rc-smoke engine-info: mjpeg, pcm_s16le); facts-only parser |

## 7. Dead / duplicated / unreachable code
N/A. Referenced by 2 test files.

## 8. Dependencies & flags
Data-only asset (54,520 bytes matches manifest row for this file).

## 9. Open questions for the next agent
None.
