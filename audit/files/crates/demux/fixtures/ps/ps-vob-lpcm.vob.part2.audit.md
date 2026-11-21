# AUDIT: crates/demux/fixtures/ps/ps-vob-lpcm.vob#part2 (chunk: bytes 89,927–168,240 of 241,664 — "lines" 567–1132 of 2,264; 78,314 bytes)
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-21 |
| Kind | binary fixture chunk (MPEG-PS / DVD-VOB program stream, ffmpeg 7.1.5-authored per docs) |
| File verdict (chunk) | REAL |
| Reachable from shipped entry point? | YES — the fixture feeds tests of the shipped PS lane: `PsDemuxer` via `open_via_input` (crates/demux/tests/ps_fixtures.rs:191–215), the CLI VOB lane `apps/cli/src/containers/ps.rs:903–911`, and fuzz-lite `crates/demux/src/ps/tests.rs:522–536`. PS itself is shipped-reachable (PsSignature/containers/ps.rs wiring, verified in my riff/asf audits). Baseline execution: ps::tests 26/26 (orchrun :1835–1859), ps_fixtures 4/4 (:2176–2182), containers::ps 4/4 (:2830–2833) — all `ok` inside the true-totals run 4534/7/49 whose 7 failures (log footer) touch no PS test. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 ISO-13818-1 start-code walk of the whole file (strict, byte-contiguous 0→241,664, 243 items: 118 packs + 2 system headers + 123 PES) and of the part2 slice; PTS bit-extraction; SCR/mux-rate decode; md5 of demuxed payloads; `ffprobe -show_entries stream` → `pcm_dvd sample_rate=48000 channels=1 bits=16` + stream counts 0/25/48 (ffmpeg 7.1.5-0+deb13u1 — same version family as the authoring build); `ffmpeg -map 0:v -c copy` → 135,397 B md5 `130873a4…` (bit-equal to our extraction); `ffmpeg -map 0:a -c copy` → 96,144 B; ffprobe AU-size table → ours == ff sizes − 3 per packet, byte-equal after header strip; Grep for `4366d145275b69f5806fd226456725fe|383ea763bddf28a41c932954c8e78823` → no files found (no hash pin); sed -n on every quoted source range. |

