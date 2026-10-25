# Container record — MPEG-TS (transport stream: PSI + PES)

Status: **implemented** (demux, cards 5.I1a PSI + 5.I1b PES, merged
by 5.W5-CLOSE with the I1a-supersedes unification). Feature-matrix
row `container-mpegts` still reads `not_started` with empty
evidence — this record is that evidence; amendment recorded in-file
for the Lead (docs-only card 5.REC-1). Front-matter keys per
doc 04 §6:

- id = `container-mpegts` (`docs/feature-matrix.toml:167`)
- status = implemented (demux); platform-independent code, evidence
  Linux-only
- phase = 5 (landed)
- fuzz_target = `fuzz_demux_ts` (`tests/ts_fuzz_survival.rs` +
  in-suite +FB; doc 04 §8.5 registry name)
- crates = [] — zero external deps (mpeg2ts-reader NOT used)
- self_built = [`crates/demux/src/ts/**` (mod/psi/pes/tests/
  test_support)]
- last_audit = 2026-10-24

## Scope landed

**Packet + PSI layer (5.I1a):**

- 188-byte bounded-resync walk: sync 0x47 / TEI / PUSI / 13-bit
  PID / AF-control / CC (tolerant policy: duplicates counted, jumps
  reset + one Discontinuity event); partial tail packet = note.
- PSI: PAT / PMT (PID + PCR + ES loop + descriptors) / CAT
  skip-with-note / **SDT** (service names + provider; the
  service-loop header pinned at 5 bytes per EN 300 468 §5.2.3 — a
  prior 6-byte misread the corpus differential caught); DVB text
  annex-A subset (ISO 6937 Western + Latin-1 fallback); native
  CRC-32/MPEG-2 table (section-must-hash-to-zero); sections
  spanning packets + multi-section payloads.
- PSI head-window collection (until PAT + all PMTs + SDT) then
  rewind; multi-program track surface; `PesConsumer` seam.
- Raw PCR (33+9) surfaced un-interpolated; scrambled packets
  delivered + counted.

**PES layer (5.I1b):**

- `PesHeader::parse` §2.4.3.7: start code, Table 2-18 stream-id
  classification (incl. padding/private/PSM/ECM/EMM/DSM-CC classes),
  `PES_packet_length` with **0 = unbounded video**, forbidden
  `PTS_DTS_flags == 01` refused, header-swallowing-packet refused.
- 33-bit PTS/DTS decode across marker-guarded fields (violations →
  typed `demux.ts.pes.bad-pts`); PID→PES assembler (bounded
  completes at `6+len`, unbounded at next PUSI or honest EOS flush;
  8 MiB accumulation ceiling = typed ResourceLimit).
- Timing events, never smoothed: DiscontinuityIndicator,
  DtsRegressed, DtsJump (>10 s), PtsRegressedNoDts (per-PID
  seen_dts gate), PcrRegressed. Time base 1/90 000.
- Seek: `SeekSupport::None` typed (no container index).

## Corpus evidence (5.I1a + 5.I1b, corpus-v4, ffmpeg 7.1.5 — 5/5 each)

PSI: program count/number, PMT+PCR PIDs, per-program stream
types→PIDs = ffprobe codec identity, SDT names/providers, packet
totals = bytes/188; every lane psi_crc_failures=0, sync_errors=0,
TEI=0:

| Lane | Streams | Service | Packets |
|---|---|---|---|
| 0000-param-midchange.ts | 0x1B→0x100 h264 | Service01/FFmpeg | 408 |
| 0018-ts-video.ts | 0x1B→0x100 h264 | Service01/FFmpeg | 331 |
| 0091-bframe-storm.ts | 0x24→0x100 hevc + 0x0F→0x101 aac | Service01/FFmpeg | 723 |
| 0113-dynres-video.ts | 0x1B→0x100 h264 | Service01/FFmpeg | 621 |
| 0134-ts-video.ts | 0x1B→0x100 h264 | Service01/FFmpeg | 96 |

PES: per-stream PES counts, PTS first/last, wire-DTS endpoints,
payload byte totals (= ffprobe Σsize, exact), monotonicity —
e.g. 0018: 234 PES / 6 566 B / DTS ✓; 0091 video: 53 PES /
103 550 B, 40 of 53 wire DTS, reorder preserved verbatim; 0000/
0113/0134: DTS absent on the wire (ffprobe synthesizes — we report
`None`, asserted). Full 53-value PTS series pinned on 0091.

## Wire facts pinned empirically (5.I1b — 2 real bugs the differential found)

1. **PES uses NO pointer_field** (§2.4.4.1's pointer is PSI-only).
2. ffprobe's `dts` column is **synthesized** for PTS-only PES.
3. 0091's audio PID carries **6 multi-AU PES packets**, not
   ffprobe's 43 (parser-split AUs; payload byte sums equal).

## Rides (later cards, cited)

5.H1: mpeg2-in-TS lane through our TsDemuxer + PES seam →
sequence-layer differential PASS. 5.G1: 0091 hevc extracted via
`ffmpeg -c copy -f hevc` (recorded seam — in-tree hvcC/in-band
wiring is the 5.G2 follow-up).

## Error paths / fuzz

Typed `demux.ts.*` vocabulary (not-ts, no-PAT, truncated section
drop-and-continue, CRC mismatch drop-not-fatal, resync-failed,
pes truncated/bad-pts…). Fuzz-lite: seeded-LCG 512 rounds × 1–8
mutations (header/PSI-biased) + 256-round real-corpus-head lane —
zero panics, deterministic replays.

## Honest gaps (D-10, from 5.I1a/5.I1b)

- **Multi-program TS**: corpus has single-program files only; the
  multi-program surface is unit/synthetic-covered, not
  corpus-verified [VERIFY on acquisition].
- Scrambled/TEI packets: delivered + counted, no descrambling (CA
  module scope); PCR pacing/interpolation = engine's job (raw
  surfaced).
- Full ISO 6937 composition table (Western subset); SDT
  service_type→genre mapping (raw byte surfaced).
- Teletext/DVB-sub extraction split to `subtitle-teletext`
  (blocked, per matrix note).
- In-tree hvcC/avcC wiring from TS extra_data → codec parsers
  (5.G2/5.F2 follow-up; the ffmpeg-extraction seam is recorded).

## Gates (card-time, exact)

5.I1a: fmt clean; clippy EXIT 0; demux 442/0 (33 new TS tests).
5.I1b: fmt clean; clippy EXIT 0; 318 lib (17 new pes unit) + all
integration suites. Merged wave-5 tree: workspace 2752/0
(5.W5-CLOSE); wave-7 close: 3040/0.

## Next steps

- Lead: matrix-row amendment. 5.G2: plug TS hvcC path into the
  HEVC parser. Multi-program corpus lane acquisition [VERIFY].
