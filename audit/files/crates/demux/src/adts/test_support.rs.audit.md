# AUDIT: crates/demux/src/adts/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-21 |
| Lines / functions | 188 / 8 (mem_stream, header_crc, adts_frame, adts_stream, demo_stream, id3v2_tag, adif_stream, BLOCK_SAMPLES) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A by design (test-support surface, "nothing in the demuxer uses it", module doc line 6); used by adts unit tests, the mod.rs doctest (baseline line 6365 `crates/demux/src/adts/mod.rs - adts (line 74) ... ok`), fuzz, and `tests/adts_corpus.rs` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | full read; bit-layout verification against ffmpeg's `adtsenc.c` write order (profile, sf_index, **private_bit, channel_configuration**, original, home) and against the committed ffmpeg fixture bytes (`od` of `fixtures/adts/0059-flip-idx15.bin` → `ff f1 44 78` parses consistently); `rg -n "fn crc16" crates/demux/src/adts/mod.rs` (duplication check); `rg -n "protection|Some(false)|Some(true)" crates/demux/src/adts/tests.rs` (self-forged CRC frames); `od` head of the committed fixtures (protection_absent=1 — unprotected); baseline log lines 1492-1517, 1957-1966, 6365 |

## 1. One-paragraph verdict
Correct, spec-faithful byte builders. I verified the ADTS fixed-header bit layout
against ffmpeg's write order (`private_bit` precedes `channel_configuration` in
byte 2 — both the builder and the demuxer's `AdtsHeader::parse` (mod.rs:203-204) get
this right, confirmed against the committed ffmpeg-authored fixture bytes), the ID3v2
syncsafe size encoding, and the ADIF bit-level header + program_config_element layout
including the SCE/CPE front-element logic (1→SCE, 2/4/6→CPEs, 3/5→SCE+CPEs — traced
correct). Payloads are labeled ramps, not zero runs ("zero runs hide truncation bugs")
— deliberate, documented test-honesty. One real gap: the CRC-16/MPEG-2 that forges
protected frames is a **byte-for-byte duplicate** of the production `crc16_header`,
every protected-frame test self-forges with that same algorithm, the committed
ffmpeg fixtures are unprotected (`head[1] = 0xf1` → protection_absent), and the
env-gated corpus that would provide independently-authored protected frames
vacuous-passes in the baseline — the CRC verification path has **no independent
evidence anywhere in the committed tree**. Two smaller builder edges: silent
sampling-index clamp in `adif_stream`, and unrepresentable frame lengths saturate to
`u16::MAX` without a word of documentation.