## 1. One-paragraph verdict
The chunk is the middle of a genuine ffmpeg-muxed DVD-VOB program stream, and
every committed expectation that depends on these bytes reproduces exactly
under my independent byte-level parse and a live ffmpeg 7.1.5 differential.
The chunk carries: 39 MPEG-2 pack headers (SCR strictly monotonic 6,424→69,622
ticks, single mux rate 32,836, zero stuffing), 27 video PES (17 stamped
PTS+DTS, 9 unstamped), 12 LPCM PES including the first of the file's four
unstamped audio AUs (@159,758), the file's only mid-stream DVD cell boundary
(pack@124,928 + system header@124,942 + zero-filled PCI nav@124,966 + DSI
nav@125,952 — nav cluster #2 of 2), and the mid-file 0xFF padding packet
(@119,017, 1,809 B). The whole-file video payload (135,397 B) and audio
payload (96,000 B after the 7-byte DVD prefix strip) that this chunk feeds are
md5-identical to the committed pins AND to a fresh `ffmpeg -c copy` video
extraction; the audio is byte-equal to ffmpeg's 96,144-B stream minus its 48
3-byte AU headers. The single finding is a test-protection gap: every
structural byte outside the hashed payloads (pack SCR/mux-rate, system
headers, padding, the LPCM geometry prefix) is pinned by nothing — a targeted
mutation there survives the entire suite. This complements A08's whole-file
S4 (no committed hash pin, fixtures gate covers only tests/fixtures), which I
independently confirmed.

## 2. What actually works (verified)
— Strict start-code walk, whole file: byte-contiguous 0→241,664 with zero
misalignment (243 items = 118 packs + 2 system headers + 4 nav + 68 video +
48 LPCM + 3 padding). The docs' "4nav+68v+48lpcm"
(docs/formats/container-mpegps.md:53) is exactly what the bytes contain.
— Payload pins: my extraction (PES-header strip; audio additionally minus the
7-byte prefix `a0 07 00 04 0c 00 80`) gives video 135,397 B md5
`130873a40a8090129373005692be7269` and audio 96,000 B md5
`d7754037c4ff56d806ada9dc67877c59` — identical to the lane table pins
(crates/demux/tests/ps_fixtures.rs:159–165).
— Live ffmpeg 7.1.5 differential (the fixture's authoring claim: "ffmpeg
7.1.5 / Lavf61.7.103, the same build that authored the fixtures",
ps_fixtures.rs:7–8; container-mpegps.md:25 "Corpus evidence (5.I2, ffmpeg
7.1.5, committed fixtures)"): `ffprobe` streams = dvd_nav_packet +
mpeg2video + pcm_dvd, packet counts 0 + 25 + 48 (== pin `ffprobe: [0, 25,
48]`, ps_fixtures.rs:156); `ffmpeg -map 0:v -c copy` = 135,397 B, md5 BIT-EQUAL to
ours; `ffmpeg -map 0:a -c copy` = 96,144 B and, with the 48 3-byte pcm_dvd AU
headers removed per ffprobe's AU sizes, BYTE-EQUAL to our 96,000 B (per
packet: ours[i] == ff_size[i] − 3, all 48). The doc's "+3 B × 48 AU headers —
exact delta pinned" (container-mpegps.md:53, ps_fixtures.rs:163–165) is true
by measurement, not just arithmetic.
— Audio geometry (unpinned by the repo, checked anyway): ffprobe reads the
prefix as 48,000 Hz / 1 channel / 16-bit → 96,000 B = exactly 1.0 s, matching
the stamped span 48,754→138,583 ticks ≈ 0.998 s + final AU. Internally
consistent.
— My chunk's special content verified byte-level: system header #2 @124,942
(24 B, length-skipped by the demuxer per mod.rs:73); PCI @124,966 (980 B,
lead 0x00, rest all-zero) and DSI @125,952 (1,018 B, lead 0x01, rest
all-zero) — satisfying `vob_nav_packets_are_real_and_zero_padded_by_ffmpeg`
(ps_fixtures.rs:315–337) for packets 2 and 3; padding @119,017 payload
entirely 0xFF.
— PTS integrity in-chunk: LPCM stamps 52,902→73,800 and video stamps
71,277→135,090 all strictly increasing; the unstamped LPCM PES @159,758
corresponds to ffprobe's interpolated AU (pts 69,416 = 67,530 + 1,886; AU
size 2,019 = our 2,016 + 3) — real muxer behavior, honestly recorded by the
repo as the unstamped-AU divergence (container-mpegps.md:67–69).
— Baseline execution of every consumer test (see header table): the pins I
byte-verified are enforced, not aspirational.

## 3. Findings

### F-001 [S4]
Where: crates/demux/fixtures/ps/ps-vob-lpcm.vob, bytes 89,927–168,240 (my chunk) — pack headers @90,112…167,936; LPCM prefixes of the 12 in-chunk audio PES; padding payload @119,017+; system header @124,942
Code:
```
(pack header at offset 90,112: 00 00 01 ba 44 00 04 c8 c4 01 01 89 c3 f8 — SCR base 6,424, mux rate 32,836, stuffing 0; none of these bytes is covered by any test)
```
What is wrong: The in-repo protection for this conformance-truth binary
covers only payload bytes (video/audio md5), packet counts, PTS values, and
nav content. Four byte classes in this chunk alone are pinned by nothing:
(a) pack-header SCR/mux-rate/stuffing of all 39 in-chunk packs — the LPCM
lane has no seek or extent test (the only PS seek test uses the MPG fixture,
apps/cli/src/containers/ps.rs:1013–1014; `run_lane` asserts only
`seekability() == SlowLinear`, ps_fixtures.rs:194), so the SCR map that
backs SlowLinear seek is entirely unverified on this file; (b) the 7-byte
LPCM prefix — the demuxer reads byte 0 for routing and strips the rest
(`PrivateKind::Lpcm => 7`, ps/mod.rs:160), so the geometry byte 0x0C
(48 kHz/mono/16-bit per ffprobe) can be flipped to anything with zero test
impact; (c) the 0xFF padding payload @119,017 (skipped positionally,
unhashed); (d) system-header bytes @124,942 (length-skipped, mod.rs:73).
Concrete mutations that survive the full suite: SCR byte flip at any of the
39 packs; `0c`→`0d` at prefix offset 4 of any in-chunk LPCM packet;
0xFF→0x00 anywhere in the padding payload.
Why it matters: The fixture is the reference truth for the whole PS demuxer
(the docs admit no .ps/.vob corpus lane exists, container-mpegps.md:88–90).
Corruption or a subtly different re-mux that touches only these classes
would silently degrade seek anchoring and future LPCM-geometry exposure (the
`[PROPOSAL …]` card at ps/mod.rs:78–80 will consume exactly these unpinned
bytes) while every gate stays green.
What correct looks like: extend the lane table (or the fixtures gate, which
today covers only `tests/fixtures` — 55 files, gate-fixtures___check.log.txt)
with a whole-file sha256 manifest row for `crates/demux/fixtures/**`; that
closes this finding and A08's part4 F-001 (same root: I confirmed by Grep
that neither `4366d145275b69f5806fd226456725fe…` nor the md5 appears
anywhere in the repo).
Confidence: HIGH — Verification: read (test bodies, demuxer strip code) + ran (my own byte-level parse proving the digest covers payloads only; ffprobe/ffmpeg live reproduction). Cross-ref: A08 part4 F-001 [S4] (no hash pin) — this finding is the in-repo-protection instance of the same gap, not a duplicate.

