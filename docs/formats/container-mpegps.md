# Container record — MPEG-PS / VOB (program stream)

Status: **implemented** (demux, card 5.I2). Feature-matrix row
`container-mpegps` still reads `not_started` with empty evidence —
this record is that evidence; amendment recorded in-file for the
Lead (docs-only card 5.REC-1). Front-matter keys per doc 04 §6:

- id = `container-mpegps` (`docs/feature-matrix.toml:180`)
- status = implemented (demux); platform-independent code, evidence
  Linux-only
- phase = 5 (landed; the doc 04 §2.1 `mpeg-ps 0.6.0` crate path was
  superseded by the native self-build)
- fuzz_target = `fuzz_demux_ps` (in-crate `ps/fuzz.rs` + 240-round
  +FB fuzz-lite; doc 04 §8.5 registry name)
- crates = [] — zero external deps
- self_built = [`crates/demux/src/ps/**` (mod/error/fuzz/
  test_support)]
- last_audit = 2026-10-24

## Scope landed (5.I2)

- Probe: pack start code `00 00 01 BA`; `EXTENSIONS = ["mpg","mpeg",
  "ps","vob","m2p"]` (ps/mod.rs:121, verified this audit).
- Pack headers both flavors: MPEG-1 12 B / MPEG-2 14 B + stuffing;
  SCR base 33-bit width-checked, mux rate 22-bit; **marker bits
  parsed, never enforced** (the `b[11]&1` lesson — VOB `01 89 c3` +
  MPEG-1 `..01` rates accepted).
- System header 0xBC / PSM skip; PES 0xC0..=0xDF (audio) /
  0xE0..=0xEF (video) in MPEG-2 ('10' flags, PTS / PTS+DTS,
  header-len overrun check) and MPEG-1 (0xFF stuffing,
  0010/0011/0001 stamps, 0x0F no-stamp) flavors.
- **Private-1 0xBD routing**: 0x80+ → AC-3, 0x88+ → DTS,
  0xA0+ → LPCM (7-byte DVD prefix strip), 0x20+ → SPU (Subtitle
  kind) — honest descriptors, decode pending, never silent.
  Unclassified substreams + unregistered stream-ids skipped with
  Recovered events.
- 0xBF private-2 → dvd_nav_packet Data track; 0xBE padding
  damage-noted; 0xB9 program-end; PES len 0 = damage-noted bounded
  skip; 64 KiB window resync; seek SlowLinear (SCR-mapped) +
  compute-then-commit reanchor.

## Corpus evidence (5.I2, ffmpeg 7.1.5, committed fixtures — 4/4)

**No .ps/.vob lane exists in corpus-v4** (verified again by
5.K4-I) — the committed ffmpeg-authored fixtures are the honest
reference set:

| Fixture | ffprobe | Lumen (PES truth) | Payload verdict |
|---|---|---|---|
| ps-mp2-video-audio.mpg | mpeg2video+mp2, 25v+39a | 32v+5a | video 63 898 B + mp2 8 150 B md5 == ffmpeg `-c copy` **bit-exact** |
| ps-vob-ac3.vob | nav+mpeg2video+ac3, 0+25+32 | 4nav+68v+7ac3 | video 135 397 B + ac3 12 288 B md5 **bit-exact** |
| ps-vob-dts.vob | nav+mpeg2video+dts, 0+25+94 | 4nav+68v+24dts | dts 48 128 B md5 **bit-exact** |
| ps-vob-lpcm.vob | nav+mpeg2video+pcm_dvd, 0+25+48 | 4nav+68v+48lpcm | ours 96 000 B pure samples; ffmpeg 96 144 = +3 B × 48 pcm_dvd AU headers — **exact delta pinned** |

Exact-delta pins (asserted): ffmpeg `dvd` writes substream+3 filler
bytes before AC-3/DTS frames (strip 4, bit-exact vs `-f ac3`);
unstamped MPEG-1 PES single 0x0F byte accepted (spec's 0xFF
stuffing must still terminate in 0x0F — both accepted).

## Honest divergences (measured, pinned — 5.I2)

- **Packet counts**: container truth = PES; ffmpeg re-packetizes to
  AUs (32v+5a vs 25v+39a etc.) — pinned, not matched.
- **Nav packets**: ffprobe demuxes 0 (its dvd demuxer consumes
  them); we expose 4 zero/0x01-lead Data packets (980/1018 B
  alternating, measured).
- **Track span end** = last observed PES stamp; ffprobe interpolates
  unstamped AUs (mpg audio 118 149 vs 136 957; ac3 126 034 vs
  137 554) — recorded, we do not fabricate.

## Rides (later cards, cited)

5.H1: mpeg2 PS lanes (ps-mp2, ps-vob-ac3) through our PsDemuxer →
sequence-layer differential PASS. 5.K4-I: private-1 carriage
verified bit-exact (0x80..=0x87→ac3 / 0x88..=0x8F→dts,
file:line-cited).

## Error paths / fuzz

Typed `demux.ps.*` (not-ps, truncated, bad-pack, bad-pes-header,
resync-failed, payload-too-large, not-started,
seek-out-of-range). +FB fuzz-lite: 240 LCG mutations+truncations
biased into header regions over the 2 smallest fixtures + all 4
clean — Ok-or-typed only, zero panics.

## Honest gaps (D-10, from 5.I2)

- **No real-corpus differential** (no .ps/.vob in corpus-v4; the
  4 committed fixtures are the reference set — stated, not
  smoothed).
- Decode of exposed substreams pending their codec cards (AC-3/DTS
  blocked per 5.K4-I; pcm_dvd decoder unscheduled
  [PROPOSAL in module docs]; SPU → subtitle path).
- DVD-VOB IFO navigation out of scope (ADR); MPEG-1 system-header
  breadth beyond the fixture family [VERIFY].
- The inherited draft's byte-0 pack rejection was found by RUNNING
  the fixtures (never run before commit) — root-caused, not
  papered over (D-10 note, 5.I2).

## Gates (5.I2, exact)

`cargo fmt -p demux -- --check` CLEAN; clippy CLEAN;
`cargo test -p demux --all-features` 442/0 (327 lib incl. 22 ps
unit; ps_fixtures 4/4; doctests 6/6).

## Next steps

- Lead: matrix-row amendment. Codec cards consume the honest
  descriptors; corpus PS/VOB lane acquisition if share justifies.
