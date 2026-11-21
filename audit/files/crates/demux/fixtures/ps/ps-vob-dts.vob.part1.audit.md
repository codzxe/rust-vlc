# AUDIT: crates/demux/fixtures/ps/ps-vob-dts.vob (part 1 of 2 — bytes 0..82680)
| Field | Value |
|---|---|
| Auditor | W04-A03 |
| Date | 2026-11-21 |
| Size / chunk | 192 512 bytes total; part 1 = bytes 0..82 680 (the first 41 of 94 packs; boundary = after the 467th of 933 newline bytes, offset 82 679 — matches W04-A02's part-2 start of 82 680) |
| File verdict | REAL (asset verified byte-level against every pin reachable from part 1) |
| Reachable from shipped entry point? | The FORMAT is shipped-reachable (apps/cli/src/containers/mod.rs "mpegps" arm → ps::open); the fixture itself is committed test data consumed by crates/demux/tests/ps_fixtures.rs:127, crates/demux/src/ps/tests.rs:526, apps/cli/src/containers/ps.rs:908 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | python3 independent MPEG-PS byte walkers (this audit, READ-ONLY: start-code coverage, pack/SCR/PES/PTS extraction, seek-anchor arithmetic); od -A d -t x1 (pack@0, system header, nav lead, DTS body); sed -n on ps/mod.rs:440-500,1100-1215; seek.rs:386-394; containers/ps.rs:86-92,149-153,895-920,1013-1050; ps_fixtures.rs:120-150,171-260,296-345; ps/tests.rs:515-540; rg "ps-vob-dts" across crates/apps/docs (7 consumers) |

## 1. One-paragraph verdict
Part 1 (82 680 bytes = 41 of the 94 packs) is bit-level-verified, structurally
flawless DVD MPEG-PS: every byte is covered by a start-code structure (zero
orphans), packs are exactly 2048-byte DVD sectors (spacing set {2048}, stuffing
0, SCR monotone 0→5 840 at 146 ticks/pack), the single system header at 14..38
plus the PCI/DSI nav pair (980 B lead 0x00 / 1018 B lead 0x01, zero-filled)
pack sector 1 to exactly 2048 bytes, 39 video PES of uniform length 2028 with a
clean PTS/DTS ladder (+3 754 ticks = 24 fps; PTS 48 754→67 523, DTS = PTS−3 754)
and payload starting with the MPEG-2 sequence header `00 00 01 b3`, and the
single DTS PES @4110 rides substream 0x88 (`88 04 00 01` + `7f fe 80 01` sync)
with PTS 48 754 — all matching the lane pins. The two real problems live around
the bytes, not in them: (F-001, S2) the fixture's SCR clock is legally but
wildly divergent from its PTS timeline, and the demuxer's seek reanchor maps
media time directly onto pack SCRs — a seek to 0.54 s on this file anchors at
byte 118 784 where the content PTS is 1.29 s — and no test pins seek on this
lane; (F-002, S4) every part-1 structural field except PES counts/payload
md5/PTS is unpinned; (F-003, S4) docs/formats/audio-dts.md:76 states "94 PES",
a number that is not any PES count of this file.

## 2. What actually works (verified)
- Independent byte-level walker (python3, this audit): part 1 = 41 pack headers
  (0xBA) + 1 system header (0xBB, 14..38) + 39 video PES (0xE0) + 1 private-1
  PES (0xBD) + 2 nav PES (0xBF); start-code coverage 82 680/82 680 bytes —
  zero orphan bytes (the 39th video PES @81 934 straddles the chunk boundary
  and ends at 83 968 in part 2 — chunk artifact, not damage).
- Whole-file cross-check (same walker): 94 packs + 2 system headers + 68 E0 +
  24 BD(substream 0x88 only) + 4 BF + 3 padding streams (0xBE @119 017/159 567/
  192 283, all in part 2) = pinned counts [4, 68, 24] EXACT; video payload
  135 397 B md5 `130873a40a8090129373005692be7269` and DTS payload stripped of
  the 4-byte DVD prefix 48 128 B md5 `de45412596ea9fc909b4553055e420b0` — both
  equal the committed digests at ps_fixtures.rs:136-139.
- Pack@0 bytes `00 00 01 ba 44 00 04 00 04 01 01 89 c3 f8` (od-verified): SCR
  base 0/ext 0; program_mux_rate = 25 200 (22-bit field, ×50 = 1 260 000 B/s =
  the DVD-standard 10.08 Mbit/s) — EQUAL to the system header's rate_bound
  (25 200, bytes `80 c4 e1`), i.e. the two declared rates are consistent; the
  demuxer's own parse comment pins this exact byte pattern
  (ps/mod.rs:443-445 "pinned against ffmpeg's `44 00 04 00 04 01` zero-SCR
  packs").
- System header @14..38 (od-verified `00 00 01 bb 00 12 80 c4 e1 04 e1 7f b9 e0
  e6 b8 c0 20 bd c0 00 bf e0 02`): rate_bound 25 200, audio_bound 1,
  video_bound 1, both lock flags set, four stream entries — lumen skips system
  headers positionally (ps/mod.rs:661-668, 1189) so the degenerate entry table
  (entries for 0xB9/0xB8, none for the actual video stream 0xE0 — ffmpeg's
  canonical DVD header) has zero consumer impact.
- Nav pair: @38 PCI, PES len 980, payload 980 zero bytes, lead `00`; @1024 DSI,
  PES len 1018, lead byte 0x01 then zeros (od-verified `00 00 01 bf 03 fa 01
  00`); 14+24+986+1024 = 2048 — the nav pair packs sector 1 exactly. Same shape
  the pinned nav test asserts on the ac3 sibling.
- Video PES: uniform PES length 2028; first PES @2062 flags 0xC1 (PTS+DTS+PES
  extension: PSTD scale 1, size 230×1024 B), PTS 48 754 / DTS 45 000; stamped
  ladder +3 754 (24 fps); unstamped PES carry hdrlen 1 with one 0xFF stuffing
  byte (`80 00 01 ff`, od-class verified at @6158); payload head
  `00 00 01 b3 2d 01 e0 21` — genuine MPEG-2 video sequence header.
- DTS PES @4110: flags 0x81, PTS 48 754 (= pin first_pts[2] Some(48_754)),
  body `88 04 00 01 7f fe 80 01` (od-verified) — substream 0x88 (documented
  DTS range 0x88..=0x8F), 3 filler bytes, then the 16-bit-mode DTS sync.
- Baseline execution: ps_fixtures 4/4 ok (orchrun-test-workspace-nofailfast.log.txt:2176-2182),
  ps::tests incl. fuzz_lite_clean_fixtures_drain (consumes this file) ok (:1835-1843),
  containers::ps::tests 4/4 ok (:2830-2833).

## 3. Findings

### F-001 [S2] Fixture's divergent SCR timeline + reanchor's SCR↔PTS domain conflation: every seek on this lane lands ~0.7 s late, and no test pins it
Where: crates/demux/fixtures/ps/ps-vob-dts.vob part 1 (pack SCR ladder) vs
crates/demux/src/ps/mod.rs:1180-1187 (consumer); call chain
crates/playback-engine/src/seek.rs:390 → pipeline.rs:1498-1499 → ps reanchor.
Code (the consumer that maps media time onto pack SCRs — ps/mod.rs:1180-1187):
```rust
            match code[3] {
                0xBA => {
                    let pack = self.parse_pack_at(pos)?;
                    if pack.scr_base <= want {
                        best = Some((pack.scr_base, pack.end));
                        pos = pack.end;
                    } else {
                        break;
                    }
                }
```
What is wrong: The fixture is legal ffmpeg output whose pack SCR clock runs
divergently from its PTS timeline: part 1's 41 packs span SCR 0→5 840 (146
ticks/pack) while the same bytes carry PTS 45 000→67 523; the pacing then jumps
to 3 840 ticks/pack (SCR 131 315 at EOF vs last PTS 136 114). The reanchor
treats a 90 kHz media-timestamp target as an SCR-domain value and anchors at
the LAST pack with scr_base ≤ want. Exact arithmetic on this file (independent
walker): seek(48 754 ticks = 0.54 s, the file's FIRST PTS) anchors at byte
118 784 (SCR 46 753) where the next stamped PES has PTS 116 322 (1.29 s);
seek(54 000 = 0.60 s) anchors at byte 139 264 → PTS 123 829 (1.38 s). The
returned landing stamp is the SCR value (mod.rs:1213-1214
`stamp_of(Some(scr as i64))`), which seek.rs:442 feeds to the media clock —
position AND reported time are both wrong (0.52 s reported, 1.29 s of content
resumed). Only 3 of this file's 94 packs have SCR in part 1's PTS range, so
part-1 content is effectively unseekable-to.
Why it matters: mpegps is a shipped container (containers/mod.rs "mpegps" arm),
SeekSupport::SlowLinear is forwarded to users, and every ffmpeg-authored VOB
muxed at the default 10.08 Mbit/s with modest content bitrate carries this
SCR/PTS divergence — a user seek in the shipped CLI silently drops ~0.7 s of
content and desyncs the clock. The only seek-flavored test on this family runs
on the .mpg lane (containers/ps.rs:1013 `start_time_skips_packets_with_slow_
linear_seekability`, ps-mp2-video-audio.mpg) where SCR tracks PTS closely
enough to pass; no test seeks on any VOB lane, and open-with-start-time does
NOT use reanchor (containers/ps.rs:88-89 "No reanchor walk runs at open" — the
PTS watermark does the positioning), so the defect is invisible to the suite.
What correct looks like: reanchor must map the target through the PTS/DTS
domain (scan PES stamps, or maintain a stamp→offset index built during
start()'s scan), not assume scr_base ≈ media time; plus one pinned seek test on
this fixture (e.g. seek to 0.54 s must land at or before the first PTS ≥
target, asserting the first post-seek stamp).
Confidence: HIGH — Verification: read (reanchor loop traced line-by-line;
anchor arithmetic recomputed by my independent walker on the committed bytes;
call chain verified by rg reanchor → seek.rs:390/pipeline.rs:1498); NOT
executed (cargo forbidden — the .mpg lane's passing watermark test in baseline
:2831 is the closest executed evidence).

### F-002 [S4] Part-1 structural fields are entirely unpinned: SCR ladder, mux rate, system header, PSTD fields, and this file's nav shape
Where: crates/demux/tests/ps_fixtures.rs:126-146 (the whole pin set for this
file) and :319 (nav shape pinned on a sibling file only)
Code (ps_fixtures.rs:133-134 — the extent of per-lane structural pins):
```rust
        track_count: 3,
        ours: [4, 68, 24],
```
Code (ps_fixtures.rs:319 — the nav-shape test runs on the ac3 sibling, not this file):
```rust
    let (_, packets) = run_lane(&LANES[1]);
```
What is wrong: The lane pins PES counts, payload md5s, byte totals and PTS
extents — all verified and exact. Nothing pins: any of the 41 part-1 SCR values
(0→5 840, 146/pack), the pack mux_rate (25 200) or system-header rate_bound,
the system-header entry table, the PSTD buffer fields on the first video/audio
PES, or the nav PCI/DSI shape ON THIS FILE — the 980/1018 + 0x00/0x01 +
zero-fill assertions at ps_fixtures.rs:315-339 run on LANES[1] (ps-vob-ac3.vob)
only. This file's nav packets are pinned by count (ours[0]=4) alone; I verified
the shape by hand (@38: 980 B all-zero; @1024: 1018 B, lead 0x01, rest zero —
od-verified), but a regression that mangled only THIS file's nav payloads
would pass every committed test.
Why it matters: These fixtures are the repo's only MPEG-PS reference set (no
corpus lane exists, no in-tree generator); the pins are the reproducibility
anchor. Together with W04-A02's part-2 F-001 (no file-level hash), the
container plane (SCR/system header/mux rate) and this file's nav payload are
regression-invisible.
What correct looks like: Extend the Lane table (or a sibling test) with a
handful of part-1 structural pins: first/last SCR, pack count 94, mux_rate
25 200, nav sizes/leads for LANES[2], or simply route the existing nav test
over all VOB lanes.
Confidence: HIGH — Verification: read (full ps_fixtures.rs pin table + test
bodies); rg "ps-vob-dts" — all 7 consumers audited, none pins these fields.

### F-003 [S4] docs/formats/audio-dts.md:76 claims "94 PES" — a number that is no PES count of this file
Where: docs/formats/audio-dts.md:76 vs the fixture bytes
Code (docs/formats/audio-dts.md:75-77):
```markdown
- **PS (5.I2):** private-stream-1 `0x88..=0x8F` → DTS routing with the
  substream+filler strip, payload bit-exact vs `ffmpeg -c copy` —
  fixture `ps-vob-dts.vob` (dvd_nav + mpeg2video + dts; 94 PES) pinned
```
What is wrong: The file contains 4 nav + 68 video + 24 DTS PES = 96 PES (99
counting the three 0xBE padding streams). "94" is the PACK count (my walk: 94
packs) and coincidentally ffprobe's DTS-AU count (the lane's ffprobe column
[0, 25, 94], ps_fixtures.rs:134-135), but the sentence says "94 PES".
Why it matters: The docs' differential tables are the provenance story for
these fixtures; a reader reconciling the doc against the pinned "ours: 4/68/24"
or against ffprobe output meets a number that matches neither PES truth.
What correct looks like: "96 PES (4 nav + 68 video + 24 dts; 94 packs; ffprobe
counts 94 DTS AUs)" — one clause, no ambiguity.
Confidence: HIGH — Verification: read + independent count (96 PES / 94 packs /
3 padding streams, zero orphans, sums to the pinned 4+68+24).