## 4. Per-item table (structural elements of the chunk; binary file — no functions)
| Item | Bytes (offset) | Verdict | Notes |
|---|---|---|---|
| MPEG-2 pack headers ×39 | 90,112…167,936 (every 2,048-aligned sector start) | REAL | 14 B each, stuffing=0; SCR base 6,424→69,622 strictly monotonic (whole-file check passed); mux rate constant 32,836; all fields unprotected by tests (F-001) |
| System header #2 (0xBB) | 124,942 (24 B) | REAL | The file's only mid-stream system header; starts the second DVD cell; demuxer length-skips (mod.rs:73); unprotected (F-001) |
| PCI nav PES (0xBF) | 124,966 (plen 980) | REAL | Lead 0x00, rest zero-filled — asserted by vob_nav test (ps_fixtures.rs:330–337); protected |
| DSI nav PES (0xBF) | 125,952 (plen 1,018) | REAL | Lead 0x01, rest zero-filled — asserted; protected |
| Video PES (0xE0) ×27 | 88,078 (straddles LO) … 161,806 | REAL | 17 with PTS+DTS (hdrlen 11), 9 unstamped (hdrlen 1); payloads feed the 135,397-B md5 pin — bit-exact vs ffmpeg live |
| LPCM PES (0xBD sub 0xA0) ×12 | 120,846…167,950 (last straddles HI) | REAL | Prefix uniform `a0 07 00 04 0c 00 80`; 11 stamped; @159,758 is unstamped (flags 0x00, hdrlen 2) = ffprobe's interpolated AU 69,416; payloads feed the 96,000-B md5 pin — byte-equal to ffmpeg minus AU headers |
| Padding PES (0xBE) ×1 | 119,017 (plen 1,809) | REAL | Payload all 0xFF (verified); unprotected (F-001) |
| Chunk boundaries | LO 89,927 inside video PES @88,078; HI 168,241 inside LPCM PES @167,950 | n/a | Line-based split of a binary cuts mid-packet by design (same as A08's part4 boundary); no structural meaning — packets were audited whole and attributed by start offset |

## 5. Test honesty (tests consuming this fixture)
| Test | Would it fail if the fixture were broken? | Reason |
|---|---|---|
| fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit (ps_fixtures.rs:254) | YES — strongest | Any byte change in any video/audio payload changes the md5 of the concatenation; the exact ffmpeg size delta (96,144 − 96,000 == 3 × 48) is also asserted |
| fixture_stream_tables_and_packet_counts_match_the_pins (:222) | YES | Track count/kinds/descriptors, per-track PES counts [4,68,48], first/last PTS per track are all asserted against measured pins |
| lpcm_packets_strip_the_full_dvd_prefix (:296) | YES (structural) | Asserts 48 packets, first 2,008 B, final 1,424 B, total 96,000 B — any re-mux changing packetization fails |
| vob_nav_packets_are_real_and_zero_padded_by_ffmpeg (:315) | YES | Sizes 980/1018 alternating, lead bytes 0x00/0x01, all-zero rest — my chunk's two nav packets are covered by indices 2,3 |
| vob_lanes_demux_and_refuse_undecodable_codecs_typed (containers/ps.rs:903) | YES | container=="mpegps", 3 tracks, pcm_dvd descriptor, 48 audio PES, last PTS 138,583, typed decode refusal |
| fuzz_lite_clean_fixtures_drain_without_terminating_errors (ps/tests.rs:522) | YES (panic-class) | Absence-of-panic + `terminal == None` on the whole file |
| — counterexample | NO | F-001's mutation classes (SCR bytes, LPCM prefix bytes 1–6, padding payload, system header) change none of the asserted values — the suite stays green. Reasoned through, not executed (read the assertions + the demuxer's strip path, ps/mod.rs:157–161) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "4nav+68v+48lpcm" packet truth | docs/formats/container-mpegps.md:53 | REAL | My walk: 4 × 0xBF, 68 × 0xE0, 48 × 0xBD sub 0xA0 |
| "ours 96 000 B pure samples; ffmpeg 96 144 = +3 B × 48 pcm_dvd AU headers — exact delta pinned" | container-mpegps.md:53; ps_fixtures.rs:163–165 | REAL | My strip sum = 96,000; live `ffmpeg -c copy` = 96,144; ffprobe AU sizes − 3 == our per-packet sizes, byte-equal stream |
| video payload "the *byte streams* … do bit-for-bit on video / MP2 / AC-3 / DTS" | ps_fixtures.rs:17–19; lane digest pin :159–160; worklog archive-2026-10.md:9166 | REAL | Live: our extraction and `ffmpeg -c copy` both 135,397 B, md5 `130873a40a8090129373005692be7269` |
| ffprobe "0+25+48" packets | container-mpegps.md:53; ps_fixtures.rs:156 | REAL | Live ffprobe -show_packets: stream0=0, stream1=25, stream2=48 |
| Nav "4 zero/0x01-lead Data packets (980/1018 B alternating)" | container-mpegps.md:64–66; worklog :9166 | REAL | Byte-verified all 4, incl. my chunk's PCI/DSI pair |
| "the DVD LPCM header bytes are skipped, not re-exposed as channel/rate facts" | crates/demux/src/ps/mod.rs:78–80 | REAL | `PrivateKind::Lpcm => 7` strip-only; ffprobe confirms the bytes carry 48 kHz/1 ch/16-bit that we drop |
| ffprobe interpolates unstamped AUs, we don't (recorded divergence) | container-mpegps.md:67–69 | REAL | In-chunk instance: unstamped PES @159,758 ↔ ffprobe AU pts 69,416 interpolated; our lane pins only real stamps (48,754/138,583) |