## 2. What actually works (verified)
— verified by `od -A d -t x1 crates/demux/fixtures/adts/0059-flip-idx15.bin` → `ff f1 44 78 46 9f fc`: the committed ffmpeg bytes parse under the same bit layout the builder writes (profile=01, sf_index=1, private=0, cc=1, flen=564) — builder and real-world writer agree.
— verified by baseline log: doctest using this module green (line 6365); 24 adts demuxer tests green (1494-1517) incl. `crc_mismatch_is_typed_and_good_crc_walks` (1506) — the forged good-CRC frame walks and the forged bad-CRC frame is typed, proving the demuxer's CRC gate fires on the builder's output.
— verified by read: `adts_frame` field packing traced bit-by-bit against ISO 13818-7 §6.2.1 / ffmpeg adtsenc order — correct, including buffer_fullness 0x7FF spread across head[5] bits 4-0 and head[6] bits 7-2, and `blocks-1` in head[6] bits 1-0 with the `clamp(1, 4)` guard.

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/adts/test_support.rs:28-41 (duplicate of crates/demux/src/adts/mod.rs:979-990)
Code:
```rust
pub fn header_crc(head7: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for byte in head7 {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}
```
What is wrong: This is a byte-for-byte copy of the production `crc16_header`
(adts/mod.rs:979-990, private). Every protected-frame test self-forges its CRC with
this copy: `adts_frame(…, Some(false))` (e.g. tests.rs:73, 259). The committed
ffmpeg-authored fixtures are all unprotected (`od` of the fixtures/adts/*.bin heads →
`head[1] = 0xf1`, protection_absent=1), and the env-gated corpus suite — the only
place independently-authored protected frames could come from — vacuous-passes in the
baseline (corpus absent; see my log correction of this session). So the demuxer's CRC
rejection/acceptance has zero evidence against bytes not produced by this same
algorithm: a spec-level error in the CRC (wrong init, wrong coverage window) would be
duplicated into both copies and **every test would stay green** while real protected
ADTS files fail (or corrupt ones pass).
Why it matters: A verification algorithm whose only verification is itself. The
failure direction is fail-closed (real protected files refused as crc-mismatch), so
severity is S3 not S2, but the evidence hole is total.
What correct looks like: (a) make `crc16_header` `pub(crate)` and have the builder
call it (removing the duplication), AND (b) commit one ffmpeg-authored
`-write_adts_crc` fixture with its measured CRC so the acceptance path has
independent truth.
Confidence: HIGH — Verification: read (both implementations compared line-by-line; fixture bytes od'd; corpus skip mechanism read at adts_corpus.rs:104-124)

### F-002 [S4]
Where: crates/demux/src/adts/test_support.rs:127
Code:
```rust
pub fn adif_stream(sampling_index: u8, channels: u16, frames: usize, frame_len: usize) -> Vec<u8> {
    let rate = SAMPLE_RATES[usize::from(sampling_index.min(12))];
```
What is wrong: The rate lookup silently clamps `sampling_index` to 12 while line 148
writes the **raw** index into the header — calling `adif_stream(13..=15, …)` builds a
header declaring an invalid index (which the demuxer refuses via
`adif_bad_params`) with a bitrate derived from rate[12] under a different index's
arithmetic. The clamp masks the caller error instead of panicking or refusing
(test-support may panic freely per house style).
Why it matters: A test author asking for index 14 gets a fixture whose bytes say 14
but whose bitrate math says 12 — if the demuxer ever widens its index acceptance,
the fixture becomes silently wrong.
What correct looks like: `assert!(sampling_index <= 12)` (test-support code may
panic), or clamp both the declaration and the lookup consistently.
Confidence: HIGH — Verification: read (line 148 `push(&mut bits, u64::from(sampling_index), 4)` writes the raw value)

### F-003 [S4]
Where: crates/demux/src/adts/test_support.rs:58-59
Code:
```rust
    let total = 7 + 2 * usize::from(protection.is_some()) + payload_len;
    let flen = u16::try_from(total).unwrap_or(u16::MAX);
```
What is wrong: For `payload_len > 65_521` (resp. 65_519 with protection) the
13-bit frame_length field cannot represent the total; the builder silently saturates
to `u16::MAX`, producing a frame whose declared length is a **lie** with no
documentation and no assertion. A test author requesting a large payload gets a
frame the demuxer will read as 65_535 bytes (and then likely fail typed-truncated),
not the frame they asked for.
Why it matters: Silent fixture corruption at an edge; the house style for
test-support builders is to panic loudly on impossible requests.
What correct looks like: `assert!(total <= u16::MAX as usize, "frame_length overflow: {total}")`.
Confidence: HIGH — Verification: read (ADTS frame_length is 13 bits — max 8191? No: 13 bits max 8191 — see note)

NOTE on F-003 self-correction: ADTS `frame_length` is 13 bits (max 8191), and the
builder packs it via `(flen >> 11) & 0x03` (2 bits) + `(flen >> 3)` (8 bits) +
`(flen & 0x07) << 5` (3 bits) — 13 bits total, so the real saturation point is
`total > 8191`, not 65_535: any `payload_len > 8184` already builds a lying frame
(high bits silently truncated by the shifts). The finding stands with a wider blast
radius than the `u16::try_from` alone suggests. Confidence HIGH (bit-width recount
verified twice).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mem_stream` | 19-23 | REAL | — | properly delegates to the shared `avi::test_support::mem_stream` (the anti-pattern riff.rs's private MemStream violates) |
| `header_crc` | 28-41 | REAL | F-001 | duplicate of production crc16_header; textbook CRC-16/MPEG-2 shape (poly 0x1021, init 0xFFFF, MSB-first, no xorout) |
| `adts_frame` | 49-83 | REAL | F-003 | bit layout verified against ffmpeg adtsenc order; blocks clamp(1,4); ramp payload (anti-zero-pad, documented); flen saturation (F-003) |
| `adts_stream` | 87-93 | REAL | — | uniform LC/44.1k/stereo/100B unprotected frames |
| `demo_stream` | 98-104 | REAL | — | doctest shape (3×8B) |
| `id3v2_tag` | 109-118 | REAL | — | syncsafe 28-bit size (4×7-bit groups, >>21/>>14/>>7) — correct per ID3v2.3 |
| `adif_stream` | 126-184 | REAL | F-002 | bit-level ADIF+PCE layout traced against ISO 14496-3 field order; SCE/CPE logic correct for 1/2/3/4/5/6 channels; bitrate derived so `bitrate×128/rate` reproduces frame_len (floor-division edge noted, harmless); index clamp (F-002) |
| `BLOCK_SAMPLES` | 188 | REAL | — | re-export for suite arithmetic |

## 5. Test honesty
This file IS test support; its own honesty is measured by whether the fixtures it
builds can lie. Mutations: breaking `header_crc` breaks `good_crc_walks` (the forged
good frame would mismatch) — but ONLY because both copies would need to break
together to hide; a shared spec error is invisible (F-001). Breaking the bit packing
(e.g. cc at bit 1) fails the ffmpeg-fixture-derived parses and
`channel_config_table_resolves_the_documented_counts` (codecs suite, baseline:240).
The ramp-payload design genuinely catches truncation (zero runs would not).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the same check the demuxer applies — exported so builders can forge protected frames" | doc 25-26 | REAL (it is the same) / and that is the problem | F-001: same-by-copy, not by-call |
| "Test-support surface, not player surface — nothing in the demuxer uses it" | module doc 6 | REAL | rg: no non-test references |
| "Deterministic payload: a byte-labeled ramp (verifiable, not zero-padded — zero runs hide truncation bugs)" | doc 77-78 | REAL | `(i % 251)` ramps in both builders |

## 7. Dead / duplicated / unreachable code
- `header_crc` — duplication of production code (F-001).
- `demo_stream` vs `adts_stream` — near-overlaps (3×8B vs N×100B) but serve
  different call sites (doctest vs suites); acceptable.

## 8. Dependencies & flags
`plugin_api::SourceStream`, `super::{SAMPLE_RATES, SAMPLES_PER_BLOCK}` (production
constants — good: the builder's rate table cannot drift from the demuxer's), std
only. No features, no unsafe, `#[must_use]` discipline on builders.

## 9. Open questions for the next agent
- Commit an ffmpeg `-write_adts_crc` fixture and pin its parse (closes F-001's
  evidence hole; `REQUEST-ORCH-RUN: ffmpeg -i <aac> -c copy -write_adts_crc 1 -f adts /tmp/protected.adts` + a fixture-generation xtask pass, if the corpus lane is ever
  re-run).
- The ADIF lane has no ffmpeg reference at all (ffmpeg has no ADIF muxer/demuxer —
  measured, corpus test doc 32-35) — the cross-container
  `adts_and_adif_agree_on_the_same_payload` test is the only ADIF truth; consider a
  hand-verified ISO reference vector for one ADIF header.
