# AUDIT: crates/demux/fixtures/ps/ps-vob-lpcm.vob#part4 (chunk: the final ~36,673 bytes, bytes 204,991-241,664 of 241,664; "lines" 1699-2264 of 2264)
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-21 |
| Kind | binary fixture chunk (MPEG-PS / DVD-VOB program stream, ffmpeg-muxed per docs) |
| File verdict (chunk) | REAL |
| Reachable from shipped entry point? | The fixture is consumed by tests that exercise the shipped PS demuxer lane: `apps/cli/src/containers/ps.rs:909` (VOB lane expectation), `crates/demux/tests/ps_fixtures.rs:148` (lane table), `crates/demux/src/ps/tests.rs:527`. PS itself is shipped-reachable (PsSignature registered in with_builtins; containers/ps.rs wired). Baseline: 28 `ps::*` tests green; `lpcm_packets_strip_the_full_dvd_prefix ... ok` (:2181); `private1_dts_lpcm_and_spu_route_with_their_strips ... ok` (:1852). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 MPEG-PS walk of the whole file and of my byte range (pack headers 0x000001BA w/ stuffing, system headers, PES 0xE0/0xBD/0xBE/0xBF, PTS bit-extraction per ISO 13818-1); sha256 computed (`4366d145275b69f5806fd226456725fe2554e9773f2f41dee367eeba4dd387fc`); `file(1)` → "MPEG sequence, v2, program multiplex"; rg for `ps-vob-lpcm` repo-wide (4 test/doc consumers); rg `4366d145|sha.*ps-vob-lpcm` → no pin anywhere; `rg -n "vob|mpegps|ps/" tools/xtask/src/cmd/fixtures.rs` → zero matches (the fixtures writer does not manage demux-crate fixtures); `head -30 gate-fixtures___check.log.txt` (gate covers only tests/fixtures, 55 files); `sed -n 145,157p ps_fixtures.rs`, `sed -n 900,915p apps/cli/src/containers/ps.rs` (committed expectations); `sed -n 1,30p + :53 docs/formats/container-mpegps.md`; worklog differential line verified. |

## 1. One-paragraph verdict
The chunk is the audio tail of a real ffmpeg-muxed DVD-VOB program stream:
17 LPCM packets (private-stream-1 substream 0xA0, PTS 107,237 → 138,583,
monotonic), 17 MPEG-2 pack headers, one 582-byte all-0xFF padding packet
whose end lands exactly at EOF, and no video/nav content (all of that lives
in parts 1-3). My independent byte-level parse of the WHOLE file reproduces
every committed expectation exactly — 4 nav + 68 video + 48 LPCM packets,
96,000 pure sample bytes (48 × 7-byte LPCM header arithmetic), last audio
PTS 138,583 — matching `ps_fixtures.rs` lane table [4, 68, 48], the CLI
expectation row (`"pcm_dvd", 48, 138_583`), and the container record's
ffmpeg differential ("ours 96 000 B; ffmpeg 96 144 = +3 B × 48 AU headers").
Nothing in the chunk is malformed, truncated, or inconsistent. The single
finding is integrity hygiene: this conformance-truth binary has no committed
hash pin and is not covered by the fixtures drift gate, which only manages
tests/fixtures.

