# AUDIT: crates/demux/fixtures/ps/ps-vob-ac3.vob (binary fixture asset, 157,696 bytes)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-11-21 |
| Lines / functions | 157,696 bytes (binary; no functions) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (fixture bytes flow through the wired PS lane: `lumen` → containers/mod.rs:71 `"mpegps" => ps::open` → PsDemuxer; executed in 4 suites in baseline) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `od -A d -t x1z` (header bytes); `stat`, `sha256sum`; python3 start-code census (`00 00 01 BA/BB/BF/E0/BD/C0` counts) — **independent byte verification of every pinned census number**; Grep tool for `ps-vob-ac3` across the repo (12 files); baseline log greps (ps_fixtures 4/4, ps::tests fuzz-lite 2/2, containers::ps, mpeg12_corpus 3/3) |

## 1. One-paragraph verdict
This fixture is the reference asset for the DVD-VOB/AC-3 lane of the PS
demuxer, and everything claimed about it checks out against my own
independent byte-level measurement: the file is a real ffmpeg-authored,
DVD-sector-aligned (157,696 = 77 × 2048) MPEG-PS stream whose packet census
I counted myself — 77 pack headers, 2 system headers, 4 nav packs
(private-2, lengths 0x03d4=980 and 0x03fa=1018, the exact "980/1018
alternating" the record doc pins), 68 mpeg2video PES, 7 private-1 AC-3 PES
(6×2028 + 1×209 bytes) — matching the test lane's pinned `ours: [4, 68, 7]`
and the record doc's "4nav+68v+7ac3" digit for digit. It is referenced by
four test suites (all passed in the baseline) and three record docs, has no
regeneration path that could drift (committed bytes only, unlike the
asf/wav external-corpus patterns), and the differential pins include the
honest divergence from ffprobe (0+25+32 — ffmpeg re-packetizes to AUs and
consumes nav packets) rather than smoothing it over. No defects found.

## 2. What actually works (verified)
- **Independent census** (python3 over the committed bytes): pack(0xBA)=77,
  system(0xBB)=2, nav(0xBF)=4, video-PES(0xE0)=68, private-1/ac3(0xBD)=7,
  mpeg-audio(0xC0)=0 — exactly the pinned lane `ours: [4, 68, 7]`
  (crates/demux/tests/ps_fixtures.rs:109) and record doc
  `docs/formats/container-mpegps.md:51` ("4nav+68v+7ac3").
- Nav packet lengths 980/1018 visible in my own `od` dump (offset 38:
  `00 00 01 bf 03 d4`; offset 2058 region: `00 00 01 bf 03 fa`) — the
  record's "980/1018 alternating, measured" pin verified by eye.
- MPEG-2 video ES structure present in the dump: sequence header
  (0x000001B3), sequence extension (0x000001B5), GOP header (0x000001B8)
  — consistent with the 5.H1 sequence-layer differential (720x480,
  Main@8, yuv420p, 24000/1001, PASS per worklog :9622).
- All four consuming suites passed in the baseline:
  - `tests/ps_fixtures.rs` 4/4 ok (:2179-2183) — incl.
    `fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit`
    (md5 parity with `ffmpeg -c copy`: video 135,397 B, ac3 12,288 B).
  - `ps::tests::fuzz_lite_{mutations_of_the_committed_fixtures_never_panic,
    clean_fixtures_drain_without_terminating_errors}` ok (:1839-1840) —
    this file is one of the two mutation targets (120 LCG rounds × 8
    header-biased flips + truncations, ps/tests.rs:499-518) and one of the
    four clean-drain fixtures (terminal None asserted, :522-537).
  - `containers::ps::tests::vob_lanes_demux_and_refuse_undecodable_codecs_typed`
    ok (:2833) — the real CLI open path, including the honest typed refusal
    of the AC-3 track (decode pending, "parse/route only" — no false
    decode claim anywhere).
  - `codecs/tests/mpeg12_corpus.rs` 3/3 ok (:1142-1148) — the mpeg2video
    lane through this VOB.
- PES arithmetic sanity: 6×2028 + 209 = 12,377 PES bytes minus 7 optional
  headers (~10 B each) minus 7×4 B substream+filler strip ≈ the pinned
  12,288 B AC-3 payload — consistent.
- sha256 (recorded here for provenance): `05556d20c0a9a89589ddcf96
  bf0146b0e7d0aea342647312e41029a602133694`. Git content- addressing plus
  the pinned track digests make an independent whole-file pin unnecessary
  (see §7).

## 3. Findings
None. What was checked and how (per the zero-findings rule):
- Existence/size sanity: present, 157,696 B = 77 × 2048 (DVD
  sector-aligned), no truncation, no zero tail (od dump).
- Content sanity: real MPEG-PS start-code structure, census verified
  independently (above); no embedded executables/compression bombs — it is
  a plain media stream.
- References (Grep for `ps-vob-ac3`, 12 files): 4 test suites + 3 record
  docs + worklog — no orphans; every reference resolves to this exact path.
- Pinned expectations vs my measurements: counts 4/68/7 exact; nav lengths
  980/1018 exact; PES length pattern consistent with the pinned byte
  totals; ffprobe divergence numbers (0+25+32) are ffmpeg-side facts I
  could not re-measure (no ffprobe run allowed) — accepted on the
  baseline's executed differential (which DID run ffprobe live and passed).
- Provenance: ffmpeg 7.1.5 dvd-muxer output (the "substream+3 filler
  bytes" pin is a dvd-muxer fingerprint, worklog :9165); committed
  read-only; the worklog's D-10 note records that a wrong first
  transcription of pins was corrected by live re-measurement, never
  weakened (:9173).
- License/origin: ffmpeg-generated test vectors from lavfi sources (per
  the fixture-family docs); no third-party copyrighted media payload.

## 4. Per-function table
Not applicable (binary asset). Consumers audited at the call-site level:
| Consumer | Location | Verdict | Notes |
|---|---|---|---|
| PS differential lane | crates/demux/tests/ps_fixtures.rs:100-120 | REAL | Tracks (dvd_nav_packet/mpeg2video/ac3), counts, digests, byte totals, first/last PTS all pinned; ffprobe divergence documented, not matched. |
| PS unit + fuzz-lite | crates/demux/src/ps/tests.rs:499,525 | REAL | Mutation target + clean-drain assert (terminal None). |
| CLI open-path lane | apps/cli/src/containers/ps.rs (vob_lanes test) | REAL | Real open path; undecodable AC-3 refused typed. |
| Probe roundtrip | crates/media-probe/tests/registry_roundtrip.rs (crate_fixture) | REAL | The VOB probes `mpegps` through the builtin registry. |
| mpeg2 sequence differential | crates/codecs/tests/mpeg12_corpus.rs | REAL | 720x480 Main@8 lane PASS (baseline 3/3). |

## 5. Test honesty
| Test | Would it fail if the asset were broken? | Reason |
|---|---|---|
| fixture_stream_tables_and_packet_counts_match_the_pins | YES | My own census matches the pins digit-for-digit; any packet-level corruption shifts a count. |
| fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit | YES | md5 of extracted tracks pinned — any payload byte change fails. |
| fuzz_lite_mutations… / clean_fixtures… | YES (for panics/terminating errors) | Absence-of-panic + clean-drain contracts on the exact bytes. |
| vob_lanes_demux_and_refuse_undecodable_codecs_typed | YES | Typed-refusal behavior pinned on this file's AC-3 track. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "ps-vob-ac3.vob | nav+mpeg2video+ac3, 0+25+32 | 4nav+68v+7ac3 | video 135 397 B + ac3 12 288 B md5 bit-exact" | docs/formats/container-mpegps.md:51 | REAL | Our-side numbers verified by my independent census; ffmpeg-side by the baseline-executed differential (ps_fixtures 4/4 ok). |
| "Nav packets: ffprobe demuxes 0 … we expose 4 zero/0x01-lead Data packets (980/1018 B alternating, measured)" | docs/formats/container-mpegps.md:64-66 | REAL | 4 × 0xBF counted; 980/1018 lengths in my od dump. |
| "ffmpeg `dvd` writes substream+3 filler bytes before AC-3/DTS frames (strip 4, bit-exact vs `-f ac3`)" | docs/formats/container-mpegps.md:55-56 | REAL | dvd-muxer fingerprint; asserted by the passing bit-exact test. |
| "committed 5.I2 fixture, ffmpeg-authored, payload bit-exact" | docs/formats/audio-ac3.md (fixture table) | REAL | See §2. |

## 7. Dead / duplicated / unreachable code
None (asset). One deliberate difference from the asf fixture family noted:
no whole-file sha256 pin table exists for the ps fixtures (asf's
`expected_sha` pins container bytes because asf has a regeneration
fallback; ps fixtures are read-only with no regeneration path, and the
per-track md5s + census pins cover every byte the parser touches). Not a
defect — recorded so the wave synthesizer sees the asymmetry was examined,
not missed.

## 8. Dependencies & flags
None (data file). Sits beside ps-mp2-video-audio.mpg, ps-vob-dts.vob,
ps-vob-lpcm.vob in crates/demux/fixtures/ps/ (the committed reference set
the mpegps record names).

## 9. Open questions for the next agent
- The other three ps fixtures (dts/lpcm/mpg) are other agents' rows — the
  same census method (start-code count + od) applies; the lpcm fixture has
  the subtlest pin (ours 96,000 vs ffmpeg 96,144 = +3 B × 48 AU headers —
  verify the delta arithmetic holds on the bytes).
- container-mpegps.md:94-95 lists "MPEG-1 system-header breadth beyond
  the fixture family [VERIFY]" as an open gap — the fixture family has no
  MPEG-1-shaped system header; whoever owns ps/mod.rs should confirm that
  gap is still open.
