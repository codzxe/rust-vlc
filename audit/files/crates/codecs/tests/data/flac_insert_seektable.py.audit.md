# AUDIT: crates/codecs/tests/data/flac_insert_seektable.py
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 132 / 3 (crc8, parse_header, main) |
| File verdict | REAL (executed; output byte-verified against the committed fixture's recorded sha256) |
| Reachable from shipped entry point? | N/A (test-data generator, recorded provenance chain) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | executed the script twice (wrong short source → 5 frames/1 point, then the recorded 4.5 s source command); `ffmpeg` regeneration + `sha256sum` -> exact match with flac_bitexact_reference.txt:86 |

## 1. One-paragraph verdict
A real, spec-faithful FLAC SEEKTABLE generator: CRC-8 (poly 0x07, init 0), full frame-header parse (sync, blocking strategy, block-size/sample-rate escapes, UTF-8-coded frame number, CRC-8 gate), every-8th-frame points, PADDING dropped, table appended as last block. I verified the ENTIRE provenance chain end-to-end: regenerated the source with the reference file's exact ffmpeg command (aevalsrc d=4.5), ran this script (44 frames, 6 points, 108 table bytes), and the output's SHA-256 (3f0cfc4d…a0cc) matches the committed fixture's recorded digest digit-for-digit. The script, fixture, and reference data are mutually consistent and reproducible. Hygiene notes only.

## 2. What actually works (verified)
— verified by execution: `python3 flac_insert_seektable.py /tmp/seektable_src.flac /tmp/seektable_full.flac` -> `frames=44 points=6 seektable_bytes=108 out_bytes=59380`; `sha256sum` -> `3f0cfc4d8e56ba388e6bbd41ab45c8b4fadb5408c86b3e0af4895531eaf3a0cc` == reference line 86 (the committed fixture's digest).
— verified the source command reproduces: ffmpeg 7.1.5 with the recorded aevalsrc line produced the identical intermediate.
— verified the script's seek points cross-check in Rust: flac_bitexact's `seektable_carrier_exposes_six_verified_points` (executed earlier this session, ok) resolves every point against frame headers.

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/data/flac_insert_seektable.py:122-123
Code: (verbatim)
```
        last = False
        out += bytes([0x80 if last else 0x00 | btype]) + len(body).to_bytes(3, "big") + body
```
What is wrong: dead assignment (`last` is always False here); the intent (non-last flag for retained blocks) is correct but the line is confusing residue. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/codecs/tests/data/flac_insert_seektable.py:39-69
Code: (excerpt, L39)
```
    first = data[off + cur]
```
What is wrong: after the initial 5-byte guard (L26), later indexing (`data[off + cur]` with cur up to 13, L62/L65) can raise IndexError on a truncated tail — the script crashes instead of skipping. Harmless for its committed inputs (verified: runs clean); a robustness nit for reuse. Confidence: HIGH — Verification: read; executed without incident on valid input.
F-003 [S4]
Where: crates/codecs/tests/data/flac_insert_seektable.py:36
Code: (verbatim)
```
    if (b3 >> 4) > 0b1010 or ((b3 >> 1) & 7) in (0b011, 0b111) or (b3 & 1):
```
What is wrong: rejects channel-assignment 3 (stereo right/side — a legal spec value) alongside the reserved 7; the docstring claims "the same rules crates/codecs/src/audio/flac/seek.rs implements" — if seek.rs shares the over-rejection, valid RS-stereo streams lose seek points. UNVERIFIED on the Rust side (seek.rs not in my assignment). No effect on the committed fixtures (ffmpeg emits LR/MS only). Confidence: MEDIUM — Verification: read; spec comparison from memory.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| crc8 | 16-22 | REAL | - | poly 0x07 init 0; gates header acceptance (verified by successful 44-frame scan) |
| parse_header | 25-80 | REAL | S4 | spec §6/§9.1 walk incl. UTF-8 number + escapes + CRC-8; F-002/F-003 |
| main | 83-128 | REAL | S4 | metadata walk, fixed-bs assert, point table >QQH (correct 18-byte seekpoints), PADDING drop, last-block flag; F-001 |

## 5. Test honesty
The script itself is generator tooling; its output is verified downstream: byte-exact fixture digest (my execution) + the Rust seektable resolution test (executed this session).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "inserts one computed by scanning frame headers with the spec sec.6 rules ... the same rules seek.rs implements" | L5-9 | REAL for the scan (44/44 frames found on the real source); "same rules as seek.rs" UNVERIFIED | my execution; seek.rs not assigned |
| reference: "run the committed script ... drops PADDING, inserts 6-point table" | flac_bitexact_reference.txt:33-34 | REAL | my run: 6 points, PADDING dropped, byte-identical result |

## 7. Dead / duplicated / unreachable code
Dead `last = False` (F-001). No duplication.

## 8. Dependencies & flags
python3 stdlib only (struct, sys). Not shipped; a recorded-provenance generator.

## 9. Open questions for the next agent
- Does seek.rs reject channel-assignment 3 too (F-003)? If yes, that is an actual decoder-side over-rejection worth an S3 there.