## 2. What actually works (verified)
— Independent MPEG-PS parse (python3, ISO-13818-1 start-code walk), whole
file: 118 packs, 1 system header, 4 × 0xBF nav, 68 × 0xE0 video, 48 × 0xBD
sub=0xA0 LPCM, 3 × 0xBE padding — EXACTLY the documented "4nav+68v+48lpcm"
(docs/formats/container-mpegps.md:53) and the lane table `ours: [4, 68, 48]`
(ps_fixtures.rs:157).
— My chunk (bytes 204,991-241,664): 17 packs, 17 LPCM (16 × plen 2028 + 1 ×
plen 1440 final), 1 padding 0xBE (582 × 0xFF) ending precisely at EOF;
every packet boundary in the range is start-code aligned (the walker never
needed a garbage-skip).
— PTS decode verified against the committed pin: last packet (off 239,630,
plen 1440) carries PTS 138,583 — identical to the CLI expectation
`("ps-vob-lpcm.vob", "pcm_dvd", 48, 138_583)` (ps.rs:909). PTS ladder
monotonic across the file (all deltas positive; min 2,059, max 2,091).
— Sample-byte accounting: 48 packets × (payload − 7-byte LPCM header) =
96,000 pure sample bytes — exactly the container record's differential
figure ("ours 96 000 B pure samples"), and consistent with the documented
ffmpeg delta (+3 B × 48 AU headers = 96,144).
— Final AU is whole: 1,424 sample bytes = 712 exact mono 16-bit frames.
— LPCM headers uniform in my range: `a0 07 00 04 0c 00` + sample data
beginning `80 f7` (16-bit LE, DVD-LPCM convention).
— Consumer tests green in baseline (28 ps tests; LPCM-specific: :1852,
:2181) — the expectations I byte-verified are enforced, not aspirational.

## 3. Findings