## 4. Per-element table (structural elements; part 1 = bytes 0..82 680)
| Element | Count in part 1 | Whole file | Notes |
|---|---|---|---|
| Pack headers (0xBA) | 41 (offsets 0..81 920, spacing exactly 2048, stuffing 0, SCR 0→5 840, monotone) | 94 | DVD sector-perfect; SCR pacing jumps to 3 840/pack after ~byte 120 k (F-001). |
| System header (0xBB) | 1 (@14..38) | 2 (second @120 830-ish region, part 2) | rate_bound 25 200 = pack mux_rate; degenerate stream-entry table (0xB9/0xB8/0xBD/0xBF, no 0xE0); lumen skips positionally (mod.rs:661). |
| Nav PES (0xBF) | 2 (@38 PCI 980 B zero, lead 0x00; @1024 DSI 1018 B, lead 0x01) | 4 | PCI/DSI pair fills sector 1 exactly (2048 B); shape on THIS file unpinned (F-002). |
| Video PES (0xE0) | 39 (uniform len 2028; 6 stamped: PTS 48 754→67 523, DTS = PTS−3 754) | 68 (24 stamped, last PTS 135 090) | First PES flags 0xC1 with PSTD (scale 1, size 230×1024); unstamped PES = flags 0x00, hdrlen 1, 0xFF byte; payload head `00 00 01 b3` (sequence header). The 39th (@81 934) straddles the chunk boundary. |
| Private-1 PES (0xBD) | 1 (@4110, substream 0x88, PTS 48 754, body `88 04 00 01 7f fe 80 01`) | 24 (all substream 0x88, last PTS 136 114) | Audio back-loaded 1:23 across the boundary — a timing-sensitivity observation, not damage. |
| Padding streams (0xBE) | 0 | 3 (all part 2; last @192 283 ends exactly at EOF) | See §9 for the part-2 correction. |
| Program end (0xB9) | 0 | 0 | W04-A02 part-2 F-002 covers the omission. |

