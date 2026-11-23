# WAVE 04 SUMMARY — demuxers I (AVI/WAV/MP3/FLAC/OGG/ADTS/ES/AIFF/ASF/IVF) + media-probe

| Field | Value |
|---|---|
| Agents | A01–A05, A07–A10 complete; A06 crashed (context deadline) and completed via re-spawn per 7.4 |
| Files audited | 120 files (160 rows), 160/160 covered |
| Findings | S0:0 S1:4 S2:9 S3:47 S4:180 (240 total) |
| Citation gate | 243 checked, 230 verbatim, 3 partial-quote defects, 1 misattribution (orchestrator-repaired) |
| Tree purity | clean |

## Worst findings
1. [S1, EXECUTED] TsSignature::probe (media-probe/src/signature/phase5.rs:160-164)
   indexes prefix[188] under guard len>=188 -> panic "index out of bounds:
   len 188, index 188" on a 188-byte 0x47-led window; reachable from the
   shipped CLI open path (demux_host.rs:443-448, no catch_unwind). Twin bug
   at demux/src/ts/mod.rs:315-322 (W04-A01, standalone rustc repro).
2. [S1, executed] MKV lacing inversion (W04-A06 prior run) + idx1 over-cap
   open failure (executed).
3. [S1] asf/header.rs:558 unchecked `off as u64 + size` -> panic on crafted
   child size (library API; ASF unwired so unreachable from CLI today)
   (W04-A10).
4. [S1] wasm32-only: AVI probe walk_hdrl/walk_strl saturating advance +
   `pos + 8` overflow -> slice panic; doc :122-123 falsely claims D-9
   handles it; same pattern in mp4.rs (W04-A07, analytic + python u32 proof).
5. [S2, measured] OGG CPU amplification: scan_landing per-byte rescan
   15.86s/8MiB garbage (shipped --start-time path); recover_sync 10.2s/4MiB
   dense OggS; caps bound bytes not CPU (W04-A04).
6. [S2] ASF demuxer fully implemented but UNREACHABLE (no probe signature,
   no registry arm) while compat-matrix claims container-asf-wmv
   implemented on 3 platforms with zero evidence (W04-A04/A08/A10; EC_NONE
   doc claim also FALSE — executed disproof via fixtures, W04-A06).
7. [S2] avi/header.rs unbounded DamageLog ~4-6x RAM amplification (W04-A03).
8. [S2 systemic] Corpus differentials are green NO-OPs: corpus-v4 absent in
   every environment -> tests SKIP and report pass (ogg/wav/ts/mkv/es/adts
   lanes; mkv_corpus has no recorded evidence at all) (W04-A02/A03/A05/A09/A10).
9. [S2] asf/mod.rs 4 GiB header-driven allocation (S1-if-wired) (W04-A08).
10. [KEY] MP4 path uses avcC (mp4.rs:237); es/h264.rs Annex-B demuxer is
    UNREACHABLE from MP4; CLI constructs AnnexbVideoDemuxer only for raw
    .264/.265 (W04-A08). FLAC: no flac demux module in demux/ (FLAC lives
    in codecs via symphonia + ogg/flac container) — Wave 7 to confirm.

## Domain verdict
Demuxers I are mostly REAL with adversarial test culture (byte surgery,
digest pins, executed fuzz loops) — the strongest code domain so far. The
rot is at the edges: unwired ASF (papered as implemented), TS/ASF/wasm
panic paths, corpus lanes that pass vacuously, and media-probe config
knobs parsed but never consumed.
