# AUDIT: crates/demux/src/ogg/mod.rs#part1 (lines 1-492)
| Field | Value |
|---|---|
| Auditor | W04-A10 |
| Date | 2026-11-21 |
| Lines / functions | 492 (chunk 1-492 of 1476; part2=493-984 A05, part3=985-1476 A03) / 20 items |
| File verdict | REAL (with one S2 spec-deviation in the Theora lane) |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.ogg|.oga|.ogv|.opus|.spx` → `crates/media-probe/src/signature/phase5.rs:100 OggSignature` ("OggS" capture, registered mod.rs:136) → `apps/cli/src/containers/mod.rs` "ogg" arm → containers/ogg.rs → `OggDemuxer::new/start/next_event`. Baseline: 19 `ogg::tests::` ok + doctest `ogg::OggDemuxer (line 406) ... ok` (nofailfast:6362-6370), demux lib 462/0/0 (:1955)) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | full Read 1-492 + targeted reads 828-915, 1009-1070, 1390-1477 (cross-chunk evidence, read-only); `sed -n` on every quoted range; `ls /tmp/my-project/corpus-v4/media` → absent; baseline greps: nofailfast:2003-2007 (corpus_ogg binary), :2543 (`corpus_dirty_outcome_classes_hold ... ignored, randomized-corpus tier 2: set LUMEN_CORPUS to activate`), doctest evidence :6362; `grep -rn "ogg::probe"` → doc-comment mention only (phase5.rs:96), zero callers; `grep -n "MAX_HEADER_BYTES"` → enforced as a byte budget at :1274; spec-layout arithmetic for Theora/Vorbis/Speex/FLAC headers done by hand (see F-001 forensics) |

## 1. One-paragraph verdict
Part 1 is the Ogg demuxer's identification layer — codec magic sniffing, granule clocks, stream state — and most of it is genuinely correct against the specs I checked by hand: Vorbis LE24 rate at bytes 12-14, Speex rate at 36-39 after the 20-byte version string, FLAC-in-Ogg's 20-bit rate at packet bytes 14-16 behind the 4-byte native block header, Theora PICW/PICH/FRN/FRD at 14/17/22/26, and the Clock timebase/nanos/granule arithmetic verified exact in both directions. But the Theora keyframe-granule shift is extracted from the wrong bytes — the code reads bytes 40-41 where the Theora I specification packs KFGSHIFT into bytes 43-44 (after PARN 24, PARD 24, CS 32, NOMBR 24) — and the unit test that "pins" the offset uses a synthetic array that is internally impossible as a real header, while the corpus differential that the comment cites as its authority skipped in every committed run (corpus absent; the skip path still reports "ok"). Every real .ogv therefore gets wrong frame counts, durations, keyframe flags and seek landings.

## 2. What actually works (verified)
- `identify` non-Theora lanes verified against formats: Vorbis ID (magic `\x01vorbis`, rate LE24 @12-14, `rate > 0` guard, 247-258); Opus (`OpusHead` → fixed 48 kHz clock per RFC 7845 §4.2, 260-262); Speex (magic 8 B + 20-byte version ⇒ rate LE32 @36-39 — matches libspeex's header layout; the `try_into` Err arm at 284 is documented-unreachable residue, 279-289); FLAC-in-Ogg (magic; clock deferred to STREAMINFO, 276-277); Skeleton (`fishead\0`, 291-293).
- `flac_streaminfo_rate` (307-314): 20-bit rate at bytes 14-16 of the second header packet (4-byte native block header + STREAMINFO) — bit arithmetic `(b14<<12)|(b15<<4)|(b16>>4)` verified exact; length-guarded.
- `Clock` arithmetic (195-243): `base()` literal 1/rate equals `TimeBase::new`'s Ok value (gcd(1,r)=1) — comment claim verified; Theora `TimeBase::new(den,num)` = one tick per frame ✓; `nanos()` u128 intermediate with negative-granule rejection ✓; `granule_of_nanos()` is the exact inverse for sample clocks (unit test 1390-1405 pins both directions, baseline-ok).
- `is_header_packet` magic sets (166-178): Vorbis 0x01/0x03/0x05 ✓, Theora 0x80..=0x82 ✓, OpusHead/OpusTags ✓, FLAC `00 00 00 22` (STREAMINFO block header, type 0, len 34) ✓.
- `probe` (120-122) is correct but dead (F-003); struct state machines (`LogicalStream`, `StickyShape`) are plain data, no logic to fail; `new` (462-482) rejects unseekable/unknown-length/empty sources via `SourceReader` (riff.rs:219-236).
- Baseline execution: 19/19 ogg unit tests + the module doctest ok (nofailfast:6362-6370).

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/ogg/mod.rs:263-275
Code:
```rust
    if first_packet.starts_with(&[0x80, b't', b'h', b'e', b'o', b'r', b'a'])
        && first_packet.len() >= 42
    {
        // Theora info: PICW BE24@14, PICH BE24@17, FRN BE32@22, FRD
        // BE32@26, KFGSHIFT packed in the QUAL/KFG/PF tail — offsets
        // pinned empirically against libtheora-encoded fixtures and
        // the corpus .ogv (see the unit tests + the differential).
        let num = be32(first_packet, 22);
        let den = be32(first_packet, 26);
        let shift = (u32::from(first_packet[40] & 0x03) << 3) | u32::from(first_packet[41] >> 5);
```
What is wrong: the KFGSHIFT extraction reads bytes 40-41. Per the Theora I specification's identification header the tail after FRD is: PARN (24-bit, bytes 30-32), PARD (24-bit, 33-35), CS (32-bit, 36-39), NOMBR (24-bit, 40-42), then QUAL (6) + KFGSHIFT (5) + PF (2) packed into bytes 43-44 of the 45-byte header. `shift` therefore reads NOMBR/bitrate bits, not the keyframe shift — the extraction is at least two bytes early (the bit pattern `(b&0x03)<<3 | b>>5` itself is right; the byte offset is not).
Why it matters: every real .ogv (`.ogv` is in `EXTENSIONS`:115 and the probe sniffs it) mis-parses the shift, and the shift feeds frame arithmetic (`Clock::nanos` 228-231: `ticks = (g>>shift)+(g&mask)`), span ends, `track_duration`, Theora keyframe flags (1035-1041: `(granule & ((1<<shift)-1)) == 0`), and seek landing. Three independent proofs the in-file pin is bogus: (1) the unit-test array (mod.rs:1438-1442, "The corpus .ogv's identification header") is internally impossible — its bytes 36-39 are `00 03 0D 40` = 200000, a bitrate, sitting in the spec's CS field whose only legal values are 0-2, and it is 42 bytes where the spec header is 45; (2) byte forensics: that array's tail `[00 03 0D 40 00 C0]` is exactly the spec-layout bytes 39-44 of a CS=0 / NOMBR=200000 / KFGSHIFT=6 / PF=0 file — a 3-byte-left splice of the real tail onto an otherwise spec-faithful prefix (PICW 360, PICH 640, FRN 1, FRD 2 all decode correctly); (3) the "differential" cited as authority never executed (F-002) — on the real 200-kbps corpus file, bytes 40-41 are `03 0D`, so the code would compute shift = 8 and report 65 ticks / 130 s for a 2-frame / 4 s file; the corpus truth pins span end 2 and 4.0 s (corpus_ogg.rs:93-97) — it would fail, which is exactly why the skip matters.
What correct looks like: `shift = (u32::from(first_packet[43] & 0x03) << 3) | u32::from(first_packet[44] >> 5)` with `len() >= 45`, and a test fixture transcribed verbatim from a real libtheora header.
Confidence: HIGH on the offset analysis (spec layout hand-verified; fixture's internal impossibility is arithmetic, not opinion); UNVERIFIED by execution (corpus file absent from this sandbox — REQUEST-ORCH-RUN below).
Verification: read + forensics; REQUEST-ORCH-RUN: `ffmpeg -y -f lavfi -i testsrc=duration=4:size=360x640:rate=1/2 -c:v libtheora -b:v 200k /tmp/theora-probe.ogv && ffprobe -v 0 -show_entries stream=duration_ts,nb_frames -of csv /tmp/theora-probe.ogv && xxd -l 64 -s 0 /tmp/theora-probe.ogv` (byte 43-44 of the first packet will carry the KFG pattern).

### F-002 [S3]
Where: crates/demux/src/ogg/mod.rs:268-269 (comment), evidence: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:2007 and :2543
Code:
```rust
        // BE32@26, KFGSHIFT packed in the QUAL/KFG/PF tail — offsets
        // pinned empirically against libtheora-encoded fixtures and
        // the corpus .ogv (see the unit tests + the differential).
```
What is wrong: the claimed pinning authority never ran in any committed evidence. `corpus_oggs_match_the_ffmpeg_truth` (tests/corpus_ogg.rs:111-124) takes a loud-skip path when `/tmp/my-project/corpus-v4/media` is absent and `LUMEN_CORPUS` unset — and a skipped test still reports `... ok`. The baseline shows `test corpus_oggs_match_the_ffmpeg_truth ... ok` (nofailfast:2007) with zero proof the differential body executed, while its sibling proves the corpus tier was inactive: `test corpus_dirty_outcome_classes_hold ... ignored, randomized-corpus tier 2: set LUMEN_CORPUS to activate` (nofailfast:2543). The corpus dir does not exist in this sandbox (`ls /tmp/my-project/corpus-v4/media` → No such file or directory).
Why it matters: every "ffmpeg-measured truth" in corpus_ogg.rs (the four md5s, granules, packet counts — and via it the Theora offsets of F-001) is unexecuted in the committed record; comments through the Ogg module cite the differential as validation it did not perform. A skip that reports ok is indistinguishable from a pass in every gate output.
What correct looks like: a committed corpus fixture (small, deterministic) or an env-forced CI lane so the differential's pass/fail is real, plus a skip path that reports `ignored` rather than `ok`.
Confidence: HIGH — Verification: baseline logs cited; dir absence verified; skip path read (corpus_ogg.rs:120-127 returns early; cargo prints captured output only on failure).

### F-003 [S4]
Where: crates/demux/src/ogg/mod.rs:120-122
Code:
```rust
pub fn probe(header: &[u8]) -> bool {
    header.len() >= 4 && &header[0..4] == page::CAPTURE
}
```
What is wrong: dead in production — repo-wide grep finds no caller (`adts::probe` is reused via `es/aac.rs:21`; `ogg::probe` is only name-dropped in a media-probe doc comment, phase5.rs:96). The live sniff is a re-implementation: `OggSignature::probe` (`prefix[0..4] == b"OggS"`, phase5.rs:100-104).
Why it matters: two copies of the format gate that can drift (the doc comment's own "mirrors" wording admits the coupling); a future version-byte check added to one and not the other silently changes which files reach the demuxer.
What correct looks like: delete `ogg::probe` (its `adts` sibling earns its keep by reuse), or make the probe crate the single source.
Confidence: HIGH — Verification: repo-wide grep; phase5.rs read.

### F-004 [S4]
Where: crates/demux/src/ogg/mod.rs:92-95
Code:
```rust
/// Maximum header packets (codec setup) trusted into memory per
/// logical stream `[PROPOSAL: 256 KiB — the largest sane Vorbis setup
/// header is tens of KiB]`.
pub const MAX_HEADER_BYTES: u64 = 256 * 1024;
```
What is wrong: the doc's first line says "Maximum header packets … trusted into memory" for a constant that is (correctly, per its name and the enforcement at mod.rs:1274: `stream.header_bytes + data.len() as u64 > MAX_HEADER_BYTES`) a byte budget.
Why it matters: one word, but it is the kind that makes a reader reason about counts when the bound is bytes.
What correct looks like: "Maximum header bytes …".
Confidence: HIGH — Verification: usage at 1274 read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_PACKET_BYTES` | 85-90 | REAL | — | 256 KiB; enforced at 876/897 (part2, read) |
| `MAX_HEADER_BYTES` | 92-95 | REAL | S4 | F-004 doc wording |
| `MAX_RECOVERY_SCAN_BYTES` / `TAIL_PROBE_BYTES` / `MAX_SEEK_SCAN_PAGES` / `MAX_OPEN_PAGES` | 97-112 | REAL | — | budgets; TAIL_PROBE 128 KiB means EOS >128 KiB from end → duration unknown (documented "cheap probe" trade-off) |
| `EXTENSIONS` | 114-115 | REAL | — | ogg/oga/ogv/opus/spx — theora lane is extension-claimed despite F-001 |
| `probe` | 120-122 | REAL | S4 | correct but dead (F-003) |
| `OggCodec` enum | 124-146 | REAL | — | codec vocabulary incl. Skeleton |
| `OggCodec::descriptor` | 150-162 | REAL | — | grammar-checked static literals; opaque for Skeleton/Unknown |
| `OggCodec::is_header_packet` | 166-178 | REAL | — | magic sets verified per codec; Skeleton/Unknown false (Skeleton packets dropped later at 1009-1011, verified) |
| `Clock` enum | 183-193 | REAL | — | three clock shapes |
| `Clock::base` | 197-215 | REAL | — | literal-Ok claims verified (gcd(1,rate)=1; Theora num/den validated >0) |
| `Clock::nanos` | 221-235 | REAL | — | exact; negative granule → None; u128 intermediate; shift<63 validated ⇒ mask math safe |
| `Clock::granule_of_nanos` | 239-243 | REAL | — | exact inverse for sample clocks; Theora approximate (documented) |
| `identify` | 247-295 | REAL | S2 | F-001: Theora KFGSHIFT at bytes 40-41 vs spec 43-44; all other lanes verified |
| `be32` | 297-302 | REAL | — | guarded by len>=42 at call sites |
| `flac_streaminfo_rate` | 307-314 | REAL | — | 20-bit rate arithmetic verified; len-guarded |
| `LogicalStream` + `new` | 317-370 | REAL | — | plain state; no invariants violated in part1 |
| `StickyShape::{of,mint}` | 373-392 | REAL | — | same pattern as adts/wav; ogg actually wraps walk errors in fail() (1310, part3 range, read) |
| `OggDemuxer` struct | 423-453 | REAL | — | state doc on `next_discontinuity` matches part3 use (1059-1066) |
| `OggDemuxer::new` | 462-482 | REAL | — | source-capability refusal via SourceReader |
| `tracks` | 489-492 | REAL | — | link-0 snapshot contract documented; chained tracks surfaced here |

## 5. Test honesty (part1-relevant tests; the mod.rs test module itself is part3/A03's)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `theora_header_offsets_pin_the_corpus_layout` (mod.rs:1437-1461, part3 range) | NO for real-world correctness | the fixture is synthetic and self-consistent with the code's wrong offsets (F-001 forensics); it pins the implementation, not the format — a tautology relative to real .ogv bytes |
| `clock_arithmetic` (mod.rs:1388-1405) | YES | exact nanos↔granule round-trips pinned |
| `identify_parses_the_magic_set` (mod.rs:1407-1435) | YES | magic set + clock shapes pinned (no Theora entry — the Theora identify path is only covered by the tainted fixture test) |
| `flac_streaminfo_rate_bits` (mod.rs:1463-1469) | YES | 20-bit assembly pinned |
| `corpus_oggs_match_the_ffmpeg_truth` (tests/corpus_ogg.rs) | UNPROVEN | skipped in every committed run (F-002); its skip still reports ok |
| baseline doctest (mod.rs:406-422) | YES | ran ok (nofailfast:6362-6370) — constructs, starts, first packet |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "offsets pinned empirically against libtheora-encoded fixtures and the corpus .ogv (see the unit tests + the differential)" | mod.rs:268-269 | FALSE (as evidence) | the fixture is synthetic-and-impossible (F-001); the differential skipped in every committed run (F-002, nofailfast:2007/:2543) |
| "Opus is always 48 kHz per RFC 7845 §4.2" | mod.rs:36 | REAL | RFC 7845 §4.2 indeed defines the Opus granule at 48 kHz |
| "Theora frame rate + keyframe-granule shift" as the clock | mod.rs:37 | SHALLOW | the FRN/FRD half is right; the shift half reads the wrong bytes (F-001) |

## 7. Dead / duplicated / unreachable code
- `probe` (120-122): dead (F-003).
- `MAX_HEADER_BYTES` doc wording (F-004).
- The `Err(_) => return (OggCodec::Unknown, None)` arm in the Speex parse (282-285): documented-unreachable (len>=40 checked) — honest residue, not a defect.

## 8. Dependencies & flags
- Reuses `crate::avi::riff::{PayloadSlots, SourceReader}` and `crate::avi::header::DamageLog` — cross-format sharing inside demux (same as adts); no layering violation.
- No feature gates, no unsafe in this range; `mod tests` cfg-gated (82-83).

## 9. Open questions for the next agent
- A05/A03 (ogg parts 2-3): confirm no other consumer re-derives a Theora shift (if a decoder card ever parses theora headers itself, F-001's blast radius shrinks to demux-side timing only).
- Wave gate: run the REQUEST-ORCH-RUN in F-001 — one ffmpeg command settles the byte offset with a real libtheora header.
- A03 (part3, owns the test module): re-audit `theora_header_offsets_pin_the_corpus_layout` as a name/provenance finding in its own right.