## 5. Test honesty (tests consuming this fixture)
| Test | Would it fail if the fixture (or parser) were broken? | Reason |
|---|---|---|
| fixture_stream_tables_and_packet_counts_match_the_pins (ps_fixtures.rs:222) | YES | Counts [4,68,24], kinds, descriptors, first/last PTS per track asserted — my independent counts/stamps match exactly. |
| fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit (:254) | YES | md5 of per-track concatenated payload asserted — my recomputed md5s are byte-identical to both pinned digests. |
| vob_nav_packets_are_real_and_zero_padded_by_ffmpeg (:315) | NO (not this file) | Runs on LANES[1] (ac3). This file's nav shape verified only by me (F-002). |
| ps::tests::fuzz_lite_clean_fixtures_drain_without_terminating_errors (ps/tests.rs:523) | YES (survival) | fuzz_demux_ps + open/start/drain over these bytes; terminal=None asserted; the 3 part-2 padding streams surface as Recovered events, tolerated. |
| containers::ps::tests::vob_lanes_demux_and_refuse_undecodable_codecs_typed (containers/ps.rs:905) | YES | 3 tracks, 24 dts packets, last PTS 136 114 through the real CLI open path — matches my parse. |
| Any seek test on this file | NONE EXISTS | F-001 — the only seek-flavored test runs on the .mpg lane. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "ps-vob-dts.vob \| nav+mpeg2video+dts, 0+25+94 \| 4nav+68v+24dts \| dts 48 128 B md5 bit-exact" | docs/formats/container-mpegps.md:52 | REAL | My parse: 4/68/24 PES; stripped DTS payload 48 128 B, md5 equals the pin; ffprobe column documentary (no ffprobe in sandbox — UNVERIFIED, consistent with 48 128 ÷ 512 B/AU ≈ 94). |
| "fixture ps-vob-dts.vob (dvd_nav + mpeg2video + dts; 94 PES) pinned" | docs/formats/audio-dts.md:76 | FALSE (the "94 PES" number) | 96 PES in the file (4+68+24); 94 = pack count / ffprobe DTS-AU count (F-003). |
| "private-stream-1 0x88..=0x8F → DTS routing with the substream+filler strip" | audio-dts.md:73-74 | REAL | Sole substream 0x88; body `88 04 00 01` + sync `7f fe 80 01` (od-verified); strip-4 arithmetic reproduces the pinned md5. |
| "we expose 4 … Data packets (sizes 980/1018 alternating, measured)" | docs/worklog/archive-2026-10.md:9166 | REAL | 980/1018 verified at 38/1024 (part 1) and 120 870/121 856 (part 2). |
| "unstamped MPEG-1 PES single 0x0F byte accepted (spec's 0xFF …)" | container-mpegps.md:57-58 | REAL (and this file exercises the 0xFF form) | Part-1 unstamped MPEG-2 PES carry hdrlen 1 + 0xFF byte (`80 00 01 ff` @6158). |