## 7. Dead / duplicated / unreachable code
n/a (binary data). Note: the lane table's nav `bytes: [0, …]` slot
(ps_fixtures.rs:162) is dead — nav packets actually carry 3,996 payload bytes
total but the slot is never asserted (digest `""` short-circuits the check).
Owner of that file (W16-A06) should decide whether to pin or delete the slot.

## 8. Dependencies & flags
The fixture is not covered by `cargo xtask fixtures --check` (gate verified
"55 file(s) under tests/fixtures" only — gate-fixtures___check.log.txt; the
xtask fixtures writer has zero references to vob/mpegps — confirmed by A08
and consistent with my read of tools/xtask/src/cmd/fixtures.rs scope). No
feature flags gate its consumers; ps_fixtures.rs runs in the default
workspace test set (baseline :2176–2182).

## 9. Open questions for the next agent
- W16-A06 (ps_fixtures.rs owner): the dead nav `bytes`/`digest` slots (§7);
  consider adding the whole-file sha256 manifest row (F-001's fix lives best
  in your file or the xtask gate).
- W05-A06/A07 (ps/mod.rs owners): the `[PROPOSAL: the 6-byte header carries
  the audio facts]` card will read the currently-unpinned prefix bytes; my
  live ffprobe measurement (48 kHz/mono/16-bit, AU = 1,006 samples = 1,886
  ticks, 4 of 48 AUs unstamped at offsets 159,758 / 184,334 / 204,814 /
  227,342) is the ground truth to test against when that lands.
- W13-A01 (containers/ps.rs owner): the only PS seek test uses the MPG
  fixture (:1013–1014); a VOB-lane seek/extent assert would close the SCR
  protection gap for all four fixtures.
