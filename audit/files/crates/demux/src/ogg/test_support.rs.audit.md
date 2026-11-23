# AUDIT: crates/demux/src/ogg/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Lines / functions | 126 / 5 pub helpers + 3 consts |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (test-support surface, documented :5-6) — consumers: ogg/tests.rs (19 tests), ogg/mod.rs:411-412 doctests, ogg/fuzz.rs:51. Note: `pub mod test_support` (ogg/mod.rs:80) is unconditional — same production-compile-graph leak as es/fuzz.rs F-002 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | full-file read; rg caller map; sed quote verification; the 19 ogg tests that execute these builders ran green (see file-12 entry: `cargo test -p demux --lib ogg::tests` → 19/19) |

## 1. One-paragraph verdict
Real deterministic Ogg page builders: correct header layout (capture/version/flags/
granule/serial/seq/CRC/page-segments), CRC patched at the right offset, correct lacing
for zero-length packets, and honest docs about the 255-continuation tool. The builders
are exercised end-to-end by the 19 passing ogg tests. Three footguns/doc lies, all
currently unhittable by existing callers: an exactly-255-byte packet merges with any
following packet in the same call, lacing truncation at 255 silently lies about the
body, and the vorbis ID builder is 31 bytes while its doc claims 30 "canonical".

## 2. What actually works (verified)
— Builders drove the 19/19 green ogg suite (ran under file 12): CRC-correct pages,
  continued packets across pages, −1 granule, BOS/EOS flags — all consumed by the real
  demuxer.
— CRC patch offset math verified: `at = page.len() - body.len() - nsegs - 5` = 22 for
  every page (Ogg CRC field position). Correct.

## 3. Findings
F-001 [S4]
Where: crates/demux/src/ogg/test_support.rs:43-53
Code:
```
        while left > 0 {
            let take = left.min(255);
            segments.push(take as u8);
            let start = packet.len() - left;
            body.extend_from_slice(&packet[start..start + take]);
            left -= take;
            first = false;
            if take == 255 && left == 0 {
                // A run of exactly 255 continues (packet stays open).
                break;
            }
```
What is wrong: A packet of exactly 255 bytes emits lacing `[255]`, which by Ogg
semantics means "this packet continues into the next lacing run". If the caller passes
`[p_255_bytes, q]` in ONE call, the built page decodes as ONE packet of `p+q` bytes,
not two packets. The doc (:32-35) describes the break as "the caller's tool for
continued packets" but the break fires for ANY packet ending at a 255 boundary, not
just the page's last. No current caller passes a 255-byte packet followed by another
in the same call (rg over all call sites), so it is an unloaded footgun.
Confidence: HIGH — Verification: read + rg call sites.
F-002 [S4]
Where: crates/demux/src/ogg/test_support.rs:63-64
Code:
```
    let nsegs = segments.len().min(255);
    segments.truncate(nsegs);
```
What is wrong: When total lacing runs exceed 255 (any packet > 65,025 B, or > 255
packets in one call), the lacing table is silently truncated while `body` keeps ALL
bytes — the produced page is CRC-valid but structurally lying (body longer than the
lacing sums). A caller would get a "valid" page whose tail is garbage by construction,
with no assert or panic. No current caller exceeds it (largest built packet in-tree:
600 B). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/demux/src/ogg/test_support.rs:83-94
Code:
```
/// A minimal Vorbis identification header packet (30 bytes, the
/// canonical BOS body for a 8 kHz mono stream).
#[must_use]
pub fn vorbis_id_packet(rate: u32) -> Vec<u8> {
    let mut v = vec![0x01];
    v.extend_from_slice(b"vorbis");
    v.extend_from_slice(&0u32.to_le_bytes());
    v.push(1); // channels
    v.extend_from_slice(&rate.to_le_bytes()[..3]); // LE24
    v.extend_from_slice(&[0; 16]); // bitrates + block sizes + framing
    v
}
```
What is wrong: The doc claims 30 bytes; the builder emits 1+6+4+1+3+16 = 31 bytes. It
is also not "canonical": the canonical Vorbis ID is 30 bytes with a u32 rate at 12..16
(this writes LE24 at 12..15 — wire-compatible only because the next byte is zero) and a
framing byte whose bit 0 must be SET (this leaves it 0, invalid per RFC 3533 §2.1, so
the builder can never emit a spec-legal BOS). The demuxer accepts it (its parser does
not check the framing bit — a lenient-parser fact worth its own look in ogg/mod.rs).
Confidence: HIGH — Verification: read + byte count.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mem_stream | 19-23 | REAL | — | delegates to avi's BytesStream (one seam) |
| CONTINUED/BOS/EOS | 26-30 | REAL | — | header-type bit values 0x01/0x02/0x04 — correct |
| build_page | 37-81 | REAL | S4 | F-001 merge footgun, F-002 silent lacing truncation; header layout + CRC patch verified correct |
| vorbis_id_packet | 86-94 | REAL | S4 | F-003: 31 bytes, doc says 30, framing bit 0 (never spec-legal) |
| opus_head_packet | 98-108 | REAL | — | 19 bytes, RFC 7845 layout correct |
| demo_stream | 113-126 | REAL | — | BOS/data/EOS trio, used by doctests + 2 tests |

## 5. Test honesty
N/A (builder module — no tests of its own; the 19 consuming ogg tests ran green).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Build one CRC-correct Ogg page" | :32 | REAL | CRC patched at offset 22, verified; pages pass the demuxer's CRC gate |
| "a final run of exactly 255 leaves the packet open" | :33-34 | PARTIAL | fires for ANY 255-boundary packet (F-001) |
| "30 bytes, the canonical BOS body" | :83-84 | FALSE | 31 bytes, non-canonical framing (F-003) |
| "nothing in the demuxer uses it" | :5-6 | REAL | rg: only tests/doctests/fuzz |

## 7. Dead / duplicated / unreachable code
None dead. Duplicated GUID/lacing? No — single builder. The unconditional
`pub mod test_support` compile-graph leak is noted under Reachability (repo-wide
pattern).

## 8. Dependencies & flags
plugin-api, super::page (CAPTURE/crc32/HEADER_BYTES), std Arc. No feature gates.

## 9. Open questions for the next agent
- ogg/mod.rs vorbis ID parser: does it check the framing bit? (If yes, this builder's
  pages would be rejected — they are not, so it does not; confirm and decide which
  side is wrong.)
- page.rs: how does the page layer treat a body longer than the lacing sum (F-002's
  lying page) — damage, truncation, or silent skip?