### F-001 [S4]
Where: crates/demux/fixtures/ps/ps-vob-lpcm.vob (whole file; discovered from this chunk's provenance work) — no line range applicable
Code:
```
(file bytes; sha256 = 4366d145275b69f5806fd226456725fe2554e9773f2f41dee367eeba4dd387fc)
```
What is wrong: This binary is conformance truth — three test sites pin
byte-derived extraction facts to it — yet it has NO committed integrity
pin (repo-wide rg for its sha256: zero matches; the corpus docs pin sha256
for other fixtures, e.g. compat-matrix-20261110.md:260-261 pins AVI corpus
files, but ps-vob-lpcm.vob has no entry), and the drift gate does not cover
it: `cargo xtask fixtures --check` verified "55 file(s) under
tests/fixtures" only (gate-fixtures___check.log.txt), and the fixtures
writer has zero references to vob/mpegps (rg over tools/xtask/src/cmd/fixtures.rs).
Why it matters: The fixture's provenance (ffmpeg 7.1.5 differential,
bit-exact md5 claims in worklog archive-2026-10.md:9166) rests entirely on
prose. A regenerated or accidentally modified file would be caught only if
the change moved the pinned counts/PTS — a subtle re-mux that preserves
packet counts but changes bytes would silently invalidate every
bit-exactness claim recorded against it.
What correct looks like: a fixtures manifest row (path, bytes, sha256) for
crates/demux/fixtures/** — the same pattern the corpus docs already use for
the AVI corpus — checked by the existing gate.
Confidence: HIGH — Verification: sha256 computed by me; repo-wide hash
grep empty; gate log read; writer grep empty.
Blast radius: provenance hygiene only; today's bytes are consistent with
every pinned expectation (§2).

## 4. Per-item table (the chunk's parse inventory)
| Item | Count / value | Verdict | Notes |
|---|---|---|---|
| MPEG-2 pack headers (00 00 01 BA) | 17 | REAL | stuffing parsed; all consumed cleanly by the walker |
| System headers (00 00 01 BB) | 0 in chunk (1 file-wide) | — | lives in part1 |
| Nav packs (0xBF) | 0 in chunk (4 file-wide) | — | parts 1-3 |
| Video PES (0xE0) | 0 in chunk (68 file-wide) | — | parts 1-3; matches lane table |
| LPCM PES (0xBD sub 0xA0) | 17 (48 file-wide) | REAL | 16 × plen 2028 + final plen 1440; PTS-bearing; monotonic 107,237→138,583 |
| Padding (0xBE) | 1 | REAL | 582 × 0xFF, ends exactly at EOF |
| Program end (0xB9) | absent | — | ffmpeg norm: EOF-terminated; demuxer treats EOF as end (no defect) |
| Tail padding | 0xFF only | REAL | VOB padding-cell norm |
| LPCM headers | uniform `a0 07 00 04 0c 00` | REAL | sub=0xA0; 7-byte header (sub+frame+AU-ptr+2 flag bytes+…), 16-bit LE samples |
| Chunk boundary | byte 204,991 cuts mid-PES (packet starting 204,814) | — | inherent to line-based chunking of a binary — see §7 note |

## 5. Test honesty
| Test (baseline) | Would it fail if the fixture were broken? | Reason |
|---|---|---|
| `ps_fixtures.rs` lane (file row :147-157) | YES | asserts 3 tracks and per-track packet counts [4, 68, 48] — my parse independently reproduces all three from bytes |
| `vob_lanes_demux_and_refuse_undecodable_codecs_typed` (ps.rs:903+) | YES | asserts audio descriptor `pcm_dvd`, 48 packets, last PTS 138,583 — byte-verified by me (last packet PTS decode = 138,583 exactly) |
| `lpcm_packets_strip_the_full_dvd_prefix` (:2181 green) | YES | strips the 7-byte DVD LPCM header — the uniform headers in my chunk are exactly the shape it strips |
| `private1_dts_lpcm_and_spu_route_with_their_strips` (:1852 green) | YES | routes sub=0xA0 LPCM vs subpicture strips — my chunk's 17 sub=0xA0 packets are that path's data |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ps-vob-lpcm.vob | nav+mpeg2video+pcm_dvd, 0+25+48 | 4nav+68v+48lpcm" | container-mpegps.md:53 | REAL | my whole-file parse: 4 BF + 68 E0 + 48 BD/A0 — exact |
| "ours 96 000 B pure samples; ffmpeg 96 144 = +3 B × 48 pcm_dvd AU headers — exact delta pinned" | container-mpegps.md:53 | REAL | 48 × (payload − 7) = 96,000 by my arithmetic |
| ffprobe counts "0+25+48" (nav consumed by ffprobe's dvd demuxer; 25 video AUs vs our 68 PES) | worklog archive-2026-10.md:9166 | REAL-UNPROVEN | ffprobe not run by me (no cargo/ffmpeg runs allowed); the divergence rationale is coherent with PES-vs-AU framing and our 68 PES count is byte-verified |
| "Status: implemented (demux, card 5.I2)" | container-mpegps.md:3-4 | REAL | 28 ps tests green in baseline; lane is shipped-reachable (PsSignature + containers/ps.rs) |
| Feature-matrix row `container-mpegps` "still reads not_started with empty evidence" | container-mpegps.md:4-6 | matrix drift (other agents' scope) | the record itself flags it — matrix row audit belongs to W18/W19 |

## 7. Dead / duplicated / unreachable / process notes
- No dead bytes in the chunk: every byte belongs to a pack, a PES packet,
  or the terminal padding packet.
- Process note (not a repo finding): this binary was assigned as a
  LINE-numbered chunk (567 "lines" of 2,264), but newline-splitting a
  binary is meaningless — the part boundaries land mid-PES (mine cuts
  inside the packet starting at byte 204,814). Part4 covers the final
  36,673 bytes (15% of the file, audio tail only); the whole-file parse
  above is what makes the chunk auditable at all. Future waves: assign
  binary fixtures by byte range or as single asset-group entries.

## 8. Dependencies & flags
- None (data file). Consumers: demux ps tests (A09's ps files), CLI
  containers/ps.rs VOB lane, docs/formats/container-mpegps.md record.

## 9. Open questions for the next agent
- Parts 1-3 owners (A10, A04, A09): the nav packs (4 × 0xBF, PCI/DSI
  contents), system header, and all 68 video E0 packets live in your
  ranges — my whole-file counts give you the ground truth to verify
  against (4/1/68 respectively).
- Whoever owns the fixtures gate: F-001's manifest gap applies to the
  whole `crates/demux/fixtures/` tree (adts/, ps/, asf/fixtures/), not
  just this file.