## 7. Dead / duplicated / unreachable code
- Binary asset: no code. No orphan bytes in part 1 (0/82 680 uncovered —
  verified).
- The fixture's video payload md5 is shared byte-identically with the ac3 and
  lpcm siblings (same video, three audio variants) — intentional differential
  design, not duplication.
- Degenerate-but-authentic structures carried and ignored: system-header entry
  table (no 0xE0 entry), nav payloads semantically empty (ffmpeg zero-fill —
  named honestly by the pinned test), PSTD fields parsed by nobody. All are
  authentic ffmpeg `dvd`-muxer output; their being unpinned is F-002.

## 8. Dependencies & flags
- Committed binary asset (192 512 B, ffmpeg 7.1.5 per docs; no in-tree
  generator, no hash manifest — see W04-A02 part-2 F-001, agreed). Consumed by
  integration tests (ps_fixtures.rs), in-crate tests (ps/tests.rs), CLI
  open-path tests (containers/ps.rs). Not covered by `cargo xtask fixtures
  --check` (baseline #12 PASS covers only the generated AVI/MP4 corpus).

## 9. Open questions for the next agent
- F-001's code-side fix belongs to the ps/mod.rs owner (W05-A06/A07 — reanchor
  at 1165-1205 sits in part 2): the SCR↔PTS conflation is a likely S2 there
  for EVERY ffmpeg-authored VOB, not just this fixture. The seek() pure query
  (mod.rs:1128 `let last = self.last_scr.unwrap_or(0);` … `.min(last)`) also
  clamps to the SCR extent (131 315) below the real last PTS (136 114).
- Cross-agent correction for W04-A02's part-2 audit (same file): (a) its
  per-element table's part-2 counts (48 packs / 25 video / 22 dts) do not sum
  to its own whole-file totals; the exact split is part 1 = 41/39/1, part 2 =
  53/29/23 (94/68/24 total — my walk, sums verified); (b) its F-002 says
  "last structure = the final DTS PES, then EOF" — the last structure is a
  0xBE padding stream @192 283 (len 223, ends exactly at 192 512); the B9
  omission conclusion stands, but part 2 also contains the three padding
  streams its table omits entirely (lumen's walk notes them as
  "padding-stream" damage, mod.rs:1041-1045).
- A nav-shape pin for THIS file (F-002) should be routed through the wave-16
  owner of ps_fixtures.rs (16-A06).
