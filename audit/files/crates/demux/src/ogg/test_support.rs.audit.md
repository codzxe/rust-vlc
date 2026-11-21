# AUDIT: crates/demux/src/ogg/test_support.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-21 |
| Lines / functions | 126 / 6 (1 delegating fn, 3 consts, 3 builders) |
| File verdict | REAL (test-support surface; compiled into the shipped library but never called by production code — doc claim verified) |
| Reachable from shipped entry point? | The file: compiled-in dead weight (pub mod, unconditional — ogg/mod.rs:80), zero production callers (rg: only ogg tests/doctests + codecs integration tests use `mem_stream`). The subject it builds fixtures for (`OggDemuxer`): YES — `apps/cli/src/containers/mod.rs:68` `"ogg" => ogg::open`. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | `rg -n "mod test_support" crates/demux/src/ogg/mod.rs` (80: pub, unconditional); `rg -ln "ogg::test_support"` (users: ogg/mod.rs doctests, ogg/tests.rs, codecs/tests/{vorbis,opus,vorbis_bitexact} — the latter three only `mem_stream`); `rg -n "build_page\(" ogg/tests.rs` (max observed segment counts small; 255-run continuation tests stay within one page's budget); `rg -n "vorbis\|rate" ogg/mod.rs` (demuxer parse at 247-258 traced); `ls crates/demux/tests/ | rg ogg` (corpus_ogg.rs, ogg_fixtures.rs — real ffmpeg fixtures); baseline: `rg -c "test ogg::"` → 31, all ok. |

## 1. One-paragraph verdict
A small, genuinely used test-builder module (four consumer suites; 31 ogg
tests green in the baseline — `orchrun-test-workspace-nofailfast.log.txt`,
`test ogg::…` count 31, e.g. 1814-1816) whose CRC arithmetic and lacing
semantics are correct — the Ogg CRC is computed over the page with a zeroed
CRC field then patched at byte 22, exactly per spec, and the 255-run
"packet stays open" continuation rule is implemented (pinned by
`continued_flag…` tests). Two defects: `build_page` silently truncates the
segment table at 255 while keeping every body byte — producing a CRC-valid
page whose lacing no longer accounts for its payload, with no error, no doc
note, and no current test that trips it (a footgun for every future fixture
author); and `vorbis_id_packet`'s doc claims 30 bytes while building 31, in a
layout that diverges from the Vorbis I spec (LE24 rate + 16-byte tail vs the
spec's LE32 rate + 14-byte tail with framing=1) — it parses only because the
demuxer's own parse (mod.rs:251-254) mirrors the same LE24 divergence.

## 2. What actually works (verified)
— CRC round-trip: `bad_crc_page_is_dropped_with_sync_recovery ... ok` and the
   30 other ogg tests pass (baseline 1814+; count 31) — every page the builder
   emits passes the demuxer's CRC gate, proving the zero-then-patch CRC at
   76-79 is spec-correct.
— Lacing: 255-run continuation semantics (50-53) pinned by the continued-
   packet tests (tests.rs:181-193: 255-run open page + continued page with
   255+90 terminal); zero-length packet → single zero lacing (55-58) is the
   Ogg rule.
— Real-bytes anchor exists: `crates/demux/tests/corpus_ogg.rs` — ffmpeg
   7.1.5-generated vorbis fixtures demuxed bit-exactly against recorded ffmpeg
   raw-dump truth (header docs at tests/corpus_ogg.rs:1-40), so the demuxer's
   lenient vorbis parse is proven on REAL 30-byte spec-shaped files, not just
   this builder's shape.
— `opus_head_packet` matches RFC 7845 exactly: 8+1+1+2+4+2+1 = 19 bytes,
   pre-skip 312 (the ffmpeg-typical value) — verified field-by-field.

## 3. Findings
### F-001 [S3]
Where: crates/demux/src/ogg/test_support.rs:63-75
Code:
```
    let nsegs = segments.len().min(255);
    segments.truncate(nsegs);
    let mut page = Vec::with_capacity(page::HEADER_BYTES + nsegs + body.len());
    page.extend_from_slice(page::CAPTURE);
```
What is wrong: If the packets' lacing runs exceed 255, the segment TABLE is
silently truncated to 255 — but `body` (lines 39, 47) still contains every
byte of every packet. The emitted page is structurally inconsistent: its
lacing sum no longer accounts for its body length, yet the CRC is computed
over the final bytes (76-79) so the page passes the demuxer's CRC gate. No
error, no assert, no doc note ("each packet is laced into 255-byte runs",
32-35, never mentions the page segment budget).
Why it matters: this is the shared fixture tool for four test suites; a
future test that builds a page from 256+ runs (e.g. 256 tiny packets, or one
65,280-byte packet) gets a CRC-valid LYING page instead of what it asked for
— the fixture silently tests a different demuxer path (body-overrun damage
handling) than the author intended. No current test trips it (verified: the
ogg tests' largest pages carry 1-2 packets; the 255-run tests use single
runs), which makes it worse, not better — the hazard is invisible until the
day it corrupts a fixture.
What correct looks like: `assert!(segments.len() <= 255, "page segment budget exceeded: {} runs", segments.len())` — a builder that refuses what it cannot express — or split into multiple pages.
Confidence: HIGH — Verification: read; segment accounting traced; current call-site census shows no trigger.
### F-002 [S4]
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
```
What is wrong: The doc says 30 bytes; the builder emits 31 (1+6+4+1+3+16).
And the layout diverges from the Vorbis I identification header: the spec's
sample rate is a 32-bit LE field at offset 12 with a 14-byte tail (bitrates
12 + blocksize 1 + framing 1 = 30 bytes total, framing bit = 1); this builder
writes a 24-bit rate and a 16-byte zero tail (framing bit 0). It parses only
because the demuxer's `identify` (ogg/mod.rs:248-258) reads the same LE24 at
12-14 and never checks length/framing — builder and demuxer mirror each
other's divergence.
Why it matters: a spec-correct Vorbis decoder rejects this packet (framing=0,
wrong length); the demuxer's leniency (accepting ≥15 bytes, ignoring framing)
is un-flagged by any test because the shared builder never produces a
spec-shaped ID header. The real-file differential (corpus_ogg.rs, ffmpeg
vorbis fixtures, bit-exact) proves the demuxer on real bytes — so this is not
a live bug — but the doc's "30 bytes" is false and the toy shape hides the
demuxer's spec divergence from every test written against this builder.
What correct looks like: build the spec shape (LE32 rate at 12, 14-byte tail,
framing bit set, 30 bytes) and fix the doc; separately, the demuxer should
validate the framing bit (ogg/mod.rs is another agent's file).
Confidence: HIGH — Verification: read; spec layout from Vorbis I §4.2.1; demuxer parse traced at mod.rs:247-258; corpus differential existence verified.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mem_stream` | 19-23 | REAL | — | Thin delegation to avi's seam; used by 3 codecs integration suites. |
| `CONTINUED`/`BOS`/`EOS` | 26-30 | REAL | — | Correct Ogg header-type bits (0x01/0x02/0x04). |
| `build_page` | 37-81 | REAL | S3 | Lacing + CRC correct (spec order, zero-then-patch CRC at byte 22 — arithmetic verified: 27+nsegs+body − body − nsegs − 5 = 22); `granule -1` → 8×FF ✓; F-001 silent 255-segment truncation. |
| `vorbis_id_packet` | 86-94 | REAL | S4 | F-002: 31 bytes built vs "30" documented; spec-divergent LE24+16-tail shape; parses only under the mirroring demuxer parse. |
| `opus_head_packet` | 98-108 | REAL | — | RFC 7845-exact (19 bytes, pre-skip 312); no defects found. |
| `demo_stream` | 113-126 | REAL | — | BOS/data/EOS with consistent serial 0xAA11 and seq 0-2; granules 0/512/1024 on the sample clock. |

## 5. Test honesty (this file IS the test tool; honesty = do its products pin what they claim)
| Product | Would a defect in the builder break the consuming tests? | Reason |
|---|---|---|
| CRC-correct pages | YES (verified) | A CRC bug would fail every parse test (`bad_crc_page…` proves the gate is live). |
| 255-run continuation pages | YES | tests.rs:181-193 pin exact reassembly across an open packet. |
| Vorbis ID shape | NO (spec divergence) | The builder's shape and the demuxer's parse co-drift (F-002) — only the ffmpeg corpus differential (corpus_ogg.rs) anchors the real shape. |
| >255-segment pages | NOTHING (untriggered) | F-001: no test builds them; a silent lie would go unnoticed. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Test-support surface, not player surface — nothing in the demuxer uses it" | test_support.rs:5-6 | REAL | rg: production callers zero; users are tests/doctests/integration only. (It is nonetheless compiled into the shipped library — `pub mod test_support` unconditional, ogg/mod.rs:80 — dead weight, same pattern as adts/avi/asf test_support.) |
| "A minimal Vorbis identification header packet (30 bytes…)" | test_support.rs:83-84 | FALSE | F-002: 31 bytes built; layout also spec-divergent. |
| "each packet is laced into 255-byte runs; a final run of exactly 255 leaves the packet open" | test_support.rs:32-35 | REAL | Implemented at 43-54; pinned by continuation tests (baseline `continued_flag_lies_are_damage_noted ... ok`, 1816). |

## 7. Dead / duplicated / unreachable code
- The module ships in the production library but has no production caller
  (documented and verified) — crate-wide pattern, noted, not re-faulted here.
- No intra-file duplication; `mem_stream` intentionally re-exports avi's seam
  (the "shared test seam" pattern).

## 8. Dependencies & flags
No feature gates; std-only; no unsafe. Depends on `super::page` (CAPTURE,
HEADER_BYTES, crc32) and avi's test_support. The 31 ogg tests + corpus
differential ran green in the baseline (default selection, release profile
deviation per INDEX.md).

## 9. Open questions for the next agent
- ogg/mod.rs auditor (A02/A03 own its parts): the vorbis leniency (length ≥15,
  LE24 rate, no framing check — 248-258) is spec-divergent; only the corpus
  differential covers the real shape. Worth a finding there.
- If any future test needs >255 lacing runs per page, F-001 bites first —
  the assert should land before that test does.
