# AUDIT: crates/demux/src/adts/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-23 |
| Lines / functions | 188 / 7 (6 builders/consts, 1 delegate) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — `pub mod test_support` (adts/mod.rs:106), doc line 6 admits "nothing in the demuxer uses it". Users: adts/tests.rs:11, adts/fuzz.rs:48, CLI container tests (apps/cli/src/containers/adts.rs:370,395), rustdoc doctest (adts/mod.rs:79-80). All test/doc surfaces. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | sed comparison of header_crc vs demuxer crc16_header (mod.rs:980-990 — identical); baseline cargo-test-workspace.txt:1707-1708+ (adts tests using these builders ok). |

## 1. One-paragraph verdict
Real, spec-correct byte builders: the ADTS frame header bit-packing (syncword,
profile/sampling/channel fields, 13-bit frame length split across head[3..5],
11-bit buffer fullness, blocks-1) matches the MPEG-2 ADTS layout field-for-
field; the ID3v2 syncsafe size encoding is correct; the ADIF bit-stream
builder derives the bitrate so the demuxer's CBR arithmetic reproduces
frame_len exactly; the CRC twin is line-identical to the demuxer's check.
The payloads are non-degenerate ramps (0 % 251-pattern), so truncation bugs
cannot hide in zero runs. Two hygiene defects: four silent out-of-range input
manglings, and the CRC duplication.

## 2. What actually works (verified)
- read: `header_crc` (28-41) is byte-for-byte the same algorithm as the
  demuxer's private `crc16_header` (adts/mod.rs:980-990) — the "same check
  the demuxer applies" claim (line 25-26) is true (verified by comparison).
- read: ADTS field layout cross-checked — head[3] carries
  `(channel_config&3)<<6 | flen>>11 & 3`, head[4] `flen>>3`, head[5]
  `(flen&7)<<5 | fullness>>6`, head[6] `0xFC | (blocks-1)` — all match the
  7-byte ADTS header spec.
- Baseline cargo-test-workspace.txt:1707-1708 (`adif_bad_params_are_typed`,
  `adif_cbr_walk_matches_the_block_arithmetic` ok) — the ADIF builder's
  derived-arithmetic contract is exercised and green; the ADTS builder
  drives `adts::tests::*` (baseline ok).

## 3. Findings
F-001 [S4]
Where: crates/demux/src/adts/test_support.rs:57-59, 127, 129, 143
Code:
```
    let blocks = blocks.clamp(1, 4);
    let total = 7 + 2 * usize::from(protection.is_some()) + payload_len;
    let flen = u16::try_from(total).unwrap_or(u16::MAX);
```
```
    let rate = SAMPLE_RATES[usize::from(sampling_index.min(12))];
```
```
    push(&mut bits, u64::from(bitrate.min(0x7F_FFFF)), 23);
```
What is wrong: four silent input manglings — blocks outside 1-4 clamped,
total > 65535 saturates the frame length to u16::MAX (a LYING header: the
frame claims 65535 bytes but carries `total`), sampling_index > 12 clamped,
bitrate > 0x7FFFFF truncated so the demuxer's derived frame length no longer
reproduces frame_len. A test author passing an out-of-range value gets a
frame whose declared fields disagree with its bytes, and the failure lands
far from the cause. Builders used to forge deliberately-malformed frames
should take the values verbatim or panic on impossible input.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/demux/src/adts/test_support.rs:28-41 vs adts/mod.rs:979-990
Code:
```
pub fn header_crc(head7: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for byte in head7 {
        crc ^= u16::from(*byte) << 8;
```
What is wrong: a 12-line duplicate of the demuxer's private `crc16_header` —
the "same check" property is maintained by hand-sync, not by a shared fn.
If one side ever changes (e.g. xorout added), forged frames stop matching
the demuxer's check and the protected-frame tests fail confusingly — or
worse, keep passing against a stale twin if the demuxer's change is
behavior-preserving for the test's vectors.
Confidence: HIGH — Verification: read + diff by eye (identical bodies).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mem_stream | 19-23 | REAL | — | delegate to avi seam |
| header_crc | 28-41 | REAL | S4 | F-002 duplication |
| adts_frame | 49-83 | REAL | S4 | F-001 mangling; bit layout spec-correct |
| adts_stream / demo_stream | 87-104 | REAL | — | |
| id3v2_tag | 109-118 | REAL | — | correct syncsafe size |
| adif_stream | 126-184 | REAL | S4 | F-001 mangling; derived-bitrate contract works (baseline) |
| BLOCK_SAMPLES | 188 | REAL | — | re-export |

## 5. Test honesty
This file is a builder, not a test file; honesty = do the frames it builds
mean what the tests claim? Yes — layouts verified against the spec by read,
ramp payloads, and the forged-CRC path is genuinely the demuxer's check.
The clamping paths (F-001) are the one dishonesty: a builder that silently
rewrites its inputs.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the same check the demuxer applies" | test_support.rs:25-26 | REAL | mod.rs:980-990 identical |
| "zero runs hide truncation bugs" (ramp rationale) | test_support.rs:77-79 | REAL | payloads are i%251 ramps |

## 7. Dead / duplicated / unreachable code
F-002 CRC twin. The module itself is production-dead by design (test surface
in a `pub mod` — same shipped-but-unused pattern as asf/avi test_support).

## 8. Dependencies & flags
No external deps. Uses adts::{SAMPLE_RATES, SAMPLES_PER_BLOCK}. No feature
gates.

## 9. Open questions for the next agent
- The ADIF builder's comment_field_bytes=0 and mono/stereo/matrix mixdown
  bits assume the demuxer ignores them — is there a fixture where a nonzero
  comment field exercises the demuxer's skip arithmetic? (No test found in
  adts/tests.rs for a padded ADIF comment field.)
